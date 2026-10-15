import { useEffect, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { call } from "./client";
import { awaitFieldWrites } from "./fieldWrites";
import { useProject } from "./project";

export function useProjectHistory(blocked: boolean) {
  const { project, update } = useProject();
  const qc = useQueryClient();
  const flight = useRef(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  async function run(command: "undo" | "redo") {
    if (!project || blocked || flight.current) return;
    flight.current = true; setPending(true); setError(null);
    const projectId = project.project_id;
    try {
      // The last edit may still be queued even when the header depth is stale.
      await awaitFieldWrites(projectId);
      await call(command, { projectId });
      update(await call("project_summary", { projectId }));
      await qc.invalidateQueries({ predicate: q => q.queryKey[1] === projectId });
    } catch (e) { setError(String(e)); }
    finally { flight.current = false; setPending(false); }
  }
  useEffect(() => {
    function keydown(event: KeyboardEvent) {
      if (event.defaultPrevented || event.isComposing || event.altKey || !(event.ctrlKey || event.metaKey)) return;
      const key = event.key.toLowerCase();
      const command = key === "z" ? (event.shiftKey ? "redo" : "undo") : key === "y" && !event.shiftKey ? "redo" : null;
      if (!command) return;
      // Preserve native text undo while typing; project history applies after committing.
      const element = event.target instanceof Element ? event.target : null;
      if (element?.closest("input, textarea, [contenteditable]:not([contenteditable='false'])") || document.querySelector("dialog[open], [role='dialog'][aria-modal='true']")) return;
      event.preventDefault();
      if (!event.repeat) void run(command);
    }
    window.addEventListener("keydown", keydown);
    return () => window.removeEventListener("keydown", keydown);
  });
  return { run, pending, error };
}
