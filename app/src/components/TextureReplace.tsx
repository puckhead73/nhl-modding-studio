// Shared texture-replace UI — the export→edit→reimport loop used by the File Browser and the Team
// Uniforms surface. `useTextureReplace` owns the pick → before/after preview → confirm → result flow;
// the dialog decodes the current on-disk texture beside the picked source, reports both dimensions,
// and only lets the write proceed when the source fits (GNF/texlib are same-size; a portrait archive
// can be resized). Everything routes through the same server-side `import_texture` dispatcher.

import { useState, type CSSProperties, type ReactNode } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { call } from "../lib/client";
import { Button, ErrorBanner, Spinner } from "./primitives";
import type { ImportTextureSummaryDto } from "../bindings/ImportTextureSummaryDto";
import type { TexturePreviewDto } from "../bindings/TexturePreviewDto";

const CONTAINER_LABEL: Record<string, string> = {
  gnf: "PS4 GNF",
  portrait: "portrait archive",
  texlib: "RW4 texlib",
};

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

type Pending = { relPath: string; name: string; sourcePath: string; textureIndex?: number };

/** Manage a texture replace: `pick(relPath, name, textureIndex?)` opens the image picker and stages a
 *  before/after preview; `dialog` and `result` are ready-to-render nodes. `textureIndex` targets one
 *  raster inside a multi-texture texlib bundle (jersey body vs. number font). `onWritten` fires after
 *  a successful write. `pendingRel` is the row currently mid-replace (to disable it). */
export function useTextureReplace(installDir: string | null, onWritten?: () => void) {
  const [pending, setPending] = useState<Pending | null>(null);
  const [imported, setImported] = useState<ImportTextureSummaryDto | null>(null);

  const confirm = useMutation({
    mutationFn: async () => {
      if (!pending || !installDir) return null;
      return call("import_texture", {
        installDir,
        relPath: pending.relPath,
        sourceImagePath: pending.sourcePath,
        writeBackup: true,
        textureIndex: pending.textureIndex,
      });
    },
    onSuccess: (r) => {
      if (r) {
        setImported(r);
        setPending(null);
        onWritten?.();
      }
    },
  });

  const pick = async (relPath: string, name: string, textureIndex?: number) => {
    const src = await openDialog({
      multiple: false,
      directory: false,
      title: "Choose the edited image to write into this texture",
      filters: [{ name: "Image", extensions: ["png", "jpg", "jpeg", "bmp", "tga"] }],
    });
    if (typeof src !== "string") return;
    setImported(null);
    setPending({ relPath, name, sourcePath: src, textureIndex });
  };

  const dialog =
    pending && installDir ? (
      <ReplaceTextureDialog
        installDir={installDir}
        relPath={pending.relPath}
        name={pending.name}
        sourcePath={pending.sourcePath}
        textureIndex={pending.textureIndex}
        confirming={confirm.isPending}
        error={confirm.error ? String(confirm.error) : null}
        onConfirm={() => confirm.mutate()}
        onCancel={() => {
          confirm.reset();
          setPending(null);
        }}
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

/** The before/after confirm step for a texture replace. Decodes the current on-disk texture and the
 *  picked source side by side, reports both dimensions, and only lets the write proceed when the
 *  source fits — GNF/texlib are same-size only; a portrait archive can be resized. */
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
  onConfirm: () => void;
  onCancel: () => void;
}) {
  const current = useQuery({
    queryKey: ["texturePreview", installDir, relPath, 0, textureIndex],
    queryFn: () => call("texture_preview", { installDir, relPath, maxEdge: 0, textureIndex }),
    staleTime: Infinity,
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
  const dimsMatch = !!cur && !!src && cur.width === src.width && cur.height === src.height;
  const resizable = cur?.container === "portrait"; // portrait archives regenerate the descriptor
  const blocked = !!cur && !!src && !dimsMatch && !resizable;
  const loading = current.isLoading || source.isLoading;
  const decodeError =
    (current.error && `Couldn't decode the current texture: ${String(current.error)}`) ||
    (source.error && `Couldn't read the source image: ${String(source.error)}`) ||
    null;

  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-black/60 p-4" onClick={onCancel}>
      <div
        className="w-full max-w-2xl rounded-lg border border-slate-700 bg-slate-900 shadow-xl"
        onClick={(e) => e.stopPropagation()}
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
              <p className="mt-3 rounded-md border border-amber-800 bg-amber-950/40 px-3 py-2 text-xs text-amber-200">
                Your image is {src.width}×{src.height}; the current texture is {cur.width}×{cur.height}.
                It will be re-encoded to {src.width}×{src.height} on write.
              </p>
            ) : (
              <p className="mt-3 text-xs text-slate-500">
                Sizes match ({cur.width}×{cur.height}). A one-time <code>.bak</code> of the original is
                kept before the first write.
              </p>
            )
          ) : null}

          {error ? (
            <div className="mt-3">
              <ErrorBanner title="Write failed" message={error} />
            </div>
          ) : null}
        </div>

        <footer className="flex items-center justify-end gap-2 border-t border-slate-800 px-4 py-3">
          <Button variant="secondary" onClick={onCancel} disabled={confirming}>
            Cancel
          </Button>
          <Button onClick={onConfirm} disabled={confirming || loading || !!decodeError || blocked}>
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
          <p className="mt-1 text-xs text-slate-500">
            The edited file is in your extracted tree — run it back through your own repack/install
            step to see it in-game.
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
