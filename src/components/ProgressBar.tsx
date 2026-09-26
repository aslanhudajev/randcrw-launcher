export function ProgressBar({ value, indeterminate }: { value: number; indeterminate?: boolean }) {
  const pct = Math.max(0, Math.min(1, value)) * 100;
  return (
    <div
      className={`pbar ${indeterminate ? "is-indeterminate" : ""}`}
      role="progressbar"
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={indeterminate ? undefined : Math.round(pct)}
    >
      <div className="pbar-fill" style={{ width: indeterminate ? undefined : `${pct}%` }} />
      <div className="pbar-ticks" />
    </div>
  );
}
