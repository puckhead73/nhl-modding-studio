export const SAVE_EVENT = "nhlms:project-saved";
export type SaveNotice = { projectId: number; at: string };
/** Emit only after a successful full-project save, never for a failed write. */
export function notifyProjectSaved(projectId: number) {
  window.dispatchEvent(new CustomEvent<SaveNotice>(SAVE_EVENT, { detail: { projectId, at: new Date().toISOString() } }));
}
