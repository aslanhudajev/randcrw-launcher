import { useEffect, useRef, useState, type ReactNode } from "react";

export interface MenuItem {
  label: string;
  icon: ReactNode;
  onSelect(): void;
  danger?: boolean;
  disabled?: boolean;
}

/** A button that opens a small popover menu above it. */
export function Menu({ items, label, children }: { items: MenuItem[]; label: string; children: ReactNode }) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const close = (e: MouseEvent) => {
      if (!ref.current?.contains(e.target as Node)) setOpen(false);
    };
    const key = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    window.addEventListener("mousedown", close);
    window.addEventListener("keydown", key);
    return () => {
      window.removeEventListener("mousedown", close);
      window.removeEventListener("keydown", key);
    };
  }, [open]);
  return (
    <div className="menu" ref={ref}>
      <button className={`btn btn-square ${open ? "is-open" : ""}`} aria-label={label} aria-expanded={open} title={label} onClick={() => setOpen((o) => !o)}>
        {children}
      </button>
      {open && (
        <ul className="menu-pop plate" role="menu">
          {items.map((it) => (
            <li key={it.label}>
              <button
                role="menuitem"
                className={it.danger ? "is-danger" : ""}
                disabled={it.disabled}
                onClick={() => {
                  setOpen(false);
                  it.onSelect();
                }}
              >
                {it.icon}
                <span>{it.label}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
