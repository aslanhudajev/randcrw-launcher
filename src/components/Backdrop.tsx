import { useState } from "react";
import type { GameDef } from "../lib/games";

/**
 * Game background. Uses the game's art when present; otherwise, or if it fails to load, a
 * drawn deep-space backdrop with a hex grid and bolt motifs in the game's accent colour.
 */
export function Backdrop({ game, mode }: { game: GameDef; mode: "color" | "grey" | "blur" }) {
  const [broken, setBroken] = useState(false);
  const art = game.bg && !broken ? game.bg : null;
  return (
    <div className={`backdrop is-${mode}`} style={{ ["--accent" as string]: game.accent }} aria-hidden="true">
      {art ? (
        <img className="bd-img" src={art} alt="" onError={() => setBroken(true)} draggable={false} />
      ) : (
        <DrawnBackdrop />
      )}
      <div className="bd-hex" />
      <div className="bd-shade" />
    </div>
  );
}

function DrawnBackdrop() {
  // Deterministic star field.
  const stars = Array.from({ length: 90 }, (_, i) => {
    const r = (n: number) => ((Math.sin(i * 12.9898 + n * 78.233) * 43758.5453) % 1 + 1) % 1;
    return { x: r(1) * 100, y: r(2) * 70, s: 0.4 + r(3) * 1.4, o: 0.25 + r(4) * 0.6 };
  });
  return (
    <div className="bd-drawn">
      <svg className="bd-stars" viewBox="0 0 100 100" preserveAspectRatio="none">
        {stars.map((s, i) => (
          <circle key={i} cx={s.x} cy={s.y} r={s.s * 0.12} fill="#fff" opacity={s.o} />
        ))}
      </svg>
      <div className="bd-planet" />
      <svg className="bd-nut" viewBox="0 0 200 200">
        <path d="M100 8 180 54v92l-80 46-80-46V54z" fill="none" stroke="currentColor" strokeWidth="6" />
        <path d="M100 38 154 69v62l-54 31-54-31V69z" fill="none" stroke="currentColor" strokeWidth="2" opacity=".6" />
        <circle cx="100" cy="100" r="30" fill="none" stroke="currentColor" strokeWidth="6" />
        <path d="M78 100h44" stroke="currentColor" strokeWidth="10" strokeLinecap="round" />
      </svg>
    </div>
  );
}
