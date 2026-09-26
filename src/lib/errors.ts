// Friendly text for extractor error codes (docs/contract.md). Unknown codes fall back to 99,
// matching ErrorCode::from_code in Rust. The `disc` line, when there was one, sharpens the text.

import type { DiscView } from "../state";
import { prettySerial } from "./format";

export interface FriendlyError {
  title: string;
  hint: string;
  /** Offer choosing another disc image as the main action. */
  pickAnother: boolean;
}

const SEQUELS: Record<string, string> = {
  rac2: "Ratchet & Clank 2",
  rac3: "Ratchet & Clank 3",
  racdl: "Ratchet: Deadlocked",
};

const TABLE: Record<number, FriendlyError> = {
  10: {
    title: "That file couldn't be read.",
    hint: "Check that the disc image still exists, that you can open it, and that the copy isn't cut short.",
    pickAnother: true,
  },
  11: {
    title: "That file isn't a usable disc image.",
    hint: "Only 2048-byte-sector .iso images work for now. Raw .bin dumps, CHD and CSO files aren't supported yet.",
    pickAnother: true,
  },
  20: {
    title: "This doesn't look like Ratchet & Clank.",
    hint: "randcrw needs a disc image of your own copy of Ratchet & Clank for PlayStation 2.",
    pickAnother: true,
  },
  21: {
    title: "This copy isn't supported yet — only the US release (SCUS-97199) for now.",
    hint: "The disc was recognised, but other regions and releases need their own support work.",
    pickAnother: true,
  },
  30: {
    title: "The game data couldn't be written.",
    hint: "Make sure the data folder is writable, or choose another one in Settings → Folders.",
    pickAnother: false,
  },
  31: {
    title: "There isn't enough disk space.",
    hint: "Free up about 4.5 GB, or move the data folder to a bigger drive in Settings → Folders.",
    pickAnother: false,
  },
  40: {
    title: "Some game files don't match.",
    hint: "The data didn't check out against the known-good disc. Re-extract from your disc image; if it happens again, the image itself is damaged.",
    pickAnother: false,
  },
  99: {
    title: "Something went wrong inside the extractor.",
    hint: "Try again. If it keeps happening, the log in the logs folder has the details.",
    pickAnother: false,
  },
};

/** The runtime's own start-up failures (exit 2, 3, 4). */
export function friendlyRuntimeError(code: number): FriendlyError & { reextract: boolean } {
  switch (code) {
    case 3:
      return {
        title: "The game data is missing or incomplete.",
        hint: "randcrw couldn't find a complete extraction in the data folder. Re-extract from your disc image.",
        pickAnother: false,
        reextract: true,
      };
    case 4:
      return {
        title: "The game data doesn't match this randcrw version.",
        hint: "This build expects a different data format. Re-extract from your disc image to update it.",
        pickAnother: false,
        reextract: true,
      };
    default:
      return {
        title: "randcrw couldn't start.",
        hint: "The launcher passed the game an option it didn't accept. Update the launcher and the game, or report it with the log.",
        pickAnother: false,
        reextract: false,
      };
  }
}

export function friendlyError(code: number, disc?: DiscView | null, message?: string): FriendlyError {
  const base = TABLE[code] ?? TABLE[99];
  if (code === 21 && disc?.game && SEQUELS[disc.game]) {
    return {
      title: `That's ${SEQUELS[disc.game]} — not supported yet.`,
      hint: "randcrw rewrites the first Ratchet & Clank for now. The sequels come later.",
      pickAnother: true,
    };
  }
  if (code === 21 && disc) {
    return {
      ...base,
      hint: `Your disc is ${prettySerial(disc.serial)} (${disc.region} v${disc.version}). Other regions and releases need their own support work.`,
    };
  }
  if (code === 20 && disc) {
    const what = disc.title && disc.game === "unknown" ? `${disc.title} (${prettySerial(disc.serial)})` : prettySerial(disc.serial);
    return { ...base, hint: `This disc identifies as ${what}. ${base.hint}` };
  }
  if (code === 99 && message?.startsWith("usage:")) {
    return {
      ...base,
      hint: "The launcher and this randcrw build disagree about the extractor's options. Update both, or report it with the log.",
    };
  }
  return base;
}
