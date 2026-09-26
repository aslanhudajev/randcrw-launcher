import { useLauncher } from "../state";
import { BoltMark, IconClose, IconMinus, IconSquare } from "./Icons";

export function TitleBar() {
  const { snapshot, backend } = useLauncher();
  const mac = snapshot?.platform === "macos";
  const runtime = snapshot?.active?.version ?? null;
  const runtimeProblem = snapshot?.active?.problem ?? null;
  return (
    <header className={`titlebar ${mac ? "is-mac" : ""}`} data-tauri-drag-region>
      {mac && backend.mode === "mock" && (
        <span className="tb-fake-lights" aria-hidden="true">
          <i />
          <i />
          <i />
        </span>
      )}
      <div className="tb-brand" data-tauri-drag-region>
        <BoltMark size={22} />
        <span className="tb-name" data-tauri-drag-region>
          rand<b>crw</b>
        </span>
        <span className="tb-sub" data-tauri-drag-region>
          Ratchet &amp; Clank: ReWrite
        </span>
      </div>
      <div className="tb-spacer" data-tauri-drag-region />
      <div className="tb-chips" data-tauri-drag-region>
        <span className="tb-chip" data-tauri-drag-region>
          <em>Launcher</em> v{snapshot?.launcher_version ?? "…"}
        </span>
        <span
          className={`tb-chip ${runtime ? "" : "is-warn"}`}
          title={runtimeProblem ?? (runtime ? "Active game version" : "No game version is active")}
          data-tauri-drag-region
        >
          <em>Runtime</em> {runtime ? `v${runtime}` : "not set!"}
        </span>
      </div>
      {!mac && (
        <div className="tb-controls">
          <button aria-label="Minimize" onClick={() => void backend.window.minimize()}>
            <IconMinus size={16} />
          </button>
          <button aria-label="Maximize" onClick={() => void backend.window.toggleMaximize()}>
            <IconSquare size={14} />
          </button>
          <button aria-label="Close" className="is-close" onClick={() => void backend.window.close()}>
            <IconClose size={16} />
          </button>
        </div>
      )}
    </header>
  );
}
