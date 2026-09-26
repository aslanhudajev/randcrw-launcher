import { useEffect, useState } from "react";

// Tiny hash router: #/game/rac1, #/settings/folders, #/settings/versions/official, #/help.
export function useRoute(): [string[], (path: string) => void] {
  const read = () => (location.hash.replace(/^#\/?/, "") || "game/rac1").split("/").filter(Boolean);
  const [parts, setParts] = useState(read);
  useEffect(() => {
    const on = () => setParts(read());
    window.addEventListener("hashchange", on);
    return () => window.removeEventListener("hashchange", on);
  }, []);
  const go = (path: string) => {
    location.hash = `#/${path}`;
  };
  return [parts, go];
}
