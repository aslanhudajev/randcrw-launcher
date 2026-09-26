import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { loadBackend } from "./backend/api";
import { LauncherProvider } from "./state";
import "./styles/fonts.css";
import "./styles/theme.css";
import "./styles/app.css";

void loadBackend().then((backend) => {
  document.documentElement.dataset.backend = backend.mode;
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <LauncherProvider backend={backend}>
        <App />
      </LauncherProvider>
    </React.StrictMode>,
  );
});
