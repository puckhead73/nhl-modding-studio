// D8 — replace a texture member **inside a packed archive** through the copied-target route: pick
// an image, the server plans the member rewrite (same container, stored form kept, placement
// predicted, siblings of a deduplicated block named), Apply holds it as a pending replacement that
// the next copied-target save stages into the archive (a private copy for shared blocks) — nothing is
// written to the install by this dialog. PS4 titles read their logos from `databigs/inxp_10.big`,
// never loose, so this is the route that reaches the game there.
//
// Ownership (OVERNIGHT-DEV-R1 F1): every async step — the file picker, the preview and the apply —
// is bound to the project / install root it started in and to the pick generation (the member
// selected at that moment). A completion for an earlier generation or another context never
// repaints: a late picker result opens nothing, a late preview is dropped, a late apply neither
// shows a result nor fires the caller's callback. The apply's summary is adopted into the global
// project state only while the project it belongs to is still the open one. Hiding the dialog
// does not cancel an apply that was already dispatched: the button is disabled while it runs,
// and the completion still lands (in-context) as the pending result.

import { useEffect, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { Button, ErrorBanner, Pill, Spinner } from "./primitives";
import { formatBytes } from "./TextureReplace";
import type { AssetMemberApplyDto } from "../bindings/AssetMemberApplyDto";
import type { AssetMemberPlanDto } from "../bindings/AssetMemberPlanDto";
import type { AssetMemberPreviewDto } from "../bindings/AssetMemberPreviewDto";

const CONTAINER_LABEL: Record<string, string> = { gnf: "PS4 GNF", portrait: "BIGF package", texlib: "RW4 texlib", png: "PNG" };

type Picked = { archive: string; memberPath: string; name: string; sourcePath: string; wholeGroup: boolean };
type Preview =
  | { status: "planning" }
  | { status: "ready"; plan: AssetMemberPlanDto }
  | { status: "refused"; kind: string; reason: string }
  | { status: "error"; message: string };

/** `pick(archive, memberPath, name)` opens the image picker and previews the member rewrite;
 *  `dialog` / `result` are ready-to-render nodes. `onApplied` fires after a plan became pending —
 *  only for the project / install root the apply started in, and only while this hook is live. */
export function useArchiveMemberReplace(projectId: number, installRoot: string | null, onApplied?: () => void) {
  const qc = useQueryClient();
  const { update } = useProject();
  const [picked, setPicked] = useState<Picked | null>(null);
  const [preview, setPreview] = useState<Preview | null>(null);
  const [applying, setApplying] = useState(false);
  const [applyError, setApplyError] = useState<string | null>(null);
  const [applied, setApplied] = useState<AssetMemberPlanDto | null>(null);

  // The context shown *now*, read after every await. `genRef` is the pick generation: a new pick,
  // a cancel, a context change or unmount all retire whatever was in flight.
  const ctxRef = useRef({ projectId, installRoot });
  ctxRef.current = { projectId, installRoot };
  const onAppliedRef = useRef(onApplied);
  onAppliedRef.current = onApplied;
  const genRef = useRef(0);
  const applyingRef = useRef(false);
  const mountedRef = useRef(true);
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      genRef.current += 1;
    };
  }, []);
  // A new project or install root: nothing held for the old one is shown again.
  useEffect(() => {
    genRef.current += 1;
    setPicked(null);
    setPreview(null);
    setApplyError(null);
    setApplied(null);
  }, [projectId, installRoot]);

  const sameContext = (forProject: number, forRoot: string | null) => mountedRef.current && ctxRef.current.projectId === forProject && ctxRef.current.installRoot === forRoot;
  const live = (gen: number, forProject: number, forRoot: string | null) => gen === genRef.current && sameContext(forProject, forRoot);

  /** Preview `p` in its context under generation `gen`; the result is kept only while live. */
  const previewPick = async (p: Picked, gen: number, forProject: number, forRoot: string) => {
    setPreview({ status: "planning" });
    try {
      const r: AssetMemberPreviewDto = await call("asset_member_preview", {
        projectId: forProject,
        installRoot: forRoot,
        archiveRelative: p.archive,
        memberPath: p.memberPath,
        sourceImagePath: p.sourcePath,
        wholeGroup: p.wholeGroup,
      });
      if (!live(gen, forProject, forRoot)) return; // stale: another pick, a cancel or another context
      setPreview(r.plan ? { status: "ready", plan: r.plan } : { status: "refused", kind: r.refusal_kind ?? "refused", reason: r.refusal ?? "" });
    } catch (e) {
      if (live(gen, forProject, forRoot)) setPreview({ status: "error", message: String(e) });
    }
  };

  const pick = async (archive: string, memberPath: string, name: string) => {
    if (applyingRef.current) return; // an apply is running: its plan must not be replaced under it
    const forProject = projectId;
    const forRoot = installRoot;
    if (!forRoot) return;
    const gen = ++genRef.current;
    // The picker is modal but asynchronous: the project, root or member may change before it returns.
    const src = await openDialog({
      multiple: false,
      directory: false,
      title: "Choose the edited image to place into this archive member",
      filters: [{ name: "Image", extensions: ["png", "jpg", "jpeg", "bmp", "tga"] }],
    });
    if (typeof src !== "string" || !live(gen, forProject, forRoot)) return;
    const p: Picked = { archive, memberPath, name, sourcePath: src, wholeGroup: false };
    setApplied(null);
    setApplyError(null);
    setPicked(p);
    await previewPick(p, gen, forProject, forRoot);
  };

  /** C7: switch the scope of a shared member (this member only / whole group). The held plan is
   *  retired (a new generation) and a new preview is made with the scope stated explicitly. */
  const setScope = async (wholeGroup: boolean) => {
    if (applyingRef.current || !picked || picked.wholeGroup === wholeGroup) return;
    const forProject = projectId;
    const forRoot = installRoot;
    if (!forRoot) return;
    const gen = ++genRef.current;
    const p: Picked = { ...picked, wholeGroup };
    setApplyError(null);
    setPicked(p);
    await previewPick(p, gen, forProject, forRoot);
  };

  /** Apply the plan shown now. Sends the ticket only; a second click while one runs is ignored. */
  const apply = async () => {
    if (applyingRef.current || !picked || preview?.status !== "ready") return;
    const forProject = projectId;
    const forRoot = installRoot;
    const gen = genRef.current;
    const ticket = preview.plan.ticket;
    applyingRef.current = true;
    setApplying(true);
    setApplyError(null);
    try {
      const r: AssetMemberApplyDto = await call("asset_member_apply_review", { projectId: forProject, ticket });
      // The server holds the replacement now. The project state is that project's: adopt it only
      // while it is still the one open; queries are refreshed either way.
      void qc.invalidateQueries();
      if (mountedRef.current && ctxRef.current.projectId === forProject) update(r.summary);
      if (!live(gen, forProject, forRoot)) return;
      setApplied(r.plan);
      setPicked(null);
      setPreview(null);
      onAppliedRef.current?.();
    } catch (e) {
      if (live(gen, forProject, forRoot)) {
        // A refused review is gone server-side (foreign, consumed or stale): the plan leaves the
        // dialog and Apply stays disabled until a new pick previews again.
        setApplyError(String(e));
        setPreview(null);
      }
    } finally {
      applyingRef.current = false;
      if (mountedRef.current) setApplying(false);
    }
  };

  const cancel = () => {
    if (applyingRef.current) return; // the apply was dispatched; it completes on the server
    genRef.current += 1;
    setPicked(null);
    setPreview(null);
    setApplyError(null);
  };

  const plan = preview?.status === "ready" ? preview.plan : null;
  const dialog = picked ? (
    <div className="fixed inset-0 z-50 grid place-items-center bg-black/60 p-4" onClick={cancel}>
      <div className="w-full max-w-2xl rounded-lg border border-slate-700 bg-slate-900 shadow-xl" onClick={(e) => e.stopPropagation()}>
        <header className="border-b border-slate-800 px-4 py-3">
          <h2 className="text-sm font-semibold text-slate-100">Replace archive member</h2>
          <p className="mt-0.5 truncate text-xs text-slate-400" title={picked.memberPath}>
            {picked.name} · {picked.archive} · {picked.memberPath}
          </p>
        </header>
        <div className="space-y-3 px-4 py-4 text-sm" data-testid="archive-member-dialog" data-ticket={plan?.ticket ?? ""}>
          {preview?.status === "planning" ? (
            <Spinner label="Planning the member rewrite…" />
          ) : preview?.status === "error" ? (
            <ErrorBanner title="Preview failed" message={preview.message} />
          ) : preview?.status === "refused" ? (
            <ErrorBanner title={`Refused (${preview.kind})`} message={preview.reason} />
          ) : plan ? (
            <>
              <p className="text-slate-200">
                {CONTAINER_LABEL[plan.container] ?? plan.container} {plan.width}×{plan.height} · stored as {plan.stored_form} ·{" "}
                {formatBytes(Number(plan.source_stored_len))} → {formatBytes(Number(plan.new_stored_len))}
              </p>
              <p className="text-xs text-slate-400">{plan.placement_prediction}</p>
              {plan.shared_with.length > 0 && (
                <div className="space-y-1 text-xs text-amber-300/90" data-testid="archive-member-scope" data-scope={plan.scope}>
                  <p>
                    Shared block — {plan.shared_with.length + 1} paths share these bytes ({plan.member_path}, {plan.shared_with.join(", ")}). Choose the scope:
                  </p>
                  <label className="flex items-start gap-2">
                    <input type="radio" name="archive-member-scope" checked={!picked.wholeGroup} onChange={() => void setScope(false)} disabled={applying} data-testid="archive-member-scope-this" />
                    <span>This member only — a private copy is appended and only this path is re-pointed; the others keep the shipped image.</span>
                  </label>
                  <label className="flex items-start gap-2">
                    <input type="radio" name="archive-member-scope" checked={picked.wholeGroup} onChange={() => void setScope(true)} disabled={applying} data-testid="archive-member-scope-group" />
                    <span>Whole group — every one of the {plan.shared_with.length + 1} paths shows the new image (rewritten in place when it fits the shared block, else appended once with every path re-pointed).</span>
                  </label>
                </div>
              )}
              <p className="text-xs text-slate-500">
                Apply holds the rewrite as a pending change; the next copied-target save stages the archive (with restore).
                Nothing is written to the install now.
              </p>
            </>
          ) : null}
          {applying && (
            <p className="text-xs text-slate-400" data-testid="archive-member-applying">
              Applying… the request has been sent to the project and completes there; closing this dialog would not cancel it.
            </p>
          )}
          {applyError && <ErrorBanner title="Apply failed — pick the image again to preview a new plan" message={applyError} />}
        </div>
        <footer className="flex justify-end gap-2 border-t border-slate-800 px-4 py-3">
          <Button variant="secondary" onClick={cancel} disabled={applying} data-testid="archive-member-cancel">
            Cancel
          </Button>
          <Button onClick={() => void apply()} disabled={!plan || applying} data-testid="archive-member-apply">
            Apply
          </Button>
        </footer>
      </div>
    </div>
  ) : null;

  const result = applied ? (
    <div className="rounded-md border border-sky-800/60 bg-sky-950/30 p-3 text-sm" data-testid="archive-member-result">
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-sky-200">
            <Pill tone="ok">pending</Pill> {applied.member_path} in {applied.archive_relative} — written by the next copied-target save
          </p>
          <p className="mt-0.5 text-xs text-slate-400">{applied.placement_prediction}</p>
        </div>
        <button onClick={() => setApplied(null)} className="shrink-0 text-xs text-slate-400 hover:text-slate-200" type="button">
          dismiss
        </button>
      </div>
    </div>
  ) : null;

  return { pick, dialog, result };
}
