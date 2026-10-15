// Shared texture-replace UI — the export→edit→reimport loop used by the File Browser and the Team
// Uniforms surface. `useTextureReplace` owns the pick → before/after preview → confirm → result flow;
// the dialog decodes the current on-disk texture beside the picked source, reports both dimensions,
// and only lets the write proceed when the source fits (GNF/texlib are same-size; a portrait archive
// can be resized). Everything routes through the same server-side `import_texture` dispatcher.

import { useEffect, useRef, useState, type CSSProperties, type ReactNode } from "react";
import { useQuery, useQueryClient, type QueryClient } from "@tanstack/react-query";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { call } from "../lib/client";
import { Button, ErrorBanner, Spinner } from "./primitives";
import type { ImportTextureSummaryDto } from "../bindings/ImportTextureSummaryDto";
import type { TexturePreviewDto } from "../bindings/TexturePreviewDto";
import type { TextureReplacePlanDto } from "../bindings/TextureReplacePlanDto";

const CONTAINER_LABEL: Record<string, string> = {
  gnf: "PS4 GNF",
  portrait: "portrait archive",
  texlib: "RW4 texlib",
  png: "PNG",
};

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

type Pending = { installDir: string; relPath: string; name: string; sourcePath: string; textureIndex?: number };

const ASSET_PREVIEW_QUERIES = new Set([
  "texturePreview", "portraitPreview", "portraitHeaderPreview", "teamLogo", "jerseyThumb",
  "headPreview", "headPreviewPaths", "catalogEntryPreview", "kitPreview", "teamKitPreview",
  "equipmentItemPreview", "project-overview", "package_inventory",
]);
function refreshAssetPreviews(client: QueryClient) {
  // Geometry previews also consume these textures; a replaced image must not leave
  // a cached model/header displaying the old installation's art.
  void client.invalidateQueries({ predicate: q => ASSET_PREVIEW_QUERIES.has(String(q.queryKey[0])) });
}

/** Manage a texture replace: `pick(relPath, name, textureIndex?)` opens the image picker and stages a
 *  before/after preview; `dialog` and `result` are ready-to-render nodes. `textureIndex` targets one
 *  raster inside a multi-texture texlib bundle (jersey body vs. number font). `onWritten` fires after
 *  a successful write. `pendingRel` is the row currently mid-replace (to disable it).
 *
 *  Ownership: every async step — the picker, the write — is bound to the install root it started in
 *  and to a pick generation. A picker that returns after the install root changed (or after a newer
 *  pick / a cancel / unmount) opens nothing; the dialog always writes into the root the pick was made
 *  for (never the current render's); a write completing after the root changed or the hook unmounted
 *  shows no result and fires no callback; a second "Write texture" while one is running is ignored;
 *  Cancel is not offered while a write runs (hiding the dialog would not cancel the dispatched write). */
export function useTextureReplace(installDir: string | null, onWritten?: () => void) {
  const queryClient = useQueryClient();
  const [pending, setPending] = useState<Pending | null>(null);
  const [imported, setImported] = useState<ImportTextureSummaryDto | null>(null);
  const [writing, setWriting] = useState(false);
  const [writeError, setWriteError] = useState<string | null>(null);

  const rootRef = useRef(installDir);
  rootRef.current = installDir;
  const onWrittenRef = useRef(onWritten);
  onWrittenRef.current = onWritten;
  const genRef = useRef(0);
  const writingRef = useRef(false);
  const mountedRef = useRef(true);
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      genRef.current += 1;
    };
  }, []);
  // A new install root: nothing held for the old one is shown again.
  useEffect(() => {
    genRef.current += 1;
    setPending(null);
    setImported(null);
    setWriteError(null);
  }, [installDir]);

  const live = (gen: number, forRoot: string) => mountedRef.current && gen === genRef.current && rootRef.current === forRoot;

  const pick = async (relPath: string, name: string, textureIndex?: number) => {
    if (writingRef.current) return; // a write is running: its target must not be replaced under it
    const forRoot = installDir;
    if (!forRoot) return;
    const gen = ++genRef.current;
    const src = await openDialog({
      multiple: false,
      directory: false,
      title: "Choose the edited image to write into this texture",
      filters: [{ name: "Image", extensions: ["png", "jpg", "jpeg", "bmp", "tga"] }],
    });
    if (typeof src !== "string" || !live(gen, forRoot)) return;
    setImported(null);
    setWriteError(null);
    setPending({ installDir: forRoot, relPath, name, sourcePath: src, textureIndex });
  };

  /** Write the pending pick into the root it was made for, under the review the dialog displayed
   *  (`reviewId`, from `texture_replace_plan`). A second call while one runs is ignored. */
  const confirm = async (reviewId: bigint) => {
    if (writingRef.current || !pending) return;
    const p = pending;
    const gen = genRef.current;
    writingRef.current = true;
    setWriting(true);
    setWriteError(null);
    try {
      const r: ImportTextureSummaryDto = await call("import_texture", {
        installDir: p.installDir,
        relPath: p.relPath,
        sourceImagePath: p.sourcePath,
        writeBackup: true,
        textureIndex: p.textureIndex,
        reviewId,
      });
      if (!live(gen, p.installDir)) return; // the file is written; this hook no longer shows that root
      setImported(r);
      setPending(null);
      refreshAssetPreviews(queryClient);
      onWrittenRef.current?.();
    } catch (e) {
      if (live(gen, p.installDir)) setWriteError(String(e));
    } finally {
      writingRef.current = false;
      if (mountedRef.current) setWriting(false);
    }
  };

  const cancel = () => {
    if (writingRef.current) return;
    genRef.current += 1;
    setWriteError(null);
    setPending(null);
  };

  const dialog = pending ? (
    <ReplaceTextureDialog
      installDir={pending.installDir}
      relPath={pending.relPath}
      name={pending.name}
      sourcePath={pending.sourcePath}
      textureIndex={pending.textureIndex}
      confirming={writing}
      error={writeError}
      onConfirm={(reviewId) => void confirm(reviewId)}
      onCancel={cancel}
    />
  ) : null;

  const result = imported ? (
    <ImportResult summary={imported} onDismiss={() => setImported(null)} />
  ) : null;

  return { pick, dialog, result, imported, pendingRel: pending?.relPath ?? null };
}

/** A small decoded thumbnail of an extracted texture, sized server-side. Lazily decoded and cached,
 *  so a list decodes each row once. Falls back to a neutral box while loading or if it doesn't decode. */
export function TextureThumbnail({
  installDir,
  relPath,
  className = "h-8 w-8",
  maxEdge = 48,
  textureIndex,
}: {
  installDir: string;
  relPath: string;
  className?: string;
  maxEdge?: number;
  textureIndex?: number;
}) {
  const q = useQuery({
    queryKey: ["texturePreview", installDir, relPath, maxEdge, textureIndex],
    queryFn: () => call("texture_preview", { installDir, relPath, maxEdge, textureIndex }),
    staleTime: Infinity,
    retry: false,
  });
  return q.data ? (
    <img
      src={q.data.data_url}
      alt=""
      className={`shrink-0 rounded border border-slate-700 bg-slate-800/60 object-contain ${className}`}
    />
  ) : (
    <div className={`shrink-0 rounded border border-slate-800 bg-slate-800/30 ${className}`} />
  );
}

/** A thumbnail of a texture inside an EB render archive (PS3 NHL 12 / 13 kit members), decoded
 *  server-side from the archive's member. Same neutral fallback as `TextureThumbnail`. */
export function ArchiveMemberThumbnail({
  installDir,
  archiveRelPath,
  memberPath,
  className = "h-8 w-8",
  maxEdge = 48,
}: {
  installDir: string;
  archiveRelPath: string;
  memberPath: string;
  className?: string;
  maxEdge?: number;
}) {
  const q = useQuery({
    queryKey: ["archiveMemberPreview", installDir, archiveRelPath, memberPath, maxEdge],
    queryFn: () => call("archive_member_texture_preview", { installDir, archiveRelPath, memberPath, maxEdge }),
    staleTime: Infinity,
    retry: false,
  });
  return q.data ? (
    <img
      src={q.data.data_url}
      alt=""
      className={`shrink-0 rounded border border-slate-700 bg-slate-800/60 object-contain ${className}`}
      data-testid="archive-member-thumbnail"
    />
  ) : (
    <span
      className={`inline-grid place-items-center rounded border border-slate-700 text-[9px] uppercase text-slate-500 ${className}`}
      title={q.error ? `could not decode the archive member: ${String(q.error)}` : "decoding the archive member…"}
      data-testid="archive-member-thumbnail-pending"
    >
      big
    </span>
  );
}

/** The before/after confirm step for a texture replace. Decodes the current on-disk texture and the
 *  picked source side by side, reports both dimensions, and lets the write proceed when the source
 *  fits or the container resizes (portrait archives, PNG, and since 21 Sep 2026 loose texlibs and
 *  GNF under the measured rules); the write reports the rule's refusals. */
function ReplaceTextureDialog({
  installDir,
  relPath,
  name,
  sourcePath,
  textureIndex,
  confirming,
  error,
  onConfirm,
  onCancel,
}: {
  installDir: string;
  relPath: string;
  name: string;
  sourcePath: string;
  textureIndex?: number;
  confirming: boolean;
  error: string | null;
  onConfirm: (reviewId: bigint) => void;
  onCancel: () => void;
}) {
  // The "before" image is re-read on every open: a file replaced at the same path since the last
  // open must not be shown (or planned against) from the cache.
  const current = useQuery({
    queryKey: ["texturePreview", installDir, relPath, 0, textureIndex],
    queryFn: () => call("texture_preview", { installDir, relPath, maxEdge: 0, textureIndex }),
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
    retry: false,
  });
  const source = useQuery({
    queryKey: ["sourceImagePreview", sourcePath],
    queryFn: () => call("source_image_preview", { sourceImagePath: sourcePath }),
    staleTime: Infinity,
    retry: false,
  });

  const cur = current.data;
  const src = source.data;
  // The pre-Apply plan (coordinator review of receipts 10–17 F1; R4 review F1, 23 Sep 2026):
  // once both sides are decoded, ask the writer's own policy what this exact write would do. An
  // Xbox 360 RW4xb2 DXT raster answers with a refusal (a changed size on a mipped raster) or the
  // partial-replacement disclosure the result will repeat; other containers answer that no
  // pre-check exists. **Write is enabled only by a current, successful answer for this exact
  // target and source**: while the plan is pending or failed nothing can be written (a partial
  // replacement must show its disclosure first), a failure offers Retry, and the answer is keyed to
  // the target path / raster, the source path and its dimensions and re-read on every open, so a
  // file replaced at the same path or a different source never inherits a displayed policy. The
  // writer stays the authority at write time.
  //
  // Since the 24 Sep 2026 review (F1) the plan is a **server-held review**: the backend reads the
  // target and the source itself, binds their bytes, the raster, the selection and the displayed
  // policy under `review_id`, and the write names that id. A target or source that changed after
  // this plan was shown makes the write refuse (a full replacement reviewed, a partial one
  // performed, is exactly the refused case); the dialog then re-plans so a fresh review is
  // displayed — it never re-plans and writes in one step.
  const plan = useQuery({
    queryKey: ["textureReplacePlan", installDir, relPath, textureIndex, sourcePath],
    queryFn: () => call("texture_replace_plan", { installDir, relPath, sourceImagePath: sourcePath, textureIndex }),
    enabled: !!cur && !!src,
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
    retry: false,
  });
  // A refused write (drift, or any other failure) invalidates the displayed review: re-read and
  // re-plan, show the fresh answer, and let the operator decide again.
  const planRefetch = plan.refetch;
  useEffect(() => {
    if (error) void planRefetch();
  }, [error, planRefetch]);
  const planData: TextureReplacePlanDto | undefined = plan.isSuccess && !plan.isFetching ? plan.data : undefined;
  const refusedByPlan = !!planData && planData.checked && !planData.allowed;
  // A current successful answer that does not refuse — a checked "allowed" or the backend's own
  // statement that no pre-check exists for this container.
  const planReady = !!planData && !refusedByPlan;
  const dimsMatch = !!cur && !!src && cur.width === src.width && cur.height === src.height;
  // Portrait archives regenerate the XPR2/DDS descriptor; a PNG carries its own size; since
  // 21 Sep 2026 (research handoffs 11 / 15) loose texlibs (Xbox 360 RW4xb2 DXT ≥ 128 texels on
  // NHL 09–Legacy, PS3 RW4ps3 any DXT) and PS4 GNF 2D block textures resize too — the write
  // re-derives the descriptor / header by the measured rule and refuses the cases outside it
  // (tiny / linear / cube / BC6H / NHL 07–08 / 8_8_8_8 texlib rasters) with the reason. (An RW4 or
  // uncompressed-DDS league-logo package inside a BIGF stays same-size only — the write reports it.)
  const resizable = cur?.container === "portrait" || cur?.container === "png" || cur?.container === "texlib" || cur?.container === "gnf";
  const blocked = !!cur && !!src && !dimsMatch && !resizable;
  const loading = current.isLoading || source.isLoading;
  const decodeError =
    (current.error && `Couldn't decode the current texture: ${String(current.error)}`) ||
    (source.error && `Couldn't read the source image: ${String(source.error)}`) ||
    null;

  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-black/60 p-4" onClick={confirming ? undefined : onCancel}>
      <div
        className="w-full max-w-2xl rounded-lg border border-slate-700 bg-slate-900 shadow-xl"
        onClick={(e) => e.stopPropagation()}
        data-testid="texture-replace-dialog"
      >
        <header className="border-b border-slate-800 px-4 py-3">
          <h2 className="text-sm font-semibold text-slate-100">Replace texture</h2>
          <p className="mt-0.5 truncate text-xs text-slate-400" title={relPath}>
            {name}
            {cur?.container ? ` · ${CONTAINER_LABEL[cur.container] ?? cur.container}` : ""}
          </p>
        </header>

        <div className="px-4 py-4">
          {loading ? (
            <div className="grid h-40 place-items-center">
              <Spinner label="Decoding preview…" />
            </div>
          ) : decodeError ? (
            <ErrorBanner title="Preview failed" message={decodeError} />
          ) : (
            <div className="grid grid-cols-[1fr_auto_1fr] items-center gap-3">
              <PreviewPane label="Current" preview={cur} />
              <span className="text-2xl text-slate-500">→</span>
              <PreviewPane label="New" preview={src} />
            </div>
          )}

          {!loading && !decodeError && cur && src ? (
            blocked ? (
              <p className="mt-3 rounded-md border border-rose-800 bg-rose-950/40 px-3 py-2 text-xs text-rose-200">
                Size mismatch — this {CONTAINER_LABEL[cur.container ?? ""] ?? "texture"} must be
                replaced at its exact size ({cur.width}×{cur.height}). Resize your image to match, then
                try again.
              </p>
            ) : !dimsMatch ? (
              <p className="mt-3 rounded-md border border-amber-800 bg-amber-950/40 px-3 py-2 text-xs text-amber-200" data-testid="texture-replace-resize">
                Your image is {src.width}×{src.height}; the current texture is {cur.width}×{cur.height}.
                It will be re-encoded to {src.width}×{src.height} on write.
                {cur.container === "portrait"
                  ? " A league logo / imagery package is resized only where it draws its texture 1:1 (the league-logo families, division logos): the Apt quad and size words are rewritten with the texture (Xbox 360 NHL 12–Legacy XPR2 at 64 or 256 wide, PS3 NHL 12–Legacy DDS; RW4 packages of NHL 07–11 are same-size only). Packages that draw a smaller or rotated quad (trophies, broadcast trophies, MUD logos) refuse with the reason. Whether the game accepts a resized quad is a runtime question."
                  : cur.container === "texlib"
                    ? " The raster's descriptor is re-derived by the measured rule (Xbox 360 NHL 09–Legacy DXT rasters of 128 texels or more; PS3 DXT rasters). On Xbox 360 only a single-level raster changes size (one level re-emitted); a raster with a mip chain refuses a changed size, because the console's packed-mip layout is not established. Tiny, linear, cube-map, uncompressed and NHL 07 / 08 rasters refuse with the reason. Whether the game accepts a resized raster is a runtime question."
                    : cur.container === "gnf"
                      ? " The GNF header's size, pitch, mip and byte-count words are recomputed by the measured rule and the micro-tiled chain re-emitted; cube maps, BC6H, volumes and headers the rule does not reproduce refuse with the reason. The registry's size directory still decides which sizes the game looks up. Whether the game accepts a resized texture is a runtime question."
                      : ""}
              </p>
            ) : (
              <p className="mt-3 text-xs text-slate-500">
                Sizes match ({cur.width}×{cur.height}). A one-time <code>.bak</code> of the original is
                kept before the first write.
              </p>
            )
          ) : null}

          {!loading && !decodeError && cur && src ? (
            plan.isLoading || plan.isFetching ? (
              <p className="mt-2 text-xs text-slate-500" data-testid="texture-replace-plan-loading">
                Checking what this write would do…
              </p>
            ) : plan.error ? (
              <div className="mt-2 rounded-md border border-amber-800 bg-amber-950/40 px-3 py-2 text-xs text-amber-200" data-testid="texture-replace-plan-unavailable">
                <p>
                  The pre-write check failed ({String(plan.error)}). Nothing is written until the check answers, because a
                  partial replacement must show what it keeps before it runs.
                </p>
                <Button variant="secondary" onClick={() => void plan.refetch()} data-testid="texture-replace-plan-retry">
                  Retry check
                </Button>
              </div>
            ) : refusedByPlan ? (
              <p className="mt-2 rounded-md border border-rose-800 bg-rose-950/40 px-3 py-2 text-xs text-rose-200" data-testid="texture-replace-plan-refusal">
                This write would be refused: {planData?.refusal}
              </p>
            ) : (
              planData?.notes.map((n) => (
                <p key={n} className="mt-2 rounded-md border border-amber-800 bg-amber-950/40 px-3 py-2 text-xs text-amber-200" data-testid="texture-replace-plan-note">
                  {n}
                </p>
              ))
            )
          ) : null}

          {confirming ? (
            <p className="mt-3 text-xs text-slate-400" data-testid="texture-replace-writing">
              Writing… the request has been sent; closing this dialog would not cancel it.
            </p>
          ) : null}
          {error ? (
            <div className="mt-3">
              <ErrorBanner title="Write failed" message={error} />
            </div>
          ) : null}
        </div>

        <footer className="flex items-center justify-end gap-2 border-t border-slate-800 px-4 py-3">
          <Button variant="secondary" onClick={onCancel} disabled={confirming} data-testid="texture-replace-cancel">
            Cancel
          </Button>
          <Button
            onClick={() => {
              if (planData) onConfirm(planData.review_id);
            }}
            disabled={confirming || loading || !!decodeError || blocked || !planReady}
            data-testid="texture-replace-write"
          >
            {confirming ? "Writing…" : "Write texture"}
          </Button>
        </footer>
      </div>
    </div>
  );
}

// A subtle checkerboard so a texture's transparency (alpha) reads clearly behind the image.
const CHECKER: CSSProperties = {
  backgroundImage:
    "linear-gradient(45deg, #1e293b 25%, transparent 25%), linear-gradient(-45deg, #1e293b 25%, transparent 25%), linear-gradient(45deg, transparent 75%, #1e293b 75%), linear-gradient(-45deg, transparent 75%, #1e293b 75%)",
  backgroundSize: "16px 16px",
  backgroundPosition: "0 0, 0 8px, 8px -8px, -8px 0px",
  backgroundColor: "#0f172a",
};

/** One labelled image pane in the before/after grid. */
function PreviewPane({ label, preview }: { label: string; preview?: TexturePreviewDto }) {
  return (
    <div className="min-w-0">
      <div className="mb-1 text-[11px] uppercase tracking-wide text-slate-500">{label}</div>
      <div
        className="grid aspect-square place-items-center overflow-hidden rounded-md border border-slate-800"
        style={CHECKER}
      >
        {preview ? (
          <img src={preview.data_url} alt={label} className="max-h-full max-w-full object-contain" />
        ) : null}
      </div>
      {preview ? (
        <div className="mt-1 text-center text-xs tabular-nums text-slate-400">
          {preview.width}×{preview.height}
        </div>
      ) : null}
    </div>
  );
}

/** The **bytes** route as a reviewed two-step write (24 Sep 2026 review F1), for a texture the UI
 *  bakes itself (the nameplate editor): `stage(pngBase64)` asks the backend what writing that image
 *  into `relPath` would do and holds the image under a review; the caller shows the answer (a
 *  refusal, the partial-replacement disclosure, or "no pre-check"); `write()` then names that review
 *  and the backend writes exactly the reviewed image, refusing if the target changed meanwhile.
 *  A refused or failed write drops the review — the caller has to stage again, so a fresh plan is
 *  shown before any further write. Every step is bound to the (installDir, relPath, textureIndex)
 *  it started with and to a stage generation; a later stage, `reset()` or a target change
 *  discards the result of an earlier one. */
export function useReviewedBytesWrite(installDir: string, relPath: string, textureIndex?: number) {
  const queryClient = useQueryClient();
  const [plan, setPlan] = useState<TextureReplacePlanDto | null>(null);
  const [planning, setPlanning] = useState(false);
  const [planError, setPlanError] = useState<string | null>(null);
  const [writing, setWriting] = useState(false);
  const [writeError, setWriteError] = useState<string | null>(null);
  const [result, setResult] = useState<ImportTextureSummaryDto | null>(null);
  const genRef = useRef(0);
  const writingRef = useRef(false);
  const mountedRef = useRef(true);
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      genRef.current += 1;
    };
  }, []);
  const targetKey = `${installDir}\u0000${relPath}\u0000${textureIndex ?? ""}`;
  const targetRef = useRef(targetKey);
  useEffect(() => {
    if (targetRef.current === targetKey) return;
    targetRef.current = targetKey;
    genRef.current += 1;
    setPlan(null);
    setPlanError(null);
    setWriteError(null);
    setResult(null);
  }, [targetKey]);
  const live = (gen: number) => mountedRef.current && gen === genRef.current && targetRef.current === targetKey;

  const reset = () => {
    if (writingRef.current) return;
    genRef.current += 1;
    setPlan(null);
    setPlanError(null);
    setWriteError(null);
    setPlanning(false);
  };

  const stage = async (pngBase64: string) => {
    if (writingRef.current) return;
    const gen = ++genRef.current;
    setPlan(null);
    setPlanError(null);
    setWriteError(null);
    setResult(null);
    setPlanning(true);
    try {
      const p: TextureReplacePlanDto = await call("texture_replace_plan_bytes", { installDir, relPath, pngBase64, textureIndex });
      if (!live(gen)) return;
      setPlan(p);
    } catch (e) {
      if (live(gen)) setPlanError(String(e));
    } finally {
      if (live(gen)) setPlanning(false);
    }
  };

  const write = async () => {
    if (writingRef.current || !plan || (plan.checked && !plan.allowed)) return;
    const gen = genRef.current;
    const reviewId = plan.review_id;
    writingRef.current = true;
    setWriting(true);
    setWriteError(null);
    try {
      const r: ImportTextureSummaryDto = await call("import_texture_bytes", { installDir, relPath, writeBackup: true, textureIndex, reviewId });
      if (!live(gen)) return;
      setResult(r);
      refreshAssetPreviews(queryClient);
      setPlan(null);
    } catch (e) {
      if (live(gen)) {
        setWriteError(String(e));
        setPlan(null); // the review is spent either way; a fresh plan must be shown before another write
      }
    } finally {
      writingRef.current = false;
      if (mountedRef.current) setWriting(false);
    }
  };

  const refused = !!plan && plan.checked && !plan.allowed;
  return { stage, write, reset, plan, planning, planError, writing, writeError, result, refused, canWrite: !!plan && !refused && !writing };
}

/** The reviewed plan of a bytes-route write, for the nameplate editor's footer: the refusal, the
 *  disclosure notes, or the statement that this container has no pre-check. */
export function ReviewedPlanNotes({ plan }: { plan: TextureReplacePlanDto }) {
  if (plan.checked && !plan.allowed) {
    return (
      <p className="rounded-md border border-rose-800 bg-rose-950/40 px-3 py-2 text-xs text-rose-200" data-testid="reviewed-plan-refusal">
        This write would be refused: {plan.refusal}
      </p>
    );
  }
  if (plan.notes.length === 0) {
    return (
      <p className="text-xs text-slate-500" data-testid="reviewed-plan-clear">
        {plan.checked
          ? `Full replacement of the ${plan.source_width}×${plan.source_height} raster; a one-time .bak of the original is kept before the first write.`
          : `No pre-write check exists for this ${CONTAINER_LABEL[plan.container ?? ""] ?? "container"}: the writer's own rule decides at write time. A one-time .bak of the original is kept before the first write.`}
      </p>
    );
  }
  return (
    <>
      {plan.notes.map((n) => (
        <p key={n} className="rounded-md border border-amber-800 bg-amber-950/40 px-3 py-2 text-xs text-amber-200" data-testid="reviewed-plan-note">
          {n}
        </p>
      ))}
    </>
  );
}

/** The green "wrote N" card after a successful texture write. */
export function ImportResult({
  summary,
  onDismiss,
}: {
  summary: ImportTextureSummaryDto;
  onDismiss: () => void;
}): ReactNode {
  const name = summary.target_path.split(/[\\/]/).pop() ?? summary.target_path;
  return (
    <div className="rounded-md border border-emerald-800/60 bg-emerald-950/30 p-3 text-sm">
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-emerald-300">
            Wrote <span className="font-medium">{name}</span> ·{" "}
            {CONTAINER_LABEL[summary.container] ?? summary.container} · {summary.width}×
            {summary.height} · {formatBytes(summary.bytes_written)}
          </p>
          {summary.backup_path ? (
            <p className="mt-0.5 text-xs text-slate-400">
              Original backed up to <span className="text-slate-300">{summary.backup_path}</span>
            </p>
          ) : null}
          {summary.notes.map((n) => (
            <p key={n} className="mt-1 rounded-md border border-amber-800 bg-amber-950/40 px-2 py-1 text-xs text-amber-200" data-testid="texture-replace-note">
              {n}
            </p>
          ))}
          <p className="mt-1 text-xs text-slate-500">
            The edited file is at the destination shown above. Project changes still need deployment;
            loose-file edits may need repacking before the game reads them.
          </p>
        </div>
        <button
          onClick={onDismiss}
          className="shrink-0 text-xs text-slate-400 hover:text-slate-200"
          type="button"
        >
          Dismiss
        </button>
      </div>
    </div>
  );
}
