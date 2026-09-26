import { useLauncher } from "../state";
import { IconCheck, IconInfo, IconWarn } from "./Icons";

export function Toasts() {
  const { toasts } = useLauncher();
  return (
    <div className="toasts" role="status" aria-live="polite">
      {toasts.map((t) => (
        <div key={t.id} className={`toast is-${t.tone}`}>
          {t.tone === "ok" ? <IconCheck size={18} /> : t.tone === "warn" ? <IconWarn size={18} /> : <IconInfo size={18} />}
          <span>{t.text}</span>
        </div>
      ))}
    </div>
  );
}
