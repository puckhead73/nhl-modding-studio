import { useEffect, useRef } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { call } from "./client";
import { useProject } from "./project";
import { useSettings, validAutosaveMinutes } from "./settings";
import { notifyProjectSaved } from "./saveNotice";
import { awaitFieldWrites } from "./fieldWrites";

/** Manual save, close-and-save and autosave share one in-flight write. */
export function useSessionSave(paused = false) {
  const context = useProject();
  const { settings } = useSettings();
  const qc = useQueryClient();
  const flight = useRef<Promise<void> | null>(null);
  const latest = useRef(context);
  latest.current = context;
  const mutation = useMutation({ mutationFn: async () => {
    const { project, info, update, updateInfo } = latest.current;
    if (!project) throw new Error("No project session is open.");
    const projectId = project.project_id;
    await awaitFieldWrites(projectId);
    if (info) {
      const result = await call("project_save", { projectId });
      if (!result.saved && !result.nothing_to_save) throw new Error("The session was not saved. Keep it open and try again.");
      update(result.summary);
      updateInfo(await call("project_info", { projectId }), undefined, projectId);
    } else {
      await call("save_workspace", { projectId });
      update(await call("project_summary", { projectId }));
    }
    notifyProjectSaved(projectId);
    await qc.invalidateQueries();
  } });
  const save = () => {
    if (!flight.current) flight.current = mutation.mutateAsync().finally(() => { flight.current = null; });
    return flight.current;
  };
  const saveRef = useRef(save);
  saveRef.current = save;
  const minutes = validAutosaveMinutes(settings.autosaveMinutes);
  useEffect(() => {
    if (paused || !context.project || !context.info) return;
    // Repeated edits must not postpone the interval indefinitely.
    const timer = setInterval(() => {
      if (latest.current.project?.dirty) void saveRef.current().catch(() => {});
    }, minutes * 60_000);
    return () => clearInterval(timer);
  }, [context.project?.project_id, !!context.info, minutes, paused]);
  return { ...mutation, save };
}
