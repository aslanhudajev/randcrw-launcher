# randcrw launcher

**randcrw** stands for **Ratchet & Clank: ReWrite**, a native Rust rewrite of the PlayStation 2
game. This is its desktop launcher. It works like the OpenGOAL launcher: pick a game version,
install the game data once from your own disc image, then play.

Tauri 2 + React 19 + TypeScript + Vite. The Rust backend does all file, process, dialog and
network work; the page is a thin view.

The launcher ↔ game contract is in [`docs/contract.md`](docs/contract.md).

## Commands

```sh
npm install

npm run dev          # the UI in a normal browser at http://localhost:1420 with a mocked backend
npm run tauri dev    # the real app (Rust backend, native window)
npm run build        # typecheck + production frontend build
npm run tauri build  # release bundle

cd src-tauri && cargo test     # contract parsing, paths, settings, versions, extractor jobs
cd src-tauri && cargo clippy
```

### Browser mock

`npm run dev` in a browser uses `src/backend/mock.ts` instead of Tauri. URL parameters pick a
state, which is handy for screenshots:

| URL | State |
|---|---|
| `/` | RAC1 not installed |
| `/?mock=extracting` | extraction running |
| `/?mock=error21` (or `error20`) | extraction failed with that code |
| `/?mock=installed` | ready to play |
| `/?mock=verifying`, `stale`, `no-version`, `playing` | other states |
| `/?iso=Ratchet%20%26%20Clank%20(Europe).iso` | what the fake file dialog returns |
| `#/game/rac2`, `#/settings/folders`, `#/settings/versions/development`, `#/help` | screens |

### Mock game build

`dev/mock/` is a complete fake version folder: `randcrw-manifest.json`, a mock extractor
(`randcrw-extract`) and a mock runtime (`randcrw`), both small Node scripts that speak the
contract. In the real app, add `dev/mock` under Settings → Version Management → Development, set
it active, and install from any `.iso` file:

- `dev/mock/make-test-isos.sh <dir>` creates empty test images. The file name steers the
  outcome: `(USA)` succeeds, `Europe`/`PAL` gives error 21, `notrac` error 20, `errNN` any code.
- `RANDCRW_MOCK_ERROR=<code>` forces a code, `RANDCRW_MOCK_SECONDS` sets the copy time,
  `RANDCRW_MOCK_VERIFY_FAIL=1` (or a `.mock-corrupt` file in the data dir) fails verification.
- The mock runtime answers `--version-json` and "plays" for `RANDCRW_MOCK_RUN_SECONDS` (5).

The scripts need Node on `PATH` (macOS and Linux).

To try the **real** extractor from the game repo, `dev/make-real-extractor-version.sh` makes
`dev/real/` (git-ignored): the real `randcrw-extract` plus the mock runtime. Add that folder under
Development instead.

`RANDCRW_DATA_ROOT=/some/dir npm run tauri dev` keeps a test run away from your real data root.

## Structure

```
src-tauri/src/
  contract.rs     manifest, extractor JSON lines, error codes, extract-info, --version-json
  paths.rs        per-OS data root, layout, location pointer, moving the root
  settings.rs     <root>/settings/launcher.json
  versions.rs     version sources (official, development), resolve + validate
  github.rs       GitHub releases client for the Official source (off by default)
  extractor.rs    runs randcrw-extract as a cancellable job, staging + promote
  launch.rs       starts the runtime with --data-dir / RC_DATA_DIR, logs output
  commands.rs     the Tauri commands the page calls
src/
  backend/        contract.ts (types), api.ts (interface), tauri.ts, mock.ts
  components/     title bar, sidebar, backdrop, modal, menu, progress bar, icons
  screens/        GameScreen (RAC1), ComingLater (RAC2/3), settings/, Help
  lib/            games, version sources, error messages, formatting, hash router
  styles/         fonts.css, theme.css (tokens + primitives), app.css (layout)
  assets/games/   logos and optional backgrounds (<id>-bg.* is picked up if present)
  assets/fonts/   bundled OFL fonts, see LICENSES.md
dev/mock/         mock version folder: manifest, extractor, runtime
docs/contract.md  the launcher ↔ game contract
assets-src/icon/  app icon: gen.py writes app-icon.svg (and app-icon-small.svg for 16/32 px)
```

### Adding a version source (mods)

Sources are listed in one place on each side: `SourceId` + `SourceId::location` in
`src-tauri/src/versions.rs`, and `SOURCES` in `src/lib/sources.ts`. A Mods source is a new
variant (installed under `versions/mods/<name>/`, like official builds), its feed, and a list
component in `screens/settings/Versions.tsx`.

## Art and fonts

- Game logos and backgrounds in `src/assets/games/` are user-provided launcher art. A missing
  `<id>-bg.*` falls back to a drawn backdrop.
- Fonts: Russo One and Exo 2, both SIL OFL 1.1, bundled locally
  (`src/assets/fonts/LICENSES.md`).

randcrw is an unofficial fan project, not affiliated with Sony Interactive Entertainment or
Insomniac Games. It ships no game data.
