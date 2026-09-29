//! End to end against a real ReRAC release and your own disc image, through the launcher
//! backend (the same functions the Tauri commands call):
//!
//! 1. install the release zip as a Development version (`install::install_archive`);
//! 2. extract the ISO with that version's extractor (`extractor::JobManager`, staging + promote);
//! 3. launch the runtime (`launch::plan` + `GameProcess`) with `RC_SCENE=0 RC_SCREENSHOT_FRAME=300`,
//!    which renders offscreen, writes a PNG and exits 0;
//! 4. runtime exit 3 (bogus data folder) and exit 4 (edited `extract-info.json`).
//!
//! Opt-in (ignored by default; about 4 GiB is written and deleted again):
//!
//! ```sh
//! # RERAC_GAME_DIR: your local checkout of re-rac/rerac (e.g. the sibling folder ../randcre)
//! RERAC_E2E_ZIP="$RERAC_GAME_DIR/dist/rerac-0.1.0-macos-arm64.zip" \
//! RERAC_E2E_ISO="$HOME/PS2/ratchet1/Ratchet & Clank (USA) (En,Fr,De,Es,It).iso" \
//! RERAC_E2E_OUT=/some/scratch/dir \
//! cargo test --test e2e_real_build -- --ignored --nocapture
//! ```
//!
//! The screenshot is kept at `$RERAC_E2E_OUT/rac1-e2e.png` and the logs under
//! `$RERAC_E2E_OUT/root/logs/`; the game data and the installed version are deleted.
//! The game window opens once (blank; the capture is offscreen).

use rerac_launcher_lib::contract::EXTRACT_INFO_FILE;
use rerac_launcher_lib::extractor::{self, FinishStatus, JobKind, JobManager, JobSpec};
use rerac_launcher_lib::install;
use rerac_launcher_lib::launch::{self, ExitedPayload, GameProcess, LaunchPlan};
use rerac_launcher_lib::paths::Layout;
use rerac_launcher_lib::settings::Settings;
use rerac_launcher_lib::versions::{self, SourceId};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Deletes the big folders even when an assertion fails.
struct Cleanup(Vec<PathBuf>);
impl Drop for Cleanup {
    fn drop(&mut self) {
        for p in &self.0 {
            let _ = fs::remove_dir_all(p);
        }
    }
}

fn run_game(plan: &LaunchPlan, env: &[(&str, &str)], timeout: Duration) -> ExitedPayload {
    let gp = GameProcess::default();
    let (tx, rx) = mpsc::channel();
    gp.launch(plan, env, move |p| tx.send(p).unwrap()).unwrap();
    rx.recv_timeout(timeout).expect("the runtime did not exit in time")
}

fn log_text(p: &Path) -> String {
    fs::read_to_string(p).unwrap_or_default()
}

#[test]
#[ignore]
fn real_build_install_extract_play() {
    let (Ok(zip), Ok(iso), Ok(out)) =
        (std::env::var("RERAC_E2E_ZIP"), std::env::var("RERAC_E2E_ISO"), std::env::var("RERAC_E2E_OUT"))
    else {
        eprintln!("RERAC_E2E_ZIP / RERAC_E2E_ISO / RERAC_E2E_OUT not set; skipping");
        return;
    };
    let out = PathBuf::from(out);
    let root = out.join("root");
    let _ = fs::remove_dir_all(&root);
    let layout = Layout::new(&root);
    layout.ensure().unwrap();
    let _cleanup = Cleanup(vec![root.join("games"), root.join("versions")]);

    // 1. Install the release zip.
    let t = Instant::now();
    let installed = install::install_archive(
        Path::new(&zip),
        SourceId::Development,
        &layout.source_dir(SourceId::Development.dir_name()),
        None,
        &["rac1"],
    )
    .unwrap();
    eprintln!(
        "installed ReRAC {} (runtime answered {} / data_format {}) into {} in {} ms",
        installed.manifest.version,
        installed.runtime.version,
        installed.runtime.data_format,
        installed.dir.display(),
        t.elapsed().as_millis()
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for b in ["rerac", "rerac-extract"] {
            let m = fs::metadata(installed.dir.join(b)).unwrap().permissions().mode();
            assert_eq!(m & 0o111, 0o111, "{b} lost its exec bits");
        }
    }
    assert!(installed.dir.join("assets/shaders").is_dir(), "assets/ must sit next to rerac");

    let settings = Settings { active_version: Some(installed.vref.clone()), ..Settings::default() };
    settings.save(&layout.settings_file()).unwrap();
    let listed = versions::list(&layout, &settings);
    assert!(listed.iter().any(|v| v.active && v.managed && v.problem.is_none()));

    // 2. Extract with the active version's extractor.
    let v = versions::resolve_active(&layout, &settings).unwrap();
    assert_eq!(v.extractor, installed.dir.join("rerac-extract"));
    let spec = JobSpec::new(JobKind::Extract, "rac1", &v, &layout, Some(PathBuf::from(&iso)), settings.ntsc_only);
    let jm = JobManager::default();
    let (ftx, frx) = mpsc::channel();
    jm.start(spec, |_| {}, move |f| ftx.send(f).unwrap()).unwrap();
    let fin = frx.recv_timeout(Duration::from_secs(600)).unwrap();
    assert_eq!(fin.status, FinishStatus::Ok, "extract failed: {} ({})", fin.message, fin.code);
    let data = layout.game_data_dir("rac1");
    let info = extractor::read_install(&data).unwrap();
    eprintln!("extracted {} files, {} bytes in {} ms", info.files, info.bytes, fin.elapsed_ms);
    assert_eq!(info.disc, "SCUS_971.99");
    assert_eq!(info.data_format, installed.manifest.data_format);
    assert!(!layout.game_staging_dir("rac1").exists());

    // 3. Play: offscreen capture of frame 300, then exit 0.
    let plan = launch::plan(&layout, &settings, "rac1").unwrap();
    assert_eq!(plan.runtime, installed.dir.join("rerac"));
    let png = out.join("rac1-e2e.png");
    let _ = fs::remove_file(&png);
    let game_settings = out.join("rac1-e2e-settings.ron");
    let png_s = png.to_string_lossy().into_owned();
    let gs_s = game_settings.to_string_lossy().into_owned();
    let t = Instant::now();
    let exited = run_game(
        &plan,
        &[
            ("RC_SCENE", "0"),
            ("RC_SCREENSHOT_FRAME", "300"),
            ("RC_SCREENSHOT", &png_s),
            ("RC_SETTINGS_FILE", &gs_s),
        ],
        Duration::from_secs(600),
    );
    eprintln!("game run: exit {:?} after {} ms, log {}", exited.code, t.elapsed().as_millis(), exited.log.display());
    assert_eq!(exited.code, Some(0), "game log:\n{}", log_text(&exited.log));
    assert!(exited.log.starts_with(layout.logs_dir()));
    let bytes = fs::read(&png).expect("no screenshot written");
    assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"), "not a PNG");
    eprintln!("screenshot: {} ({} bytes)", png.display(), bytes.len());

    // 4a. Exit 3: a folder that is not game data.
    let bogus = out.join("not-game-data");
    fs::create_dir_all(&bogus).unwrap();
    let bad_plan = LaunchPlan { data_dir: bogus.clone(), ..plan.clone() };
    let e3 = run_game(&bad_plan, &[("RC_SETTINGS_FILE", &gs_s)], Duration::from_secs(60));
    eprintln!("bogus data dir: exit {:?}, {:?}", e3.code, e3.message);
    assert_eq!(e3.code, Some(launch::EXIT_NO_DATA));
    assert!(e3.message.as_deref().is_some_and(|m| m.starts_with("error:")));
    let _ = fs::remove_dir_all(&bogus);

    // 4b. Exit 4: extract-info.json claims another data_format. The launcher itself refuses
    // (stale data); the runtime, started anyway, exits 4.
    let info_path = data.join(EXTRACT_INFO_FILE);
    let original = fs::read_to_string(&info_path).unwrap();
    let mut json: serde_json::Value = serde_json::from_str(&original).unwrap();
    json["data_format"] = serde_json::json!(info.data_format + 998);
    fs::write(&info_path, json.to_string()).unwrap();
    let refused = launch::plan(&layout, &settings, "rac1").unwrap_err();
    assert!(refused.contains("Re-extract"), "{refused}");
    let e4 = run_game(&plan, &[("RC_SETTINGS_FILE", &gs_s)], Duration::from_secs(60));
    eprintln!("edited extract-info: exit {:?}, {:?}", e4.code, e4.message);
    assert_eq!(e4.code, Some(launch::EXIT_DATA_MISMATCH));
    assert!(e4.message.as_deref().is_some_and(|m| m.starts_with("error:")));
    fs::write(&info_path, original).unwrap();
    assert!(launch::plan(&layout, &settings, "rac1").is_ok());
}
