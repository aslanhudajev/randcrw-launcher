import { Sidebar } from "./components/Sidebar";
import { TitleBar } from "./components/TitleBar";
import { Toasts } from "./components/Toasts";
import { gameById, GAMES } from "./lib/games";
import { useRoute } from "./lib/router";
import { ComingLater } from "./screens/ComingLater";
import { GameScreen } from "./screens/GameScreen";
import { Help } from "./screens/Help";
import { Settings } from "./screens/settings/Settings";

export default function App() {
  const [route, go] = useRoute();
  const [section, sub] = route;

  let main;
  if (section === "settings") main = <Settings route={route} go={go} />;
  else if (section === "help") main = <Help />;
  else {
    const game = gameById(sub ?? "") ?? GAMES[0];
    main = game.supported ? <GameScreen key={game.id} game={game} go={go} /> : <ComingLater key={game.id} game={game} />;
  }

  return (
    <div className="app">
      <TitleBar />
      <div className="app-body">
        <Sidebar route={section === "game" || !section ? ["game", sub ?? "rac1"] : route} go={go} />
        <main className="main" key={section}>
          {main}
        </main>
      </div>
      <Toasts />
    </div>
  );
}
