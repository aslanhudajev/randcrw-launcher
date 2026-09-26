import { useEffect, useState, type ReactNode } from "react";
import { errorText } from "../backend/api";
import type { GameStatus, Stage } from "../backend/contract";
import { Backdrop } from "../components/Backdrop";
import { IconDisc, IconDots, IconFolder, IconPlay, IconRefresh, IconShield, IconTrash, IconWarn } from "../components/Icons";
import type { Backend } from "../backend/api";
import { Menu } from "../components/Menu";
import { Modal } from "../components/Modal";
import { ProgressBar } from "../components/ProgressBar";
import { friendlyError, friendlyRuntimeError } from "../lib/errors";
import { formatBytes, formatDuration, prettySerial } from "../lib/format";
import type { GameDef } from "../lib/games";
import { useLauncher, type JobResult, type JobView } from "../state";

export function GameScreen({ game, go }: { game: GameDef; go: (p: string) => void }) {
  const { backend, snapshot, job, results, clearResult, statusTick, bumpStatus, run, toast, refresh } = useLauncher();
  const [status, setStatus] = useState<GameStatus | null>(null);
  const [confirm, setConfirm] = useState<"uninstall" | "reextract" | null>(null);
  const [starting, setStarting] = useState(false);

  useEffect(() => {
    let alive = true;
    backend
      .gameStatus(game.id)
      .then((s) => alive && setStatus(s))
      .catch((e) => toast(errorText(e), "warn"));
    return () => {
      alive = false;
    };
  }, [backend, game.id, statusTick, toast, snapshot?.game_running, snapshot?.settings.active_version?.id]);

  const myJob = job && job.game === game.id ? job : null;
  const result = results[game.id];
  const failed = result && result.status === "error" ? result : null;
  const installed = !!status?.installed;
  const running = !!status?.running || snapshot?.game_running === game.id;
  const hasVersion = !!snapshot?.active && !snapshot.active.problem;

  async function install() {
    const iso = await run(() => backend.pickIso());
    if (!iso) return;
    clearResult(game.id);
    setStarting(true);
    await run(() => backend.startExtract(game.id, iso));
    setStarting(false);
  }

  async function play() {
    clearResult(game.id);
    const ok = await run(async () => {
      await backend.launchGame(game.id);
      return true;
    });
    if (ok) {
      toast(`Starting randcrw${snapshot?.active?.version ? ` v${snapshot.active.version}` : ""}…`, "info");
      bumpStatus();
      void refresh();
    }
  }

  async function verify() {
    clearResult(game.id);
    await run(() => backend.startVerify(game.id));
  }

  async function uninstall() {
    setConfirm(null);
    const ok = await run(async () => {
      await backend.uninstallGame(game.id);
      return true;
    });
    if (ok) {
      clearResult(game.id);
      toast("Game data removed.", "info");
      bumpStatus();
    }
  }

  let panel: ReactNode;
  if (myJob || starting) {
    panel = <ProgressPanel job={myJob} onCancel={() => void run(() => backend.cancelJob())} />;
  } else if (failed) {
    panel = (
      <ErrorPanel
        result={failed}
        backend={backend}
        onPickAnother={install}
        onReextract={() => setConfirm("reextract")}
        onDismiss={() => clearResult(game.id)}
        installed={installed}
      />
    );
  } else if (!hasVersion) {
    panel = <NoVersionPanel problem={snapshot?.active?.problem ?? null} onOpen={() => go("settings/versions/development")} />;
  } else if (installed && status?.stale) {
    panel = <StalePanel status={status} onReextract={install} />;
  } else if (installed) {
    panel = (
      <div className="gs-ready">
        <button className="btn btn-play" onClick={play} disabled={running}>
          <IconPlay size={22} />
          <span>{running ? "Running…" : "Play"}</span>
        </button>
        <Menu
          label="More options"
          items={[
            { label: "Open data folder", icon: <IconFolder size={18} />, onSelect: () => void run(() => backend.openFolder(`game:${game.id}`)) },
            { label: "Verify files", icon: <IconShield size={18} />, onSelect: verify, disabled: running },
            { label: "Re-extract from ISO", icon: <IconRefresh size={18} />, onSelect: () => setConfirm("reextract"), disabled: running },
            { label: "Uninstall data", icon: <IconTrash size={18} />, onSelect: () => setConfirm("uninstall"), danger: true, disabled: running },
          ]}
        >
          <IconDots size={22} />
        </Menu>
      </div>
    );
  } else {
    panel = (
      <div className="gs-install">
        <p className="gs-note">
          Uses your own disc image of the <strong>US release (SCUS-97199)</strong>. The disc is read once; after that the
          image is no longer needed.
        </p>
        <button className="btn btn-primary btn-lg" onClick={install}>
          <IconDisc size={20} />
          <span>Install via ISO</span>
        </button>
      </div>
    );
  }

  const chip = myJob
    ? { tone: "busy", text: myJob.kind === "extract" ? "Installing" : "Verifying" }
    : running
      ? { tone: "ok", text: "Running" }
      : installed
        ? status?.stale
          ? { tone: "warn", text: "Data outdated" }
          : { tone: "ok", text: "Ready" }
        : { tone: "off", text: "Not installed" };

  return (
    <section className={`game-screen ${installed ? "is-installed" : "is-missing"}`}>
      <Backdrop game={game} mode={installed ? "color" : "grey"} />
      <div className="gs-bottom">
        <div className="gs-title">
          <span className={`status-chip is-${chip.tone}`}>
            <i />
            {chip.text}
          </span>
          <h1>{game.title}</h1>
          {installed && status?.info && (
            <p className="gs-meta">
              {prettySerial(status.info.disc)} · {status.info.files} files · {formatBytes(status.info.bytes)}
              {status.info.ntsc_only ? " · NTSC-only" : ""}
            </p>
          )}
        </div>
        <div className="gs-actions">{panel}</div>
      </div>

      {confirm === "uninstall" && (
        <Modal title="Uninstall game data?" confirmLabel="Uninstall" danger onConfirm={uninstall} onClose={() => setConfirm(null)}>
          <p>
            This deletes the extracted data in <code>{status?.data_dir}</code>. Your saves and settings stay. To play
            again you will need your disc image.
          </p>
        </Modal>
      )}
      {confirm === "reextract" && (
        <Modal
          title="Re-extract from your disc image?"
          confirmLabel="Choose ISO…"
          onConfirm={() => {
            setConfirm(null);
            void install();
          }}
          onClose={() => setConfirm(null)}
        >
          <p>The current data stays in place until the new extraction has finished and checked out.</p>
        </Modal>
      )}
    </section>
  );
}

// `extract` reports identify then copy; files are hashed against the known-good table while
// they are copied, so there is no separate verify stage. `verify` reports only verify.
const EXTRACT_STAGES: { id: Stage; label: string }[] = [
  { id: "identify", label: "Identify disc" },
  { id: "copy", label: "Copy & check files" },
];
const VERIFY_STAGES: { id: Stage; label: string }[] = [{ id: "verify", label: "Check every file" }];

function overall(job: JobView): number {
  const f = job.total > 0 ? job.done / job.total : 0;
  if (job.kind === "verify") return f;
  switch (job.stage) {
    case "identify":
      return 0.03 * f;
    case "copy":
      return 0.03 + 0.97 * f;
    case "verify": // not sent by extract today; treat as the tail end
      return 1;
    default:
      return 0;
  }
}

function ProgressPanel({ job, onCancel }: { job: JobView | null; onCancel(): void }) {
  const [, tick] = useState(0);
  useEffect(() => {
    const t = setInterval(() => tick((n) => n + 1), 1000);
    return () => clearInterval(t);
  }, []);
  const frac = job ? overall(job) : 0;
  const stages = job?.kind === "verify" ? VERIFY_STAGES : EXTRACT_STAGES;
  const idx = job?.stage ? stages.findIndex((s) => s.id === job.stage) : -1;
  let eta: string | null = null;
  if (job?.stage === "copy" && job.copyStart) {
    const dt = Date.now() - job.copyStart.at;
    const dd = job.done - job.copyStart.done;
    if (dt > 1500 && dd > 0) eta = `about ${formatDuration(((job.total - job.done) / dd) * dt)} left`;
  }
  return (
    <div className="panel progress-panel plate">
      <div className="pp-head">
        <span className="pp-kind">{job?.kind === "verify" ? "Verifying files" : "Installing"}</span>
        {job?.disc && (
          <span className="pp-disc">
            <IconDisc size={14} /> {prettySerial(job.disc.serial)} · {job.disc.region} v{job.disc.version}
          </span>
        )}
      </div>
      <ol className="pp-stages">
        {stages.map((s, i) => (
          <li key={s.id} className={i < idx ? "is-done" : i === idx ? "is-active" : ""}>
            <span className="pp-stage-dot" />
            {s.label}
          </li>
        ))}
      </ol>
      <div className="pp-numbers">
        <span className="pp-pct">
          {Math.floor(frac * 100)}
          <small>%</small>
        </span>
        <span className="pp-bytes">
          {job && job.stage !== "identify" && job.total > 0
            ? `${formatBytes(job.done)} of ${formatBytes(job.total)}`
            : "Reading the disc…"}
          {eta && <em> · {eta}</em>}
        </span>
      </div>
      <ProgressBar value={frac} indeterminate={!job || !job.stage} />
      <div className="pp-foot">
        <span className="pp-file mono" title={job?.file}>
          {job?.file || "Starting…"}
        </span>
        <button className="btn btn-ghost btn-sm" onClick={onCancel} disabled={!job}>
          Cancel
        </button>
      </div>
    </div>
  );
}

function ErrorPanel({
  result,
  backend,
  installed,
  onPickAnother,
  onReextract,
  onDismiss,
}: {
  result: JobResult;
  backend: Backend;
  installed: boolean;
  onPickAnother(): void;
  onReextract(): void;
  onDismiss(): void;
}) {
  const launch = result.kind === "launch";
  const rt = launch ? friendlyRuntimeError(result.code) : null;
  const f = rt ?? friendlyError(result.code, result.disc, result.message);
  const verifyFail = result.kind === "verify" || !!rt?.reextract;
  return (
    <div className="panel error-panel plate" role="alert">
      <div className="ep-head">
        <IconWarn size={22} />
        <span className="ep-code">{launch ? (result.code < 0 ? "Game stopped" : `Game exit ${result.code}`) : `Error ${result.code}`}</span>
        {result.disc && (
          <span className="ep-disc">
            {prettySerial(result.disc.serial)} · {result.disc.region} v{result.disc.version}
          </span>
        )}
      </div>
      <h3>{f.title}</h3>
      <p>{f.hint}</p>
      <details className="ep-detail">
        <summary>Details</summary>
        <code>{result.message}</code>
      </details>
      <div className="ep-actions">
        <button className="btn btn-ghost btn-sm" onClick={onDismiss}>
          {installed ? "Dismiss" : "Back"}
        </button>
        {launch && result.log && (
          <button className="btn btn-ghost btn-sm" onClick={() => void backend.openLog(result.log!).catch(() => backend.openFolder("logs"))}>
            <IconFolder size={16} /> Open logs
          </button>
        )}
        {launch && !rt?.reextract ? null : verifyFail || (installed && !f.pickAnother) ? (
          <button className="btn btn-primary btn-sm" onClick={onReextract}>
            <IconRefresh size={16} /> Re-extract
          </button>
        ) : (
          <button className="btn btn-primary btn-sm" onClick={onPickAnother}>
            <IconDisc size={16} /> {f.pickAnother ? "Choose another ISO" : "Try again"}
          </button>
        )}
      </div>
    </div>
  );
}

function NoVersionPanel({ problem, onOpen }: { problem: string | null; onOpen(): void }) {
  return (
    <div className="panel notice-panel plate">
      <h3>{problem ? "The active game version can't be used" : "No game version set"}</h3>
      <p>{problem ?? "Installing and playing need a randcrw build. Add one under Version Management."}</p>
      <div className="ep-actions">
        <button className="btn btn-primary btn-sm" onClick={onOpen}>
          Open Version Management
        </button>
      </div>
    </div>
  );
}

function StalePanel({ status, onReextract }: { status: GameStatus; onReextract(): void }) {
  const have = status.info?.data_format;
  const want = status.expected_format;
  return (
    <div className="panel notice-panel plate">
      <h3>Game data needs an update</h3>
      <p>
        The installed data is format {have}, but randcrw {status.active_version ? `v${status.active_version}` : ""} needs
        format {want}. Re-extract from your disc image to continue, or switch back to a version that matches.
      </p>
      <div className="ep-actions">
        <button className="btn btn-primary btn-sm" onClick={onReextract}>
          <IconDisc size={16} /> Re-extract via ISO
        </button>
      </div>
    </div>
  );
}
