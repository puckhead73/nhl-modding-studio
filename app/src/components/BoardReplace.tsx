// Receipt 67 (C37) — replace one PS3 board-ad raster (NHL 11–15 / Legacy) with a same-size image: pick the image, the server
// admits the raster (research handoff 10 §8.1), regenerates every declared DXT1 / DXT3 level and plans the archive member —
// and, on NHL 14 / 15 / Legacy, the loose copy of the same package — as one pending change. Apply holds it; the next
// copied-target save writes it as one transaction. Nothing is written to the install by this dialog, and nothing here says
// how the game draws the board.
//
// Ownership (as ArchiveMemberReplace): every async step is bound to the project / install root and pick generation it started
// in; a late picker result, preview or apply for another context never repaints.

import { useEffect, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { Button, ErrorBanner, Pill, Spinner } from "./primitives";
import type { AssetMemberApplyDto } from "../bindings/AssetMemberApplyDto";
import type { AssetMemberPlanDto } from "../bindings/AssetMemberPlanDto";
import type { BoardReplacementPreviewDto } from "../bindings/BoardReplacementPreviewDto";

type Picked = { pkg: string; descriptor: number; label: string; sourcePath: string };
type Preview =
  | { status: "planning" }
  | { status: "ready"; board: BoardReplacementPreviewDto; plan: AssetMemberPlanDto }
  | { status: "refused"; kind: string; reason: string }
  | { status: "error"; message: string };

export function useBoardReplace(projectId: number, installRoot: string | null) {
  const qc = useQueryClient();
  const { update } = useProject();
  const [picked, setPicked] = useState<Picked | null>(null);
  const [preview, setPreview] = useState<Preview | null>(null);
  const [applying, setApplying] = useState(false);
  const [applyError, setApplyError] = useState<string | null>(null);
  const [applied, setApplied] = useState<AssetMemberPlanDto | null>(null);
  const ctxRef = useRef({ projectId, installRoot });
  ctxRef.current = { projectId, installRoot };
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
  useEffect(() => {
    genRef.current += 1;
    setPicked(null);
    setPreview(null);
    setApplyError(null);
    setApplied(null);
  }, [projectId, installRoot]);
  const live = (gen: number, p: number, r: string | null) => gen === genRef.current && mountedRef.current && ctxRef.current.projectId === p && ctxRef.current.installRoot === r;

  /** Open the picker for `pkg`'s raster `descriptor` and preview the replacement. */
  const pick = async (pkg: string, descriptor: number, label: string) => {
    if (applyingRef.current) return;
    const forProject = projectId;
    const forRoot = installRoot;
    if (!forRoot) return;
    const gen = ++genRef.current;
    const src = await openDialog({
      multiple: false,
      directory: false,
      title: "Choose the board image (the raster's own size)",
      filters: [{ name: "Image", extensions: ["png", "jpg", "jpeg", "bmp", "tga"] }],
    });
    if (typeof src !== "string" || !live(gen, forProject, forRoot)) return;
    setApplied(null);
    setApplyError(null);
    setPicked({ pkg, descriptor, label, sourcePath: src });
    setPreview({ status: "planning" });
    try {
      const r: BoardReplacementPreviewDto = await call("ps3_board_replacement_preview", { projectId: forProject, installRoot: forRoot, package: pkg, descriptor, sourceImagePath: src });
      if (!live(gen, forProject, forRoot)) return;
      setPreview(r.plan ? { status: "ready", board: r, plan: r.plan } : { status: "refused", kind: r.refusal_kind ?? "refused", reason: r.refusal ?? "" });
    } catch (e) {
      if (live(gen, forProject, forRoot)) setPreview({ status: "error", message: String(e) });
    }
  };

  const apply = async () => {
    if (applyingRef.current || preview?.status !== "ready") return;
    const forProject = projectId;
    const forRoot = installRoot;
    const gen = genRef.current;
    applyingRef.current = true;
    setApplying(true);
    setApplyError(null);
    try {
      const r: AssetMemberApplyDto = await call("asset_member_apply_review", { projectId: forProject, ticket: preview.plan.ticket });
      void qc.invalidateQueries();
      if (mountedRef.current && ctxRef.current.projectId === forProject) update(r.summary);
      if (!live(gen, forProject, forRoot)) return;
      setApplied(r.plan);
      setPicked(null);
      setPreview(null);
    } catch (e) {
      if (live(gen, forProject, forRoot)) {
        setApplyError(String(e));
        setPreview(null);
      }
    } finally {
      applyingRef.current = false;
      if (mountedRef.current) setApplying(false);
    }
  };

  const cancel = () => {
    if (applyingRef.current) return;
    genRef.current += 1;
    setPicked(null);
    setPreview(null);
    setApplyError(null);
  };

  const ready = preview?.status === "ready" ? preview : null;
  const dialog = picked ? (
    <div className="fixed inset-0 z-50 grid place-items-center bg-black/60 p-4" onClick={cancel}>
      <div className="w-full max-w-2xl rounded-lg border border-slate-700 bg-slate-900 shadow-xl" onClick={(e) => e.stopPropagation()}>
        <header className="border-b border-slate-800 px-4 py-3">
          <h2 className="text-sm font-semibold text-slate-100">Replace board raster</h2>
          <p className="mt-0.5 truncate text-xs text-slate-400">
            {picked.label} · {picked.pkg} · raster {picked.descriptor}
          </p>
        </header>
        <div className="space-y-2 px-4 py-4 text-sm" data-testid="board-dialog" data-ticket={ready?.plan.ticket ?? ""}>
          {preview?.status === "planning" ? (
            <Spinner label="Checking the raster and planning the change…" />
          ) : preview?.status === "error" ? (
            <ErrorBanner title="Preview failed" message={preview.message} />
          ) : preview?.status === "refused" ? (
            <ErrorBanner title={`Refused (${preview.kind})`} message={preview.reason} />
          ) : ready ? (
            <>
              <p className="text-slate-200" data-testid="board-summary">
                {ready.board.format} {ready.plan.width}×{ready.plan.height} · {ready.board.levels} levels ·{" "}
                {ready.board.topology === "pair" ? "loose package + archive member, written together" : "archive member only"}
              </p>
              <p className="text-xs text-amber-300/90" data-testid="board-materials">
                Used by {ready.board.materials.length > 0 ? ready.board.materials.join(", ") : "no named material"} — all of them change.
              </p>
              <ul className="list-disc space-y-0.5 pl-4 text-xs text-slate-400">
                {ready.plan.notes.map((n) => (
                  <li key={n}>{n}</li>
                ))}
                <li>{ready.plan.placement_prediction}</li>
              </ul>
              <p className="text-xs text-slate-500">Apply holds this as a pending change; the next copied-target save writes it (with restore). Nothing is written now.</p>
            </>
          ) : null}
          {applyError && <ErrorBanner title="Apply failed — pick the image again" message={applyError} />}
        </div>
        <footer className="flex justify-end gap-2 border-t border-slate-800 px-4 py-3">
          <Button variant="secondary" onClick={cancel} disabled={applying} data-testid="board-cancel">
            Cancel
          </Button>
          <Button onClick={() => void apply()} disabled={!ready || applying} data-testid="board-apply">
            Apply
          </Button>
        </footer>
      </div>
    </div>
  ) : null;

  const result = applied ? (
    <p className="rounded-md border border-sky-800/60 bg-sky-950/30 p-2 text-xs text-sky-200" data-testid="board-result">
      <Pill tone="ok">pending</Pill> {applied.description} — written by the next copied-target save
    </p>
  ) : null;

  return { pick, dialog, result };
}
