//! Data root location and layout.
//!
//! The data root holds everything the launcher manages:
//!
//! ```text
//! <root>/versions/<source>/<version>/   downloaded game builds (official, later mods)
//! <root>/games/<game>/data/             extracted game data (the runtime's --data-dir)
//! <root>/logs/                          launcher and game logs
//! <root>/settings/launcher.json         launcher settings
//! ```
//!
//! The default root is per OS (`default_data_root_for`). When the user moves it, a small
//! `location.json` pointer is left in the default root so the launcher can find it again.
//! `RANDCRW_DATA_ROOT` overrides both (development and tests).

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const APP_DIR_NAME: &str = "randcrw";
pub const ROOT_ENV: &str = "RANDCRW_DATA_ROOT";
pub const LOCATION_FILE: &str = "location.json";
/// Top-level entries the launcher owns and moves with the root.
pub const MANAGED_ENTRIES: [&str; 4] = ["versions", "games", "logs", "settings"];

/// Per-OS default data root. `env` is injected so this is testable on any host.
pub fn default_data_root_for(os: &str, env: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let non_empty = |k: &str| env(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    let base = match os {
        "macos" => non_empty("HOME").map(|h| h.join("Library").join("Application Support")),
        "windows" => non_empty("LOCALAPPDATA")
            .or_else(|| non_empty("USERPROFILE").map(|p| p.join("AppData").join("Local"))),
        _ => non_empty("XDG_DATA_HOME").or_else(|| non_empty("HOME").map(|h| h.join(".local").join("share"))),
    }?;
    Some(base.join(APP_DIR_NAME))
}

pub fn default_data_root() -> Option<PathBuf> {
    default_data_root_for(std::env::consts::OS, &|k| std::env::var(k).ok())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct Location {
    data_root: PathBuf,
}

/// Resolves the active data root: env override, then the pointer in the default root, then
/// the default root itself.
pub fn resolve_data_root(default_root: &Path, env_override: Option<PathBuf>) -> PathBuf {
    if let Some(p) = env_override.filter(|p| !p.as_os_str().is_empty()) {
        return p;
    }
    read_location(default_root).unwrap_or_else(|| default_root.to_path_buf())
}

fn read_location(default_root: &Path) -> Option<PathBuf> {
    let text = fs::read_to_string(default_root.join(LOCATION_FILE)).ok()?;
    let loc: Location = serde_json::from_str(&text).ok()?;
    Some(loc.data_root).filter(|p| p.is_absolute())
}

fn write_location(default_root: &Path, data_root: &Path) -> io::Result<()> {
    let pointer = default_root.join(LOCATION_FILE);
    if paths_equal(default_root, data_root) {
        match fs::remove_file(&pointer) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
            _ => return Ok(()),
        }
    }
    fs::create_dir_all(default_root)?;
    let text = serde_json::to_string_pretty(&Location { data_root: data_root.to_path_buf() })
        .map_err(io::Error::other)?;
    fs::write(pointer, text)
}

fn paths_equal(a: &Path, b: &Path) -> bool {
    let canon = |p: &Path| fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    canon(a) == canon(b)
}

/// Folder layout under a data root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Layout {
    pub root: PathBuf,
}

impl Layout {
    pub fn new(root: impl Into<PathBuf>) -> Layout {
        Layout { root: root.into() }
    }
    pub fn versions_dir(&self) -> PathBuf {
        self.root.join("versions")
    }
    pub fn source_dir(&self, source: &str) -> PathBuf {
        self.versions_dir().join(source)
    }
    pub fn game_dir(&self, game: &str) -> PathBuf {
        self.root.join("games").join(game)
    }
    pub fn game_data_dir(&self, game: &str) -> PathBuf {
        self.game_dir(game).join("data")
    }
    /// Extraction goes here first and is renamed over `data/` only on success, so a failed or
    /// cancelled run never damages an existing install.
    pub fn game_staging_dir(&self, game: &str) -> PathBuf {
        self.game_dir(game).join("data.staging")
    }
    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }
    pub fn settings_dir(&self) -> PathBuf {
        self.root.join("settings")
    }
    pub fn settings_file(&self) -> PathBuf {
        self.settings_dir().join("launcher.json")
    }
    pub fn ensure(&self) -> io::Result<()> {
        for d in [self.versions_dir(), self.logs_dir(), self.settings_dir(), self.root.join("games")] {
            fs::create_dir_all(d)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum MoveError {
    Same,
    Inside,
    NotEmpty(PathBuf),
    Io(io::Error),
}

impl std::fmt::Display for MoveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MoveError::Same => write!(f, "That is already the data folder."),
            MoveError::Inside => write!(f, "The new folder cannot be inside the current data folder."),
            MoveError::NotEmpty(p) => write!(
                f,
                "\"{}\" already exists in the chosen folder. Pick an empty folder.",
                p.display()
            ),
            MoveError::Io(e) => write!(f, "Moving failed: {e}"),
        }
    }
}

impl From<io::Error> for MoveError {
    fn from(e: io::Error) -> Self {
        MoveError::Io(e)
    }
}

/// Moves the managed entries from `from` to `to` and records the new location in the pointer
/// file under `default_root`. Uses rename where possible, copy + delete across volumes.
pub fn move_data_root(default_root: &Path, from: &Path, to: &Path) -> Result<(), MoveError> {
    if paths_equal(from, to) {
        return Err(MoveError::Same);
    }
    let canon_from = fs::canonicalize(from).unwrap_or_else(|_| from.to_path_buf());
    let canon_to_parent = to
        .parent()
        .and_then(|p| fs::canonicalize(p).ok())
        .map(|p| p.join(to.file_name().unwrap_or_default()))
        .unwrap_or_else(|| to.to_path_buf());
    if canon_to_parent.starts_with(&canon_from) {
        return Err(MoveError::Inside);
    }
    for entry in MANAGED_ENTRIES {
        let dst = to.join(entry);
        if dst.exists() {
            return Err(MoveError::NotEmpty(dst));
        }
    }
    fs::create_dir_all(to)?;
    for entry in MANAGED_ENTRIES {
        let src = from.join(entry);
        if !src.exists() {
            continue;
        }
        let dst = to.join(entry);
        if fs::rename(&src, &dst).is_err() {
            copy_dir_all(&src, &dst)?;
            fs::remove_dir_all(&src)?;
        }
    }
    write_location(default_root, to)?;
    Ok(())
}

pub fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else if ty.is_symlink() {
            let link = fs::read_link(entry.path())?;
            #[cfg(unix)]
            std::os::unix::fs::symlink(link, &target)?;
            #[cfg(not(unix))]
            {
                let _ = link;
                fs::copy(entry.path(), &target)?;
            }
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// A unique scratch dir under the system temp dir, removed on drop.
    pub struct TempDir(pub PathBuf);
    impl TempDir {
        pub fn new(tag: &str) -> TempDir {
            static N: AtomicU32 = AtomicU32::new(0);
            let p = std::env::temp_dir().join(format!(
                "randcrw-launcher-test-{tag}-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::SeqCst)
            ));
            let _ = fs::remove_dir_all(&p);
            fs::create_dir_all(&p).unwrap();
            TempDir(p)
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        move |k| map.get(k).cloned()
    }

    #[test]
    fn default_roots_per_os() {
        let mac = default_data_root_for("macos", &env_of(&[("HOME", "/Users/a")])).unwrap();
        assert_eq!(mac, PathBuf::from("/Users/a/Library/Application Support/randcrw"));

        let win = default_data_root_for("windows", &env_of(&[("LOCALAPPDATA", "C:/Users/a/AppData/Local")])).unwrap();
        assert_eq!(win, PathBuf::from("C:/Users/a/AppData/Local").join("randcrw"));
        let win2 = default_data_root_for("windows", &env_of(&[("USERPROFILE", "C:/Users/a")])).unwrap();
        assert_eq!(win2, PathBuf::from("C:/Users/a").join("AppData").join("Local").join("randcrw"));

        let xdg = default_data_root_for("linux", &env_of(&[("XDG_DATA_HOME", "/d"), ("HOME", "/h")])).unwrap();
        assert_eq!(xdg, PathBuf::from("/d/randcrw"));
        let lin = default_data_root_for("linux", &env_of(&[("XDG_DATA_HOME", ""), ("HOME", "/h")])).unwrap();
        assert_eq!(lin, PathBuf::from("/h/.local/share/randcrw"));

        assert_eq!(default_data_root_for("linux", &env_of(&[])), None);
    }

    #[test]
    fn layout_matches_contract() {
        let l = Layout::new("/r");
        assert_eq!(l.versions_dir(), PathBuf::from("/r/versions"));
        assert_eq!(l.source_dir("official"), PathBuf::from("/r/versions/official"));
        assert_eq!(l.game_data_dir("rac1"), PathBuf::from("/r/games/rac1/data"));
        assert_eq!(l.logs_dir(), PathBuf::from("/r/logs"));
        assert_eq!(l.settings_file(), PathBuf::from("/r/settings/launcher.json"));
        assert!(!l.root.to_string_lossy().contains("randcre"));
    }

    #[test]
    fn resolve_prefers_env_then_pointer_then_default() {
        let t = TempDir::new("resolve");
        let def = t.0.join("default");
        assert_eq!(resolve_data_root(&def, None), def);
        assert_eq!(resolve_data_root(&def, Some("/env".into())), PathBuf::from("/env"));
        let moved = t.0.join("moved");
        write_location(&def, &moved).unwrap();
        assert_eq!(resolve_data_root(&def, None), moved);
        // A relative pointer is ignored.
        fs::write(def.join(LOCATION_FILE), r#"{"data_root":"relative"}"#).unwrap();
        assert_eq!(resolve_data_root(&def, None), def);
    }

    #[test]
    fn move_and_move_back() {
        let t = TempDir::new("move");
        let def = t.0.join("default");
        let layout = Layout::new(&def);
        layout.ensure().unwrap();
        fs::create_dir_all(layout.game_data_dir("rac1")).unwrap();
        fs::write(layout.game_data_dir("rac1").join("a.bin"), b"abc").unwrap();
        fs::write(layout.settings_file(), "{}").unwrap();

        let dest = t.0.join("elsewhere").join("randcrw");
        move_data_root(&def, &def, &dest).unwrap();
        assert_eq!(fs::read(dest.join("games/rac1/data/a.bin")).unwrap(), b"abc");
        assert!(dest.join("settings/launcher.json").exists());
        assert!(!def.join("games").exists());
        assert_eq!(resolve_data_root(&def, None), dest);

        move_data_root(&def, &dest, &def).unwrap();
        assert!(def.join("games/rac1/data/a.bin").exists());
        assert!(!def.join(LOCATION_FILE).exists());
        assert_eq!(resolve_data_root(&def, None), def);
    }

    #[test]
    fn move_refuses_bad_targets() {
        let t = TempDir::new("move-bad");
        let def = t.0.join("default");
        Layout::new(&def).ensure().unwrap();
        assert!(matches!(move_data_root(&def, &def, &def), Err(MoveError::Same)));
        assert!(matches!(move_data_root(&def, &def, &def.join("sub")), Err(MoveError::Inside)));
        let busy = t.0.join("busy");
        fs::create_dir_all(busy.join("games")).unwrap();
        assert!(matches!(move_data_root(&def, &def, &busy), Err(MoveError::NotEmpty(_))));
        // Nothing moved.
        assert!(def.join("settings").exists());
    }

    #[test]
    fn copy_dir_all_copies_nested() {
        let t = TempDir::new("copy");
        fs::create_dir_all(t.0.join("a/b")).unwrap();
        fs::write(t.0.join("a/b/c.txt"), "x").unwrap();
        copy_dir_all(&t.0.join("a"), &t.0.join("z")).unwrap();
        assert_eq!(fs::read_to_string(t.0.join("z/b/c.txt")).unwrap(), "x");
    }
}
