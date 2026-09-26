import type { GameId } from "../backend/contract";
import rac1Logo from "../assets/games/rac1-logo.webp";
import rac2Logo from "../assets/games/rac2-logo.webp";
import rac3Logo from "../assets/games/rac3-logo.webp";

// Background art is optional: the game repo renders rac1-bg later. Any
// src/assets/games/<id>-bg.{webp,png,jpg,jpeg,avif} is picked up at build time; a missing file
// falls back to the drawn backdrop (components/Backdrop.tsx).
const bgs = import.meta.glob("../assets/games/*-bg.{webp,png,jpg,jpeg,avif}", {
  eager: true,
  import: "default",
}) as Record<string, string>;

function bgFor(id: GameId): string | null {
  const key = Object.keys(bgs).find((k) => k.split("/").pop()!.startsWith(`${id}-bg.`));
  return key ? bgs[key] : null;
}

export interface GameDef {
  id: GameId;
  title: string;
  subtitle: string;
  year: number;
  logo: string;
  bg: string | null;
  /** Hue used by the drawn backdrop. */
  accent: string;
  supported: boolean;
}

export const GAMES: GameDef[] = [
  { id: "rac1", title: "Ratchet & Clank", subtitle: "Ratchet & Clank", year: 2002, logo: rac1Logo, bg: bgFor("rac1"), accent: "#f28c28", supported: true },
  { id: "rac2", title: "Going Commando", subtitle: "Ratchet & Clank 2", year: 2003, logo: rac2Logo, bg: bgFor("rac2"), accent: "#4fb8ff", supported: false },
  { id: "rac3", title: "Up Your Arsenal", subtitle: "Ratchet & Clank 3", year: 2004, logo: rac3Logo, bg: bgFor("rac3"), accent: "#ffcf3a", supported: false },
];

export const gameById = (id: string) => GAMES.find((g) => g.id === id);
