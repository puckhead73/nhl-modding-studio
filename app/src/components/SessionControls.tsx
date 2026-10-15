import { useCallback, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useNavigate } from "react-router-dom";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useProject } from "../lib/project";
import { useSessionSave } from "../lib/sessionSave";
import { useAcknowledgedClose } from "../lib/closeProject";
import { Button } from "./primitives";
import { ProjectOverviewLink } from "./ProjectOverviewLink";
import { SAVE_EVENT, type SaveNotice } from "../lib/saveNotice";
import { registerSessionClose } from "./WindowCloseGuard";
import { useProjectHistory } from "../lib/useProjectHistory";

export function SessionControls({ historyTarget }: { historyTarget?: HTMLElement | null }) {
  const { project, info, close } = useProject();
  const [lastSave, setLastSave] = useState<string | undefined>(info?.last_saved);
  const [notice, setNotice] = useState(false);
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const saved = (event: Event) => {
      const detail = (event as CustomEvent<SaveNotice>).detail;
      if (detail.projectId !== project?.project_id) return;
      setLastSave(detail.at); setNotice(true);
      clearTimeout(timer);
      timer = setTimeout(() => setNotice(false), 3000);
    };
    window.addEventListener(SAVE_EVENT, saved);
    return () => { window.removeEventListener(SAVE_EVENT, saved); clearTimeout(timer); };
  }, [project?.project_id]);
  const savedAt = lastSave ?? info?.last_saved;
  const savedDate = savedAt ? new Date(savedAt) : null;
  const savedLabel = savedDate && !Number.isNaN(savedDate.getTime())
    ? savedDate.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" }) : "Unsaved";
  const navigate = useNavigate();
  const [target, setTarget] = useState<"project" | "window" | null>(null);
  const targetRef = useRef(target); targetRef.current = target;
  const [historyBusy, setHistoryBusy] = useState(false);
  const save = useSessionSave(target !== null || historyBusy);
  const onClosed = useCallback(() => {
    const quit = targetRef.current === "window";
    close(); navigate("/", { replace: true });
    if (quit) void getCurrentWindow().destroy().catch(console.error);
  }, [close, navigate]);
  const closing = useAcknowledgedClose(project!.project_id, onClosed);
  const history = useProjectHistory(target !== null || save.isPending || closing.pending);
  useEffect(() => setHistoryBusy(history.pending), [history.pending]);
  const busy = save.isPending || closing.pending || history.pending;
  const [confirming, setConfirming] = useState(false);
  const inFlight = useRef(false);
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    if (!project) return;
    return registerSessionClose(project.project_id, () => setTarget("window"));
  }, [project?.project_id]);
  useEffect(() => {
    if (target) dialog.current?.showModal();
    else dialog.current?.close();
  }, [target]);
  async function finish(shouldSave: boolean) {
    if (inFlight.current || busy) return;
    inFlight.current = true; setConfirming(true);
    try {
      if (shouldSave) await save.save();
      await closing.requestClose();
    } catch { /* Save error remains visible; the project and window stay open. */ }
    finally { inFlight.current = false; setConfirming(false); }
  }
  const error = (save.error ? String(save.error) : closing.error) || history.error;
  const historyButtons = <>
    <Button variant="secondary" onClick={() => void history.run("undo")} disabled={busy || confirming || !project?.undo_depth} title="Undo last edit (Ctrl+Z)">Undo</Button>
    <Button variant="secondary" onClick={() => void history.run("redo")} disabled={busy || confirming || !project?.redo_depth} title="Redo last edit (Ctrl+Y or Ctrl+Shift+Z)">Redo</Button>
  </>;
  return <>
    {historyTarget ? createPortal(historyButtons, historyTarget) : historyTarget === undefined ? historyButtons : null}
    <div className="relative grid grid-cols-[auto_auto] grid-rows-[28px_28px] items-center gap-x-2 gap-y-1">
      <Button className="col-start-1 row-start-1 h-7 !px-2 !py-0 !text-xs" onClick={() => void save.save().catch(() => {})} disabled={busy || confirming} title="Save this session to the project folder">{save.isPending ? "Saving…" : "Save Session"}</Button>
      <span className="col-start-1 row-start-2 flex h-7 items-center justify-center whitespace-nowrap rounded-full border border-slate-700 bg-slate-900 px-2 text-[11px] text-slate-300">Last Save: {savedLabel}</span>
      <div className="col-start-2 row-start-1 [&>a]:h-7 [&>a]:justify-center [&>a]:whitespace-nowrap [&>a]:px-2 [&>a]:py-0"><ProjectOverviewLink /></div>
      <Button className="col-start-2 row-start-2 h-7 !px-2 !py-0 !text-xs" variant="secondary" onClick={() => setTarget("project")} disabled={busy || confirming}>Close</Button>
      {error && !target && <p role="alert" className="absolute right-0 top-full z-30 mt-2 w-96 rounded border border-rose-800 bg-slate-950 p-3 text-xs text-rose-200">{error}</p>}
    </div>
    {notice && createPortal(<div role="status" className="fixed left-1/2 top-4 z-[100] -translate-x-1/2 rounded-lg border border-emerald-400 bg-emerald-900 px-6 py-3 text-sm font-medium text-emerald-50 shadow-xl">Project has been saved</div>, document.body)}
    {createPortal(<dialog ref={dialog} aria-labelledby="close-session-title" onCancel={event => {
      event.preventDefault(); if (!busy && !confirming) setTarget(null);
    }} className="fixed inset-0 m-auto w-[min(28rem,calc(100vw-2rem))] rounded-xl border border-slate-700 bg-slate-900 p-6 text-slate-100 backdrop:bg-black/60">
      <h2 id="close-session-title" className="text-lg font-semibold">Close project session?</h2>
      <p className="my-4 text-sm text-slate-300">{project?.dirty ? "Your session has unsaved changes. Save them before closing?" : "Your saved project will be available when you reopen Studio."}</p>
      {error && <p role="alert" className="mb-4 text-sm text-rose-300">{error}</p>}
      <div className="flex flex-wrap justify-end gap-2">
        <Button variant="secondary" disabled={busy || confirming} onClick={() => setTarget(null)}>Cancel</Button>
        <Button variant="secondary" disabled={busy || confirming} onClick={() => void finish(false)}>{project?.dirty ? "Close Without Saving" : "Close Session"}</Button>
        <Button disabled={busy || confirming} onClick={() => void finish(true)}>{busy || confirming ? "Please wait…" : "Save and Close"}</Button>
      </div>
    </dialog>, document.body)}
  </>;
}
