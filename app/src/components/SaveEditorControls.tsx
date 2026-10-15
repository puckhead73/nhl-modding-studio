// Save Editor session controls: Save (back to the opened file; the original is copied to the app
// data folder the first time a session overwrites it), Save Copy As, Undo/Redo and Close. The
// Mod Studio equivalents (project save, autosave, Project Overview) do not apply to a single file.
import { useCallback, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useNavigate, NavLink } from "react-router-dom";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { useAcknowledgedClose } from "../lib/closeProject";
import { useProjectHistory } from "../lib/useProjectHistory";
import { registerSessionClose } from "./WindowCloseGuard";
import { Button } from "./primitives";

export function SaveEditorControls({ historyTarget }: { historyTarget?: HTMLElement | null }) {
  const { project, saveFile, update, close } = useProject();
  const qc = useQueryClient();
  const navigate = useNavigate();
  const [notice, setNotice] = useState<string | null>(null);
  const [target, setTarget] = useState<"session" | "window" | null>(null);
  const targetRef = useRef(target);
  targetRef.current = target;
  const dialog = useRef<HTMLDialogElement>(null);
  const projectId = project!.project_id;
  const readOnly = saveFile?.read_only ?? false;

  const save = useMutation({
    mutationFn: () => call("save_editor_save", { projectId }),
    onSuccess: async (r) => {
      update(r.summary);
      setNotice(r.backup_path ? `Saved. The original was backed up to ${r.backup_path}` : "Saved.");
      await qc.invalidateQueries();
    },
  });
  const saveCopy = useMutation({
    mutationFn: async () => {
      const out = await saveDialog({ title: "Save a copy of this file", defaultPath: saveFile?.file.file_name });
      if (!out) return null;
      await call("save_project", { projectId, outPath: out });
      return out;
    },
    onSuccess: (out) => { if (out) setNotice(`Copy saved to ${out}. Further saves still go to the opened file.`); },
  });

  const onClosed = useCallback(() => {
    const quit = targetRef.current === "window";
    close();
    navigate("/save-editor", { replace: true });
    if (quit) void getCurrentWindow().destroy().catch(console.error);
  }, [close, navigate]);
  const closing = useAcknowledgedClose(projectId, onClosed);
  const busy = save.isPending || saveCopy.isPending || closing.pending;
  const history = useProjectHistory(busy || target !== null);

  useEffect(() => registerSessionClose(projectId, () => setTarget("window")), [projectId]);
  useEffect(() => {
    if (target) dialog.current?.showModal();
    else dialog.current?.close();
  }, [target]);
  useEffect(() => {
    if (!notice) return;
    const t = setTimeout(() => setNotice(null), 6000);
    return () => clearTimeout(t);
  }, [notice]);

  async function finish(shouldSave: boolean) {
    try {
      if (shouldSave) await save.mutateAsync();
      await closing.requestClose();
    } catch {
      // The error stays visible; the session stays open.
    }
  }

  const error = (save.error ? String(save.error) : null) ?? (saveCopy.error ? String(saveCopy.error) : null) ?? closing.error ?? history.error;
  const historyButtons = <>
    <Button variant="secondary" onClick={() => void history.run("undo")} disabled={busy || !project?.undo_depth} title="Undo last edit (Ctrl+Z)">Undo</Button>
    <Button variant="secondary" onClick={() => void history.run("redo")} disabled={busy || !project?.redo_depth} title="Redo last edit (Ctrl+Y or Ctrl+Shift+Z)">Redo</Button>
  </>;
  return <>
    {historyTarget ? createPortal(historyButtons, historyTarget) : historyTarget === undefined ? historyButtons : null}
    <div className="relative grid grid-cols-[auto_auto] grid-rows-[28px_28px] items-center gap-x-2 gap-y-1">
      <Button className="col-start-1 row-start-1 h-7 !px-2 !py-0 !text-xs" onClick={() => save.mutate()} disabled={busy || readOnly}
        title={readOnly ? "Studio cannot write this save format back" : "Write your changes back to the opened file"}>
        {save.isPending ? "Saving…" : "Save"}
      </Button>
      <Button className="col-start-2 row-start-1 h-7 !px-2 !py-0 !text-xs" variant="secondary" onClick={() => saveCopy.mutate()} disabled={busy}
        title="Write the current state to another file, leaving the opened file as it is">
        Save Copy As…
      </Button>
      <NavLink to="database/pending" className="col-start-1 row-start-2 flex h-7 items-center justify-center whitespace-nowrap rounded-full border border-slate-700 bg-slate-900 px-2 text-[11px] text-slate-300 hover:text-sky-200">
        {project?.dirty ? "Unsaved changes" : "No unsaved changes"}
      </NavLink>
      <Button className="col-start-2 row-start-2 h-7 !px-2 !py-0 !text-xs" variant="secondary" onClick={() => setTarget("session")} disabled={busy}>Close</Button>
      {error && !target && <p role="alert" className="absolute right-0 top-full z-30 mt-2 w-96 rounded border border-rose-800 bg-slate-950 p-3 text-xs text-rose-200">{error}</p>}
    </div>
    {notice && createPortal(<div role="status" className="fixed left-1/2 top-4 z-[100] max-w-[90vw] -translate-x-1/2 break-all rounded-lg border border-emerald-400 bg-emerald-900 px-6 py-3 text-sm font-medium text-emerald-50 shadow-xl">{notice}</div>, document.body)}
    {createPortal(<dialog ref={dialog} aria-labelledby="close-save-title" onCancel={(e) => { e.preventDefault(); if (!busy) setTarget(null); }}
      className="fixed inset-0 m-auto w-[min(28rem,calc(100vw-2rem))] rounded-xl border border-slate-700 bg-slate-900 p-6 text-slate-100 backdrop:bg-black/60">
      <h2 id="close-save-title" className="text-lg font-semibold">Close this file?</h2>
      <p className="my-4 text-sm text-slate-300">{project?.dirty ? "You have unsaved changes. Save them to the file before closing?" : "Nothing is left unsaved."}</p>
      {error && <p role="alert" className="mb-4 text-sm text-rose-300">{error}</p>}
      <div className="flex flex-wrap justify-end gap-2">
        <Button variant="secondary" disabled={busy} onClick={() => setTarget(null)}>Cancel</Button>
        <Button variant="secondary" disabled={busy} onClick={() => void finish(false)}>{project?.dirty ? "Close Without Saving" : "Close"}</Button>
        {project?.dirty && !readOnly && <Button disabled={busy} onClick={() => void finish(true)}>{busy ? "Please wait…" : "Save and Close"}</Button>}
      </div>
    </dialog>, document.body)}
  </>;
}
