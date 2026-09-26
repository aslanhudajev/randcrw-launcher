# Launcher ↔ game contract v0

The randcrw launcher (this repo) and the randcrw game repo both implement exactly this. Types:
`src-tauri/src/contract.rs` (Rust) and `src/backend/contract.ts` (TypeScript). Change all three
together.

## Version

A **version** is one game build, official or (later) a mod. It is a folder
`versions/<source>/<version>/` that contains `randcrw-manifest.json`:

```json
{
  "schema": 1,
  "name": "randcrw",
  "version": "0.1.0",
  "game": "rac1",
  "runtime": "<relative path to game binary>",
  "extractor": "<relative path to extractor binary>",
  "supported_discs": ["SCUS_971.99"],
  "data_format": 1
}
```

- `runtime` and `extractor` are relative to the version folder and may not leave it (no `..`,
  no absolute paths). The launcher rejects the manifest otherwise.
- The launcher accepts `schema` 1 only and requires `name` = `randcrw`.
- The Development source points at a local folder with this manifest (for example a dev build
  folder). It is used in place; nothing is copied.

## Extractor CLI (`randcrw-extract`)

```
randcrw-extract identify --iso <path> --json
randcrw-extract extract  --iso <path> --out <game_data_dir> [--ntsc-only] --json
randcrw-extract verify   --out <game_data_dir> --json
```

- `identify` identifies the disc only.
- `extract` does the full extraction.
- `verify` re-hashes against the built-in size/SHA-1 table.

With `--json`, stdout is JSON lines, one object per line:

| `type` | Fields |
|---|---|
| `progress` | `stage`: `identify` \| `copy` \| `verify`; `done`, `total` (bytes); `file` (path) |
| `info` | `message` |
| `disc` | `serial` (e.g. `SCUS_971.99`), `region` (e.g. `NTSC-U`), `version` (e.g. `1.00`), `supported` (bool) |
| `error` | `code`, `message` |
| `done` | `elapsed_ms` |

The process exit code equals the error code:

| Code | Meaning | Launcher message |
|---|---|---|
| 0 | ok | |
| 10 | cannot open/read file | That file couldn't be read. |
| 11 | not an ISO9660 image | That file isn't a disc image. |
| 20 | not Ratchet & Clank (unknown disc) | This doesn't look like Ratchet & Clank. |
| 21 | Ratchet & Clank, unsupported build/region (only SCUS_971.99 v1.00 for now) | This copy isn't supported yet — only the US release (SCUS-97199) for now. |
| 30 | write failure | The game data couldn't be written. |
| 31 | not enough disk space | There isn't enough disk space. |
| 40 | verification failed | Some game files don't match. |
| 99 | internal error | Something went wrong inside the extractor. |

On success the extractor writes `<game_data_dir>/extract-info.json`:

```json
{"disc":"SCUS_971.99","data_format":1,"extractor_version":"...","ntsc_only":false,"files":0,"bytes":0}
```

## Runtime

- `<runtime> --version-json` prints `{"name":"randcrw","version":"...","game":"rac1","data_format":1}`
  and exits.
- The launcher starts the game with `<runtime> --data-dir <game_data_dir>`. The env var
  `RC_DATA_DIR` is equivalent; the launcher sets both.
- Game side (clarification): `--version-json` answers in milliseconds without opening a window.
  `--data-dir <dir>` / `--data-dir=<dir>` wins over `RC_DATA_DIR`. The folder must hold `toc.bin`
  and an `extract-info.json` with the build's `data_format`.
- When the runtime cannot start, it prints one `error: ...` line on stderr and exits before any
  window opens:

  | Exit | Meaning | Launcher |
  |---|---|---|
  | 2 | `--data-dir` given without a value | "randcrw couldn't start." (a launcher/game mismatch) |
  | 3 | data folder missing, not a data folder, or extraction incomplete | "The game data is missing or incomplete." + Re-extract |
  | 4 | `data_format` mismatch, or the info file can't be read | "The game data doesn't match this randcrw version." + Re-extract |

  The launcher shows the `error:` line under Details. Any other non-zero exit is reported as a
  crash with a pointer to the log.
- The packaged runtime binary is named `randcrw`; its path always comes from the manifest.

## Folders

Per-OS app data root named `randcrw`:

| OS | Default |
|---|---|
| macOS | `~/Library/Application Support/randcrw/` |
| Windows | `%LOCALAPPDATA%\randcrw\` |
| Linux | `$XDG_DATA_HOME/randcrw/` (default `~/.local/share/randcrw/`) |

Under it:

```
versions/            game builds, by source (versions/official/<tag>/, later versions/mods/...)
games/rac1/data/     the extracted game data (the runtime's --data-dir)
logs/
settings/            settings/launcher.json holds the launcher settings
```

The user can move the data root. Never use the name "randcre" anywhere user-facing.

## Clarifications (game side)

The canonical copy of the contract and of these clarifications lives in the game repo at
`docs/plan/launcher_contract.md`. None of them changes a field, a command or a code.

1. Every run ends with exactly one `done` line (exit 0) or one `error` line (exit = its code).
   The exit code is authoritative if the stream is cut short.
2. Objects may carry extra fields; the launcher ignores unknown ones. The `disc` line has
   `game` (`rac1`, `rac2`, `rac3`, `racdl` or `unknown`), `title` and `elf_sha1`.
3. The `disc` line appears whenever the boot ELF was read: on success, for code 21, and for code
   20 when the image is another PS2 game. Never for 10 or 11.
4. The sequels (R&C 2, 3, Deadlocked) return **21** with `game` set, not 20. The launcher says
   e.g. "That's Ratchet & Clank 2 — not supported yet."
5. A truncated image returns 10; a raw 2352-byte `.bin` returns 11. Usage errors return 99 with a
   message starting `usage:`.
6. `extract` reports the stages `identify` then `copy`; every file is hashed while it is copied
   and a mismatch returns 40. `verify` reports `verify`. Progress is rate-limited to about 10
   lines a second and each stage ends with `done == total`.
7. Cancel = kill. Files are written as `.partial` and renamed when complete;
   `extract-info.json` is deleted at start and written last, so a folder without it is
   incomplete and is never launched.
8. `--iso` falls back to `RC_ISO`; `--threads N`, `--help` and `--version` exist.
9. A full extraction of about 4 GiB takes 3–8 s, `verify` about 2 s.

## Launcher-side details (not part of the game's obligations)

These are how the launcher uses the contract. The game side does not need to do anything for them.

- **Staging.** The launcher runs `extract` with `--out games/<game>/data.staging/`, and renames
  it over `games/<game>/data/` only after exit code 0 and a readable `extract-info.json`. A failed
  or cancelled run deletes the staging folder and leaves an existing install untouched.
- **Cancel** kills the extractor process.
- **Throttling.** The launcher forwards at most about 12 progress events a second per stage to
  the page, plus every stage change and each stage's final line.
- **Outcome.** The exit code decides success or failure. The message shown comes from the last
  `error` line with the same code, else the last stderr line. Lines on stdout that are not
  contract objects are treated as `info`.
- **Moved data root.** When the user moves the data root, `location.json`
  (`{"data_root": "<absolute path>"}`) is written into the default root so the launcher finds it
  again. `RANDCRW_DATA_ROOT` overrides the data root for development.
- **Stale data.** If the installed `extract-info.json` has a different `data_format` than the
  active version's manifest, the launcher asks for a re-extract instead of starting the game.
- **Validation.** A version is usable when its manifest parses, both binaries exist, and
  `<runtime> --version-json` answers within 10 s with `name` = `randcrw` and the same `game` and
  `data_format` as the manifest.
- **Logs.** `logs/extract-<game>-<unix>.log`, `logs/verify-<game>-<unix>.log` and
  `logs/<game>-<unix>.log` (game stdout and stderr).
