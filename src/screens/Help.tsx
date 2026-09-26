import { IconFolder, IconLink } from "../components/Icons";
import { useLauncher } from "../state";

// Placeholder links until the project has public pages.
const LINKS = [
  { title: "Getting started", text: "How to dump your disc and install the game data.", url: "https://github.com/randcrw" },
  { title: "Report a problem", text: "Open an issue with your launcher and game logs attached.", url: "https://github.com/randcrw" },
  { title: "Compatibility", text: "Which discs and releases are supported.", url: "https://github.com/randcrw" },
];

export function Help() {
  const { backend, snapshot, run } = useLauncher();
  return (
    <section className="page help">
      <header className="page-head">
        <h1>Help</h1>
      </header>
      <div className="page-body">
        <div className="card plate">
          <h3>Getting going</h3>
          <ol className="steps">
            <li>
              <b>Pick a game version.</b> Settings → Version Management. For now, add a Development build folder.
            </li>
            <li>
              <b>Install via ISO.</b> Choose a disc image of your own Ratchet & Clank (US, SCUS-97199). It is read once.
            </li>
            <li>
              <b>Play.</b> The launcher starts randcrw with your extracted data.
            </li>
          </ol>
        </div>
        <div className="help-grid">
          {LINKS.map((l) => (
            <button key={l.title} className="card plate link-card" onClick={() => void run(() => backend.openUrl(l.url))}>
              <span className="link-title">
                {l.title} <IconLink size={16} />
              </span>
              <span className="muted">{l.text}</span>
              <span className="soon-tag">Placeholder</span>
            </button>
          ))}
          <button className="card plate link-card" onClick={() => void run(() => backend.openFolder("logs"))}>
            <span className="link-title">
              Logs folder <IconFolder size={16} />
            </span>
            <span className="muted">Extractor and game logs, for bug reports.</span>
          </button>
        </div>
        <div className="card plate about">
          <h3>About</h3>
          <p>
            <b>randcrw</b> stands for <b>Ratchet &amp; Clank: ReWrite</b>: the PlayStation 2 game rebuilt natively, from
            scratch, to run on today's computers with the data from your own disc. Launcher v
            {snapshot?.launcher_version ?? "…"}.
          </p>
          <p>
            randcrw is an unofficial fan project, not affiliated with or endorsed by Sony Interactive Entertainment or
            Insomniac Games. It ships no game data: you provide your own disc.
          </p>
          <p className="muted small">
            Fonts: Russo One (Jovanny Lemonad) and Exo 2 (The Exo 2 Project Authors), both under the SIL Open Font
            License 1.1.
          </p>
        </div>
      </div>
    </section>
  );
}
