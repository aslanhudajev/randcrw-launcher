import type { SourceId } from "../backend/contract";

// Version sources shown under Settings → Version Management. Mods will be one more entry
// (source id "mods") plus a list component; the Rust side adds a SourceId variant.
export interface SourceDef {
  id: SourceId | "mods";
  label: string;
  blurb: string;
  /** Present but not usable yet. */
  disabledNote?: string;
}

export const SOURCES: SourceDef[] = [
  {
    id: "official",
    label: "Official",
    blurb: "Release builds published by the randcrw project.",
  },
  {
    id: "development",
    label: "Development",
    blurb: "Local builds from a folder containing randcrw-manifest.json.",
  },
  {
    id: "mods",
    label: "Mods",
    blurb: "Game builds from other sources.",
    disabledNote: "Coming later",
  },
];
