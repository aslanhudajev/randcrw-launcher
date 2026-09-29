import logo from "../assets/brand/rerac-logo-480.png";
import { IconFolder, IconLink } from "../components/Icons";
import { useLauncher } from "../state";

// Placeholder links until the website has these pages.
const LINKS = [
  { title: "Getting started", text: "How to dump your disc and install the game data.", url: "https://re-rac.github.io" },
  { title: "Report a problem", text: "Open an issue with your launcher and game logs attached.", url: "https://github.com/re-rac/rerac-launcher/issues" },
  { title: "Compatibility", text: "Which discs and releases are supported.", url: "https://re-rac.github.io" },
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
              <b>Pick a game version.</b> Settings → Version Management → Development: install a build .zip (or add a build folder) and set it active.
            </li>
            <li>
              <b>Install via ISO.</b> Choose a disc image of your own Ratchet & Clank (US, SCUS-97199). It is read once.
            </li>
            <li>
              <b>Play.</b> The launcher starts ReRAC with your extracted data.
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
          <img className="about-logo" src={logo} alt="ReRAC" draggable={false} />
          <p>
            <b>ReRAC</b> is the PlayStation 2 game Ratchet &amp; Clank (2002) rebuilt natively, from scratch, to run on
            today's computers with the data from your own disc. Launcher v{snapshot?.launcher_version ?? "…"}.
          </p>
          <p>
            ReRAC is an unofficial fan project, not affiliated with or endorsed by Sony Interactive Entertainment or
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
