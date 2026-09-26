//! Starts the game: `<runtime> --data-dir <game_data_dir>` with `RC_DATA_DIR` set too
//! (contract §Runtime). Output goes to `logs/<game>-<unix time>.log`.

use crate::contract::DATA_DIR_ENV;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const EXITED: &str = "game://exited";

#[derive(Debug, Clone, Serialize)]
pub struct ExitedPayload {
    pub game: String,
    pub code: Option<i32>,
    pub log: PathBuf,
    /// The runtime's `error: ...` line (stderr) when it could not start.
    pub message: Option<String>,
}

/// Runtime exit codes when it cannot start (contract §Runtime).
pub const EXIT_BAD_ARGS: i32 = 2;
pub const EXIT_NO_DATA: i32 = 3;
pub const EXIT_DATA_MISMATCH: i32 = 4;

/// Last `error: ...` line of a runtime log.
pub fn error_line(log: &str) -> Option<String> {
    log.lines().rev().find(|l| l.starts_with("error:")).map(|l| l.trim().to_string())
}

#[derive(Default)]
pub struct GameProcess {
    running: Arc<Mutex<Option<String>>>,
}

pub fn command(runtime: &Path, workdir: &Path, data_dir: &Path) -> Command {
    let mut c = Command::new(runtime);
    c.arg("--data-dir").arg(data_dir).env(DATA_DIR_ENV, data_dir).current_dir(workdir);
    c
}

impl GameProcess {
    pub fn running(&self) -> Option<String> {
        self.running.lock().unwrap().clone()
    }

    pub fn launch(
        &self,
        game: &str,
        runtime: &Path,
        workdir: &Path,
        data_dir: &Path,
        logs_dir: &Path,
        on_exit: impl FnOnce(ExitedPayload) + Send + 'static,
    ) -> Result<(), String> {
        let mut slot = self.running.lock().unwrap();
        if slot.is_some() {
            return Err("The game is already running.".into());
        }
        fs::create_dir_all(logs_dir).map_err(|e| e.to_string())?;
        let unix = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let log_path = logs_dir.join(format!("{game}-{unix}.log"));
        let log = fs::File::create(&log_path).map_err(|e| e.to_string())?;
        let log_err = log.try_clone().map_err(|e| e.to_string())?;
        let child: Child = command(runtime, workdir, data_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(log_err))
            .spawn()
            .map_err(|e| format!("Could not start the game ({}): {e}", runtime.display()))?;
        *slot = Some(game.to_string());
        drop(slot);

        let running = self.running.clone();
        let game = game.to_string();
        std::thread::spawn(move || {
            let mut child = child;
            let code = loop {
                match child.try_wait() {
                    Ok(Some(s)) => break s.code(),
                    Ok(None) => std::thread::sleep(Duration::from_millis(200)),
                    Err(_) => break None,
                }
            };
            *running.lock().unwrap() = None;
            let message = if code == Some(0) {
                None
            } else {
                fs::read_to_string(&log_path).ok().as_deref().and_then(error_line)
            };
            on_exit(ExitedPayload { game, code, log: log_path, message });
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_passes_data_dir_both_ways() {
        let c = command(Path::new("/v/bin/randcrw"), Path::new("/v"), Path::new("/r/games/rac1/data"));
        let args: Vec<_> = c.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        assert_eq!(args, ["--data-dir", "/r/games/rac1/data"]);
        let env: Vec<_> = c
            .get_envs()
            .map(|(k, v)| (k.to_string_lossy().into_owned(), v.map(|v| v.to_string_lossy().into_owned())))
            .collect();
        assert_eq!(env, [("RC_DATA_DIR".to_string(), Some("/r/games/rac1/data".to_string()))]);
        assert_eq!(c.get_current_dir(), Some(Path::new("/v")));
    }

    #[test]
    fn finds_the_runtime_error_line() {
        assert_eq!(
            error_line("booting\nerror: data_format 2 does not match 1\n"),
            Some("error: data_format 2 does not match 1".to_string())
        );
        assert_eq!(error_line("all good\n"), None);
    }

    #[cfg(unix)]
    #[test]
    fn early_exit_reports_code_and_error_line() {
        use std::os::unix::fs::PermissionsExt;
        let t = crate::paths::tests::TempDir::new("launch");
        let rt = t.0.join("rt");
        fs::write(&rt, "#!/bin/sh\necho 'error: the data folder is incomplete' >&2\nexit 3\n").unwrap();
        fs::set_permissions(&rt, fs::Permissions::from_mode(0o755)).unwrap();
        let gp = GameProcess::default();
        let (tx, rx) = std::sync::mpsc::channel();
        gp.launch("rac1", &rt, &t.0, &t.0.join("data"), &t.0.join("logs"), move |p| tx.send(p).unwrap())
            .unwrap();
        let p = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(p.code, Some(EXIT_NO_DATA));
        assert_eq!(p.message.as_deref(), Some("error: the data folder is incomplete"));
        assert!(gp.running().is_none());
    }
}
