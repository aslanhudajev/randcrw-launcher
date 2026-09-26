//! Tauri commands: the page's only way to touch files, processes, dialogs and the network.
//! Errors are user-facing strings.

use crate::contract::ExtractInfo;
use crate::extractor::{self, EventPayload, FinishedPayload, JobKind, JobSpec, JobState};
use crate::github;
use crate::launch::{self, ExitedPayload, GameProcess};
use crate::paths::{self, Layout};
use crate::settings::Settings;
use crate::versions::{self, SourceId, VersionInfo, VersionRef};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, RwLock};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

type CmdResult<T> = Result<T, String>;

/// Games the launcher knows. Only these ids are accepted from the page.
pub const GAMES: [&str; 3] = ["rac1", "rac2", "rac3"];
/// Games that can be installed today.
pub const INSTALLABLE: [&str; 1] = ["rac1"];

pub struct Launcher {
    pub default_root: PathBuf,
    pub layout: RwLock<Layout>,
    pub settings: Mutex<Settings>,
    pub jobs: extractor::JobManager,
    pub game: GameProcess,
}

impl Launcher {
    pub fn init(default_root: PathBuf, env_root: Option<PathBuf>) -> Launcher {
        let root = paths::resolve_data_root(&default_root, env_root);
        let layout = Layout::new(root);
        if let Err(e) = layout.ensure() {
            eprintln!("[randcrw-launcher] cannot create {}: {e}", layout.root.display());
        }
        let settings = Settings::load(&layout.settings_file());
        Launcher {
            default_root,
            layout: RwLock::new(layout),
            settings: Mutex::new(settings),
            jobs: Default::default(),
            game: Default::default(),
        }
    }

    fn layout(&self) -> Layout {
        self.layout.read().unwrap().clone()
    }

    fn settings(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    fn update_settings(&self, f: impl FnOnce(&mut Settings)) -> CmdResult<Settings> {
        let mut s = self.settings.lock().unwrap();
        f(&mut s);
        s.save(&self.layout().settings_file())
            .map_err(|e| format!("Could not save settings: {e}"))?;
        Ok(s.clone())
    }
}

fn check_game(game: &str) -> CmdResult<()> {
    if GAMES.contains(&game) {
        Ok(())
    } else {
        Err(format!("Unknown game \"{game}\""))
    }
}

fn check_installable(game: &str) -> CmdResult<()> {
    check_game(game)?;
    if INSTALLABLE.contains(&game) {
        Ok(())
    } else {
        Err("This game is not supported yet.".into())
    }
}

// ---------------------------------------------------------------------------------------------
// Snapshot

#[derive(Serialize)]
pub struct ActiveSummary {
    #[serde(rename = "ref")]
    pub vref: VersionRef,
    pub version: Option<String>,
    pub game: Option<String>,
    pub data_format: Option<u32>,
    pub problem: Option<String>,
}

#[derive(Serialize)]
pub struct AppSnapshot {
    pub launcher_version: String,
    pub platform: &'static str,
    pub data_root: PathBuf,
    pub default_data_root: PathBuf,
    pub settings: Settings,
    pub active: Option<ActiveSummary>,
    pub job: Option<JobState>,
    pub game_running: Option<String>,
}

fn snapshot(app: &AppHandle, st: &Launcher) -> AppSnapshot {
    let layout = st.layout();
    let settings = st.settings();
    let active = settings.active_version.clone().map(|vref| {
        match versions::resolve(&layout, &settings, &vref) {
            Ok(v) => ActiveSummary {
                vref,
                version: Some(v.manifest.version),
                game: Some(v.manifest.game),
                data_format: Some(v.manifest.data_format),
                problem: None,
            },
            Err(e) => ActiveSummary { vref, version: None, game: None, data_format: None, problem: Some(e) },
        }
    });
    AppSnapshot {
        launcher_version: app.package_info().version.to_string(),
        platform: std::env::consts::OS,
        data_root: layout.root.clone(),
        default_data_root: st.default_root.clone(),
        settings,
        active,
        job: st.jobs.current(),
        game_running: st.game.running(),
    }
}

#[tauri::command]
pub fn get_snapshot(app: AppHandle, st: State<'_, Launcher>) -> AppSnapshot {
    snapshot(&app, &st)
}

// ---------------------------------------------------------------------------------------------
// Game install state

#[derive(Serialize)]
pub struct GameStatus {
    pub game: String,
    pub data_dir: PathBuf,
    pub installed: bool,
    pub info: Option<ExtractInfo>,
    /// Installed data has a different `data_format` than the active version expects.
    pub stale: bool,
    pub job: Option<JobState>,
    pub running: bool,
}

#[tauri::command]
pub fn game_status(game: String, st: State<'_, Launcher>) -> CmdResult<GameStatus> {
    check_game(&game)?;
    let layout = st.layout();
    let settings = st.settings();
    let data_dir = layout.game_data_dir(&game);
    let info = extractor::read_install(&data_dir);
    let expected = versions::resolve_active(&layout, &settings).ok().map(|v| v.manifest.data_format);
    let stale = matches!((&info, expected), (Some(i), Some(f)) if i.data_format != f);
    Ok(GameStatus {
        installed: info.is_some(),
        stale,
        info,
        data_dir,
        job: st.jobs.current().filter(|j| j.game == game),
        running: st.game.running().as_deref() == Some(game.as_str()),
        game,
    })
}

#[tauri::command]
pub async fn pick_iso(app: AppHandle) -> CmdResult<Option<String>> {
    let picked = app
        .dialog()
        .file()
        .set_title("Choose your Ratchet & Clank disc image")
        .add_filter("PS2 disc image (.iso)", &["iso", "ISO"])
        .blocking_pick_file();
    match picked {
        None => Ok(None),
        Some(fp) => {
            let p = fp.into_path().map_err(|e| e.to_string())?;
            ensure_iso(&p)?;
            Ok(Some(p.to_string_lossy().into_owned()))
        }
    }
}

fn ensure_iso(p: &Path) -> CmdResult<()> {
    let is_iso = p.extension().is_some_and(|e| e.eq_ignore_ascii_case("iso"));
    if is_iso {
        Ok(())
    } else {
        Err("Please choose a .iso disc image.".into())
    }
}

fn job_spec(st: &Launcher, game: &str, kind: JobKind, iso: Option<PathBuf>) -> CmdResult<JobSpec> {
    check_installable(game)?;
    if st.game.running().is_some() {
        return Err("Close the game first.".into());
    }
    let layout = st.layout();
    let settings = st.settings();
    let v = versions::resolve_active(&layout, &settings)?;
    if v.manifest.game != game {
        return Err(format!("The active version is for {}, not {game}.", v.manifest.game));
    }
    Ok(JobSpec {
        kind,
        game: game.to_string(),
        extractor: v.extractor,
        workdir: v.dir,
        iso,
        ntsc_only: settings.ntsc_only,
        data_dir: layout.game_data_dir(game),
        staging_dir: layout.game_staging_dir(game),
        logs_dir: layout.logs_dir(),
    })
}

fn start_job(app: &AppHandle, st: &Launcher, spec: JobSpec) -> CmdResult<u64> {
    let a1 = app.clone();
    let a2 = app.clone();
    st.jobs.start(
        spec,
        move |p: EventPayload| {
            let _ = a1.emit(extractor::EVENT, p);
        },
        move |p: FinishedPayload| {
            let _ = a2.emit(extractor::FINISHED, p);
        },
    )
}

#[tauri::command]
pub fn start_extract(app: AppHandle, st: State<'_, Launcher>, game: String, iso: String) -> CmdResult<u64> {
    let iso = PathBuf::from(iso);
    ensure_iso(&iso)?;
    if !iso.is_file() {
        return Err(format!("{} does not exist.", iso.display()));
    }
    let spec = job_spec(&st, &game, JobKind::Extract, Some(iso))?;
    start_job(&app, &st, spec)
}

#[tauri::command]
pub fn start_verify(app: AppHandle, st: State<'_, Launcher>, game: String) -> CmdResult<u64> {
    let spec = job_spec(&st, &game, JobKind::Verify, None)?;
    if extractor::read_install(&spec.data_dir).is_none() {
        return Err("The game data is not installed.".into());
    }
    start_job(&app, &st, spec)
}

#[tauri::command]
pub fn cancel_job(st: State<'_, Launcher>) -> bool {
    st.jobs.cancel()
}

#[tauri::command]
pub fn uninstall_game(st: State<'_, Launcher>, game: String) -> CmdResult<()> {
    check_installable(&game)?;
    if st.jobs.is_busy() || st.game.running().is_some() {
        return Err("Wait for the running job or game to finish first.".into());
    }
    let layout = st.layout();
    for d in [layout.game_data_dir(&game), layout.game_staging_dir(&game)] {
        extractor::remove_dir_if_exists(&d).map_err(|e| format!("Could not remove {}: {e}", d.display()))?;
    }
    Ok(())
}

#[tauri::command]
pub fn launch_game(app: AppHandle, st: State<'_, Launcher>, game: String) -> CmdResult<()> {
    check_installable(&game)?;
    if st.jobs.is_busy() {
        return Err("Wait for the extractor to finish first.".into());
    }
    let layout = st.layout();
    let settings = st.settings();
    let v = versions::resolve_active(&layout, &settings)?;
    let data = layout.game_data_dir(&game);
    let info = extractor::read_install(&data).ok_or("The game data is not installed.")?;
    if info.data_format != v.manifest.data_format {
        return Err(format!(
            "The installed data is format {}, but randcrw {} needs format {}. Re-extract from your disc image.",
            info.data_format, v.manifest.version, v.manifest.data_format
        ));
    }
    let app2 = app.clone();
    st.game.launch(&game, &v.runtime, &v.dir, &data, &layout.logs_dir(), move |p: ExitedPayload| {
        let _ = app2.emit(launch::EXITED, p);
    })
}

// ---------------------------------------------------------------------------------------------
// Folders

fn open_path(app: &AppHandle, p: &Path) -> CmdResult<()> {
    std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
    app.opener()
        .open_path(p.to_string_lossy(), None::<&str>)
        .map_err(|e| e.to_string())
}

/// `which`: `root`, `logs`, `versions`, `game:<id>`, `source:<id>`.
#[tauri::command]
pub fn open_folder(app: AppHandle, st: State<'_, Launcher>, which: String) -> CmdResult<()> {
    let layout = st.layout();
    let p = match which.split_once(':') {
        None if which == "root" => layout.root.clone(),
        None if which == "logs" => layout.logs_dir(),
        None if which == "versions" => layout.versions_dir(),
        Some(("game", g)) => {
            check_game(g)?;
            layout.game_data_dir(g)
        }
        Some(("source", s)) => {
            let src: SourceId = serde_json::from_value(serde_json::Value::String(s.into())).map_err(|e| e.to_string())?;
            match src.location() {
                versions::SourceLocation::Managed(dir) => layout.source_dir(dir),
                versions::SourceLocation::External => return Err("Development builds live in their own folders.".into()),
            }
        }
        Some(("path", p)) => {
            // Only folders of registered development builds.
            let p = PathBuf::from(p);
            if !st.settings().dev_versions.iter().any(|d| d.path == p) {
                return Err("Unknown folder.".into());
            }
            p
        }
        _ => return Err(format!("Unknown folder \"{which}\"")),
    };
    open_path(&app, &p)
}

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> CmdResult<()> {
    if !url.starts_with("https://") {
        return Err("Only https links can be opened.".into());
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pick_folder(app: AppHandle, title: String) -> CmdResult<Option<String>> {
    let picked = app.dialog().file().set_title(title).blocking_pick_folder();
    match picked {
        None => Ok(None),
        Some(fp) => Ok(Some(fp.into_path().map_err(|e| e.to_string())?.to_string_lossy().into_owned())),
    }
}

/// Moves the data root into `<target>/randcrw` (or `target` itself if it is already named
/// randcrw or is the default root).
#[tauri::command]
pub fn move_data_root(app: AppHandle, st: State<'_, Launcher>, target: String) -> CmdResult<AppSnapshot> {
    if st.jobs.is_busy() || st.game.running().is_some() {
        return Err("Wait for the running job or game to finish first.".into());
    }
    let target = PathBuf::from(target);
    if !target.is_absolute() {
        return Err("Choose an absolute folder.".into());
    }
    let to = if target == st.default_root || target.file_name().is_some_and(|n| n == paths::APP_DIR_NAME) {
        target
    } else {
        target.join(paths::APP_DIR_NAME)
    };
    let from = st.layout().root;
    paths::move_data_root(&st.default_root, &from, &to).map_err(|e| e.to_string())?;
    *st.layout.write().unwrap() = Layout::new(to);
    Ok(snapshot(&app, &st))
}

#[tauri::command]
pub fn set_ntsc_only(app: AppHandle, st: State<'_, Launcher>, value: bool) -> CmdResult<AppSnapshot> {
    st.update_settings(|s| s.ntsc_only = value)?;
    Ok(snapshot(&app, &st))
}

// ---------------------------------------------------------------------------------------------
// Versions

#[tauri::command]
pub fn list_versions(st: State<'_, Launcher>) -> Vec<VersionInfo> {
    versions::list(&st.layout(), &st.settings())
}

/// Adds a development build folder and validates it (manifest, binaries, `--version-json`).
/// The folder is kept in the list even when invalid so the user can fix it and re-validate.
#[tauri::command]
pub async fn add_dev_version(st: State<'_, Launcher>, path: String) -> CmdResult<VersionInfo> {
    let p = PathBuf::from(&path);
    if !p.is_absolute() || !p.is_dir() {
        return Err("Choose an existing folder.".into());
    }
    if !p.join(crate::contract::MANIFEST_FILE).is_file() {
        return Err(format!(
            "This folder has no {}. Pick the build folder that contains it.",
            crate::contract::MANIFEST_FILE
        ));
    }
    let settings = st.update_settings(|s| {
        s.add_dev_version(p.clone());
    })?;
    let vref = VersionRef { source: SourceId::Development, id: path };
    let active = settings.is_active(&vref);
    tauri::async_runtime::spawn_blocking(move || versions::validate(vref, p, active))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn validate_version(st: State<'_, Launcher>, vref: VersionRef) -> CmdResult<VersionInfo> {
    let settings = st.settings();
    let dir = versions::version_dir(&st.layout(), &settings, &vref)?;
    let active = settings.is_active(&vref);
    tauri::async_runtime::spawn_blocking(move || versions::validate(vref, dir, active))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_dev_version(app: AppHandle, st: State<'_, Launcher>, path: String) -> CmdResult<AppSnapshot> {
    st.update_settings(|s| s.remove_dev_version(Path::new(&path)))?;
    Ok(snapshot(&app, &st))
}

/// Validates the version fully, then makes it active.
#[tauri::command]
pub async fn set_active_version(app: AppHandle, st: State<'_, Launcher>, vref: VersionRef) -> CmdResult<AppSnapshot> {
    if st.jobs.is_busy() || st.game.running().is_some() {
        return Err("Wait for the running job or game to finish first.".into());
    }
    let settings = st.settings();
    let dir = versions::version_dir(&st.layout(), &settings, &vref)?;
    let v = vref.clone();
    let info = tauri::async_runtime::spawn_blocking(move || versions::validate(v, dir, false))
        .await
        .map_err(|e| e.to_string())?;
    if let Some(problem) = info.problem {
        return Err(problem);
    }
    st.update_settings(|s| s.active_version = Some(vref))?;
    Ok(snapshot(&app, &st))
}

// ---------------------------------------------------------------------------------------------
// Official releases (disabled by default)

pub const DOWNLOAD_PROGRESS: &str = "download://progress";

#[tauri::command]
pub async fn official_releases(st: State<'_, Launcher>) -> CmdResult<Vec<github::OfficialRelease>> {
    let s = st.settings().official;
    if !s.enabled {
        return Err("Official releases are turned off.".into());
    }
    let releases = github::fetch_releases(&s.owner, &s.repo).await?;
    Ok(github::to_rows(releases, &st.layout().source_dir("official")))
}

#[tauri::command]
pub fn set_official_config(
    app: AppHandle,
    st: State<'_, Launcher>,
    enabled: bool,
    owner: String,
    repo: String,
) -> CmdResult<AppSnapshot> {
    github::releases_url(&owner, &repo)?;
    st.update_settings(|s| {
        s.official.enabled = enabled;
        s.official.owner = owner;
        s.official.repo = repo;
    })?;
    Ok(snapshot(&app, &st))
}

#[derive(Clone, Serialize)]
struct DownloadProgress {
    version: String,
    done: u64,
    total: u64,
}

/// Downloads the platform asset of release `version` into `versions/official/<version>/`.
/// Unpacking and install are left for the integration step (no release format exists yet).
#[tauri::command]
pub async fn download_official(app: AppHandle, st: State<'_, Launcher>, version: String) -> CmdResult<String> {
    let s = st.settings().official;
    if !s.enabled {
        return Err("Official releases are turned off.".into());
    }
    let releases = github::fetch_releases(&s.owner, &s.repo).await?;
    let rel = releases
        .into_iter()
        .find(|r| r.tag_name == version)
        .ok_or_else(|| format!("Release {version} not found"))?;
    let asset = github::pick_asset(&rel.assets, std::env::consts::OS, std::env::consts::ARCH)
        .cloned()
        .ok_or("This release has no download for your system.")?;
    let vref = VersionRef { source: SourceId::Official, id: version.clone() };
    let dest = versions::version_dir(&st.layout(), &st.settings(), &vref)?;
    let file = github::download_asset(&asset, &dest, |done, total| {
        let _ = app.emit(DOWNLOAD_PROGRESS, DownloadProgress { version: version.clone(), done, total });
    })
    .await?;
    Ok(file.to_string_lossy().into_owned())
}

pub fn default_root_or_fallback(app: &AppHandle) -> PathBuf {
    paths::default_data_root()
        .or_else(|| app.path().local_data_dir().ok().map(|d| d.join(paths::APP_DIR_NAME)))
        .unwrap_or_else(|| PathBuf::from(paths::APP_DIR_NAME))
}
