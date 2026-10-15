import { useEffect, useRef, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useProject } from "../lib/project";

let session: { projectId: number; prompt: () => void } | null = null;
export function registerSessionClose(projectId: number, prompt: () => void) {
  const owner = { projectId, prompt };
  session = owner;
  return () => { if (session === owner) session = null; };
}

/** One app-lifetime listener: menu/onboarding closes normally; an active editor owns the prompt.
 * Keeping the listener above the routes avoids orphaned native close interception after navigation. */
export function WindowCloseGuard() {
  const { project } = useProject();
  const active = useRef(project?.project_id);
  active.current = project?.project_id;
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    if (!isTauri()) return;
    let disposed = false;
    let stop: (() => void) | undefined;
    getCurrentWindow().onCloseRequested(event => {
      if (!disposed && session && session.projectId === active.current) {
        event.preventDefault();
        session.prompt();
      }
      // Without preventDefault, Tauri destroys the window, including on the main menu.
    }).then(unlisten => { if (disposed) unlisten(); else stop = unlisten; })
      .catch(e => setError(`The close prompt could not be enabled: ${String(e)}. Use Save Session before exiting.`));
    return () => { disposed = true; stop?.(); };
  }, []);
  return error ? <p role="alert" className="fixed left-1/2 top-4 z-[100] -translate-x-1/2 rounded bg-rose-950 p-3 text-sm text-rose-200">{error}</p> : null;
}
