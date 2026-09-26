//! Game versions and where they come from.
//!
//! A version is a folder holding `randcrw-manifest.json` (contract §Version). Versions come from
//! *sources*:
//!
//! - `official`: GitHub releases, downloaded into `<root>/versions/official/<tag>/`.
//! - `development`: local build folders the user adds; they stay where they are.
//!
//! Mods will be one more source (`versions/mods/<name>/`, installed like official builds from
//! another feed). Adding it means a new `SourceId` variant, its arm in `SourceId::location`
//! and a listing function; nothing else keys on the source list.

use crate::contract::{Manifest, RuntimeVersion, MANIFEST_FILE};
use crate::paths::Layout;
use crate::settings::Settings;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceId {
    Official,
    Development,
}

/// Where a source keeps its versions.
pub enum SourceLocation {
    /// Installed under `<root>/versions/<dir>/<id>/`.
    Managed(&'static str),
    /// The id is an absolute path the user registered.
    External,
}

impl SourceId {
    pub const ALL: [SourceId; 2] = [SourceId::Official, SourceId::Development];

    pub fn location(self) -> SourceLocation {
        match self {
            SourceId::Official => SourceLocation::Managed("official"),
            SourceId::Development => SourceLocation::External,
        }
    }
}

/// Identifies one version: `(source, id)`. For managed sources `id` is the folder name, for
/// development builds it is the absolute folder path.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VersionRef {
    pub source: SourceId,
    pub id: String,
}

/// What the UI shows for a version.
#[derive(Debug, Clone, Serialize)]
pub struct VersionInfo {
    #[serde(rename = "ref")]
    pub vref: VersionRef,
    pub path: PathBuf,
    pub manifest: Option<Manifest>,
    pub runtime: Option<RuntimeVersion>,
    /// `None` when valid; otherwise why the version cannot be used.
    pub problem: Option<String>,
    pub active: bool,
}

/// A usable version with absolute binary paths.
#[derive(Debug, Clone)]
pub struct ResolvedVersion {
    pub vref: VersionRef,
    pub dir: PathBuf,
    pub manifest: Manifest,
    pub runtime: PathBuf,
    pub extractor: PathBuf,
}

/// Folder of a version, or an error if the ref cannot name one.
pub fn version_dir(layout: &Layout, settings: &Settings, r: &VersionRef) -> Result<PathBuf, String> {
    match r.source.location() {
        SourceLocation::Managed(dir) => {
            let id_ok = !r.id.is_empty()
                && !r.id.contains(['/', '\\'])
                && r.id != "."
                && r.id != "..";
            if !id_ok {
                return Err(format!("invalid version id \"{}\"", r.id));
            }
            Ok(layout.source_dir(dir).join(&r.id))
        }
        SourceLocation::External => {
            let p = PathBuf::from(&r.id);
            if settings.dev_versions.iter().any(|d| d.path == p) {
                Ok(p)
            } else {
                Err("this development build is no longer in the list".into())
            }
        }
    }
}

pub fn read_manifest(dir: &Path) -> Result<Manifest, String> {
    let file = dir.join(MANIFEST_FILE);
    let text = fs::read_to_string(&file)
        .map_err(|_| format!("No {MANIFEST_FILE} in {}", dir.display()))?;
    Manifest::parse(&text).map_err(|e| e.to_string())
}

/// Manifest plus existence of both binaries. Cheap; used for listings.
pub fn resolve_dir(vref: VersionRef, dir: PathBuf) -> Result<ResolvedVersion, String> {
    let manifest = read_manifest(&dir)?;
    let runtime = dir.join(&manifest.runtime);
    let extractor = dir.join(&manifest.extractor);
    for (what, p) in [("Game runtime", &runtime), ("Extractor", &extractor)] {
        if !p.is_file() {
            return Err(format!("{what} not found at {}", p.display()));
        }
    }
    Ok(ResolvedVersion { vref, dir, manifest, runtime, extractor })
}

pub fn resolve(layout: &Layout, settings: &Settings, r: &VersionRef) -> Result<ResolvedVersion, String> {
    let dir = version_dir(layout, settings, r)?;
    resolve_dir(r.clone(), dir)
}

pub fn resolve_active(layout: &Layout, settings: &Settings) -> Result<ResolvedVersion, String> {
    let r = settings
        .active_version
        .as_ref()
        .ok_or("No game version is active. Pick one in Settings → Version Management.")?;
    resolve(layout, settings, r)
}

/// Runs `<runtime> --version-json` (10 s timeout) and checks it agrees with the manifest.
pub fn query_runtime(v: &ResolvedVersion) -> Result<RuntimeVersion, String> {
    let mut child = Command::new(&v.runtime)
        .arg("--version-json")
        .current_dir(&v.dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Could not start {}: {e}", v.runtime.display()))?;
    let mut stdout = child.stdout.take().expect("piped stdout");
    let reader = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stdout.read_to_string(&mut s);
        s
    });
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(25)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("The runtime did not answer --version-json within 10 s".into());
            }
        }
    }
    let out = reader.join().unwrap_or_default();
    let rv = RuntimeVersion::parse(&out)?;
    check_runtime_matches(&v.manifest, &rv)?;
    Ok(rv)
}

pub fn check_runtime_matches(m: &Manifest, rv: &RuntimeVersion) -> Result<(), String> {
    if rv.name != "randcrw" {
        return Err(format!("The runtime reports name \"{}\", expected randcrw", rv.name));
    }
    if rv.game != m.game {
        return Err(format!("The runtime is for {}, the manifest says {}", rv.game, m.game));
    }
    if rv.data_format != m.data_format {
        return Err(format!(
            "The runtime expects data format {}, the manifest says {}",
            rv.data_format, m.data_format
        ));
    }
    Ok(())
}

/// Full validation: manifest, binaries, and the runtime's own answer.
pub fn validate(vref: VersionRef, dir: PathBuf, active: bool) -> VersionInfo {
    match resolve_dir(vref.clone(), dir.clone()) {
        Ok(v) => match query_runtime(&v) {
            Ok(rv) => VersionInfo { vref, path: dir, manifest: Some(v.manifest), runtime: Some(rv), problem: None, active },
            Err(e) => VersionInfo { vref, path: dir, manifest: Some(v.manifest), runtime: None, problem: Some(e), active },
        },
        Err(e) => VersionInfo {
            vref,
            path: dir.clone(),
            manifest: read_manifest(&dir).ok(),
            runtime: None,
            problem: Some(e),
            active,
        },
    }
}

/// Cheap listing (no processes spawned) of every source.
pub fn list(layout: &Layout, settings: &Settings) -> Vec<VersionInfo> {
    let mut out = Vec::new();
    for source in SourceId::ALL {
        let refs: Vec<(VersionRef, PathBuf)> = match source.location() {
            SourceLocation::Managed(dir) => {
                let base = layout.source_dir(dir);
                let mut entries: Vec<_> = fs::read_dir(&base)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .filter(|e| e.path().join(MANIFEST_FILE).is_file())
                    .map(|e| {
                        let id = e.file_name().to_string_lossy().into_owned();
                        (VersionRef { source, id }, e.path())
                    })
                    .collect();
                entries.sort_by(|a, b| b.0.id.cmp(&a.0.id));
                entries
            }
            SourceLocation::External => settings
                .dev_versions
                .iter()
                .map(|d| (VersionRef { source, id: d.path.to_string_lossy().into_owned() }, d.path.clone()))
                .collect(),
        };
        for (vref, dir) in refs {
            let active = settings.is_active(&vref);
            let (manifest, problem) = match resolve_dir(vref.clone(), dir.clone()) {
                Ok(v) => (Some(v.manifest), None),
                Err(e) => (read_manifest(&dir).ok(), Some(e)),
            };
            out.push(VersionInfo { vref, path: dir, manifest, runtime: None, problem, active });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::tests::TempDir;

    fn write_version(dir: &Path, runtime_body: &str) {
        fs::create_dir_all(dir.join("bin")).unwrap();
        fs::write(
            dir.join(MANIFEST_FILE),
            r#"{"schema":1,"name":"randcrw","version":"0.1.0","game":"rac1","runtime":"bin/randcrw","extractor":"bin/randcrw-extract","supported_discs":["SCUS_971.99"],"data_format":1}"#,
        )
        .unwrap();
        for f in ["bin/randcrw", "bin/randcrw-extract"] {
            fs::write(dir.join(f), runtime_body).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(dir.join(f), fs::Permissions::from_mode(0o755)).unwrap();
            }
        }
    }

    #[test]
    fn managed_ids_cannot_escape() {
        let l = Layout::new("/r");
        let s = Settings::default();
        for bad in ["", "..", "a/b", "a\\b"] {
            let r = VersionRef { source: SourceId::Official, id: bad.into() };
            assert!(version_dir(&l, &s, &r).is_err(), "{bad}");
        }
        let ok = VersionRef { source: SourceId::Official, id: "v0.1.0".into() };
        assert_eq!(version_dir(&l, &s, &ok).unwrap(), PathBuf::from("/r/versions/official/v0.1.0"));
    }

    #[test]
    fn dev_refs_must_be_registered() {
        let l = Layout::new("/r");
        let mut s = Settings::default();
        let r = VersionRef { source: SourceId::Development, id: "/b".into() };
        assert!(version_dir(&l, &s, &r).is_err());
        s.add_dev_version("/b".into());
        assert_eq!(version_dir(&l, &s, &r).unwrap(), PathBuf::from("/b"));
    }

    #[test]
    fn listing_covers_both_sources() {
        let t = TempDir::new("versions");
        let layout = Layout::new(t.0.join("root"));
        write_version(&layout.source_dir("official").join("v0.1.0"), "");
        fs::create_dir_all(layout.source_dir("official").join("junk")).unwrap();
        let dev = t.0.join("devbuild");
        write_version(&dev, "");
        let mut s = Settings::default();
        let dref = s.add_dev_version(dev.clone());
        s.add_dev_version(t.0.join("missing"));
        s.active_version = Some(dref.clone());

        let all = list(&layout, &s);
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].vref.source, SourceId::Official);
        assert_eq!(all[0].vref.id, "v0.1.0");
        assert!(all[0].problem.is_none());
        assert!(all[1].active && all[1].vref == dref && all[1].problem.is_none());
        assert!(all[2].problem.as_deref().unwrap().contains("No randcrw-manifest.json"));
        assert_eq!(resolve_active(&layout, &s).unwrap().runtime, dev.join("bin/randcrw"));
    }

    #[cfg(unix)]
    #[test]
    fn validate_runs_version_json() {
        let t = TempDir::new("validate");
        let good = t.0.join("good");
        write_version(
            &good,
            "#!/bin/sh\necho 'booting'\necho '{\"name\":\"randcrw\",\"version\":\"0.1.0\",\"game\":\"rac1\",\"data_format\":1}'\n",
        );
        let r = VersionRef { source: SourceId::Development, id: good.to_string_lossy().into() };
        let info = validate(r, good, false);
        assert_eq!(info.problem, None);
        assert_eq!(info.runtime.unwrap().version, "0.1.0");

        let bad = t.0.join("bad");
        write_version(
            &bad,
            "#!/bin/sh\necho '{\"name\":\"randcrw\",\"version\":\"0.1.0\",\"game\":\"rac1\",\"data_format\":2}'\n",
        );
        let r = VersionRef { source: SourceId::Development, id: bad.to_string_lossy().into() };
        let info = validate(r, bad, false);
        assert!(info.problem.unwrap().contains("data format"));
    }
}
