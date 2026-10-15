import { assetRoot } from "../lib/assetRoot";
// File Browser — pick any file (or a whole folder) from the game's extracted install tree and
// export it, decoding recognized assets to the same glTF/PNG/DDS the player-asset exporter produces.
//
// Where the Export page is entity-driven (choose *players*, art resolved by EA's naming convention),
// this is file-driven: you navigate the real on-disk tree — which already reflects each title's own
// layout — and choose what comes out. Extraction of `.big` archives happens at onboarding, so this
// operates purely on the loose tree rooted at the install dir.

import { useMemo, useState } from "react";
import { useQuery, useMutation } from "@tanstack/react-query";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import {
  Button,
  Card,
  EmptyState,
  ErrorBanner,
  PageScroll,
  Pill,
  Spinner,
  Stat,
  type Tone,
} from "../components/primitives";
import {
  formatBytes,
  TextureThumbnail,
  useTextureReplace,
} from "../components/TextureReplace";
import type { BrowseEntryDto } from "../bindings/BrowseEntryDto";
import type { FileExportSummaryDto } from "../bindings/FileExportSummaryDto";

/** File kinds whose texture can be re-encoded and written back in place. */
const REPLACEABLE_KINDS = new Set(["portrait", "texture"]);

/** Badge tone + label for a file's decoded output kind (null = the exporter doesn't recognize it). */
const KIND_META: Record<string, { tone: Tone; label: string }> = {
  mesh: { tone: "info", label: "mesh → glb" },
  portrait: { tone: "ok", label: "portrait → png" },
  texture: { tone: "warn", label: "texture" },
};

export function FileBrowserPage() {
  const { install } = useProject();
  const installDir = assetRoot(install) ?? null;

  const [cwd, setCwd] = useState(""); // "" = the install root
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [meshesGlb, setMeshesGlb] = useState(true);
  const [portraitsPng, setPortraitsPng] = useState(true);
  const [textures, setTextures] = useState(true);
  const [copyUnrecognized, setCopyUnrecognized] = useState(false);
  const [crop, setCrop] = useState(false);
  const [summary, setSummary] = useState<FileExportSummaryDto | null>(null);
  const [showThumbs, setShowThumbs] = useState(false);

  const listing = useQuery({
    queryKey: ["browseInstallFiles", installDir, cwd],
    queryFn: () => call("browse_install_files", { installDir: installDir!, rel: cwd }),
    enabled: !!installDir,
  });

  const entries = useMemo(() => listing.data?.entries ?? [], [listing.data]);

  const toggle = (relPath: string) =>
    setSelected((prev) => {
      const next = new Set(prev);
      next.has(relPath) ? next.delete(relPath) : next.add(relPath);
      return next;
    });

  const selectAllHere = () =>
    setSelected((prev) => {
      const next = new Set(prev);
      for (const e of entries) next.add(e.rel_path);
      return next;
    });

  const run = useMutation({
    mutationFn: async () => {
      const outDir = await openDialog({
        multiple: false,
        directory: true,
        title: "Choose a folder to export the selected files into",
      });
      if (typeof outDir !== "string") return null;
      return call("export_install_files", {
        installDir: installDir!,
        relPaths: [...selected],
        outDir,
        options: {
          meshes_glb: meshesGlb,
          portraits_png: portraitsPng,
          textures,
          copy_unrecognized: copyUnrecognized,
          crop_portraits: crop,
        },
      });
    },
    onSuccess: (s) => s && setSummary(s),
  });

  // The pick → before/after preview → confirm → result flow (shared with the Team Uniforms surface).
  // Refetch the listing after a write, since the on-disk size may have changed.
  const replace = useTextureReplace(installDir, () => void listing.refetch());

  if (!installDir) {
    return (
      <EmptyState
        title="No install folder for this project"
        hint="Choose a game source folder under Project Overview → Game source folders. Your edits stay in this project."
      />
    );
  }

  return (
    <PageScroll>
      <div className="max-w-4xl space-y-3 p-3">
        <Card
          title="File browser"
          subtitle="Browse the install's files and export anything you pick — meshes to glTF, portraits and textures to images. Folders export in bulk, keeping the game's own layout."
        >
          <Breadcrumb cwd={cwd} onNavigate={setCwd} />

          {listing.isLoading ? (
            <Spinner label="Reading folder…" />
          ) : listing.error ? (
            <ErrorBanner title="Couldn't read this folder" message={String(listing.error)} />
          ) : (
            <div className="mt-2 overflow-hidden rounded-md border border-slate-800">
              {listing.data?.parent !== null && listing.data?.parent !== undefined ? (
                <Row
                  onClick={() => setCwd(listing.data!.parent!)}
                  className="text-slate-400 hover:bg-slate-800/60"
                >
                  <span className="w-5 text-center">↵</span>
                  <span className="flex-1">.. (up one folder)</span>
                </Row>
              ) : null}

              {entries.length === 0 ? (
                <div className="px-3 py-6 text-center text-xs text-slate-500">
                  This folder is empty.
                </div>
              ) : (
                entries.map((e) => {
                  const replaceable = !e.is_dir && !!e.kind && REPLACEABLE_KINDS.has(e.kind);
                  return (
                    <EntryRow
                      key={e.rel_path}
                      entry={e}
                      checked={selected.has(e.rel_path)}
                      onToggle={() => toggle(e.rel_path)}
                      onOpen={e.is_dir ? () => setCwd(e.rel_path) : undefined}
                      onReplace={replaceable ? () => void replace.pick(e.rel_path, e.name) : undefined}
                      replacing={replace.pendingRel === e.rel_path}
                      thumb={showThumbs && replaceable ? { installDir } : undefined}
                    />
                  );
                })
              )}
            </div>
          )}

          {entries.length > 0 ? (
            <div className="mt-2 flex flex-wrap items-center gap-3 text-xs">
              <button
                onClick={selectAllHere}
                className="text-sky-400 hover:text-sky-300"
                type="button"
              >
                Select everything in this folder
              </button>
              <span className="text-slate-600">·</span>
              <label className="flex cursor-pointer items-center gap-1.5 text-slate-400">
                <input
                  type="checkbox"
                  checked={showThumbs}
                  onChange={(ev) => setShowThumbs(ev.target.checked)}
                />
                Show thumbnails
              </label>
              <span className="text-slate-600">·</span>
              <span className="text-slate-500">
                Portraits and textures show a <span className="text-sky-400">Replace…</span> action to
                preview and write an edited image back in place.
              </span>
            </div>
          ) : null}

          {replace.result ? <div className="mt-2">{replace.result}</div> : null}
        </Card>

        {replace.dialog}

        <Card title="Export" subtitle="What to write for the selected files.">
          <div className="space-y-4 text-sm">
            <div className="space-y-1">
              <Check label="Meshes → .glb" checked={meshesGlb} onChange={setMeshesGlb} />
              <Check label="Portraits → .png" checked={portraitsPng} onChange={setPortraitsPng} />
              <Check
                label="Textures → .png"
                checked={textures}
                onChange={setTextures}
                hint="RenderWare texlibs and PS4 GNF maps decode to .png."
              />
              <Check
                label="Crop portraits to the subject"
                checked={crop}
                onChange={setCrop}
                hint="EA ships portraits on a mostly-empty canvas. Leave off to keep them as shipped."
              />
              <Check
                label="Copy unrecognized files as-is"
                checked={copyUnrecognized}
                onChange={setCopyUnrecognized}
                hint="Include files the exporter can't decode (and fall back to a raw copy when a decode fails), rather than skipping them."
              />
            </div>

            <div className="flex items-center gap-3 border-t border-slate-800 pt-3">
              <Button
                onClick={() => run.mutate()}
                disabled={run.isPending || selected.size === 0}
              >
                {run.isPending ? "Exporting…" : `Export ${selected.size} selected…`}
              </Button>
              <p className="text-xs text-slate-500">
                {selected.size === 0
                  ? "Tick files or folders above to select them."
                  : `${selected.size} item${selected.size === 1 ? "" : "s"} selected`}
              </p>
              {selected.size > 0 ? (
                <button
                  onClick={() => setSelected(new Set())}
                  className="text-xs text-slate-400 hover:text-slate-200"
                  type="button"
                >
                  Clear
                </button>
              ) : null}
            </div>

            {run.error ? <ErrorBanner title="Export failed" message={String(run.error)} /> : null}
          </div>
        </Card>

        {summary ? <Summary summary={summary} /> : null}
      </div>
    </PageScroll>
  );
}

function Breadcrumb({ cwd, onNavigate }: { cwd: string; onNavigate: (rel: string) => void }) {
  const parts = cwd ? cwd.split("/") : [];
  return (
    <div className="flex flex-wrap items-center gap-1 text-xs text-slate-400">
      <button onClick={() => onNavigate("")} className="hover:text-sky-300" type="button">
        install
      </button>
      {parts.map((part, i) => {
        const rel = parts.slice(0, i + 1).join("/");
        return (
          <span key={rel} className="flex items-center gap-1">
            <span className="text-slate-600">/</span>
            <button onClick={() => onNavigate(rel)} className="hover:text-sky-300" type="button">
              {part}
            </button>
          </span>
        );
      })}
    </div>
  );
}

function EntryRow({
  entry,
  checked,
  onToggle,
  onOpen,
  onReplace,
  replacing,
  thumb,
}: {
  entry: BrowseEntryDto;
  checked: boolean;
  onToggle: () => void;
  onOpen?: () => void;
  onReplace?: () => void;
  replacing?: boolean;
  /** When set, show a decoded thumbnail of this texture in place of the file icon. */
  thumb?: { installDir: string };
}) {
  const kind = entry.kind ? KIND_META[entry.kind] : undefined;
  return (
    <Row className="hover:bg-slate-800/40">
      <input
        type="checkbox"
        checked={checked}
        onChange={onToggle}
        onClick={(e) => e.stopPropagation()}
        className="shrink-0"
        aria-label={`Select ${entry.name}`}
      />
      {thumb ? (
        <TextureThumbnail installDir={thumb.installDir} relPath={entry.rel_path} />
      ) : (
        <span className="w-5 shrink-0 text-center text-slate-500">
          {entry.is_dir ? "📁" : "📄"}
        </span>
      )}
      {onOpen ? (
        <button
          onClick={onOpen}
          className="flex-1 truncate text-left text-slate-200 hover:text-sky-300"
          type="button"
          title={entry.name}
        >
          {entry.name}
        </button>
      ) : (
        <span className="flex-1 truncate text-slate-300" title={entry.name}>
          {entry.name}
        </span>
      )}
      {kind ? <Pill tone={kind.tone}>{kind.label}</Pill> : null}
      {onReplace ? (
        <button
          onClick={onReplace}
          disabled={replacing}
          className="shrink-0 text-xs text-sky-400 hover:text-sky-300 disabled:text-slate-600"
          type="button"
          title="Preview and write an edited image back into this texture (keeps a .bak)"
        >
          Replace…
        </button>
      ) : null}
      <span className="w-20 shrink-0 text-right text-xs tabular-nums text-slate-500">
        {entry.is_dir ? "" : formatBytes(entry.size)}
      </span>
    </Row>
  );
}

function Row({
  children,
  className,
  onClick,
}: {
  children: React.ReactNode;
  className?: string;
  onClick?: () => void;
}) {
  return (
    <div
      onClick={onClick}
      className={
        "flex items-center gap-2 border-b border-slate-800/60 px-3 py-1.5 text-sm last:border-b-0 " +
        (onClick ? "cursor-pointer " : "") +
        (className ?? "")
      }
    >
      {children}
    </div>
  );
}

function Summary({ summary }: { summary: FileExportSummaryDto }) {
  return (
    <Card title="Last export" subtitle={`${formatBytes(summary.bytes_written)} written`}>
      <div className="grid grid-cols-4 gap-2">
        <Stat label="Meshes" value={summary.meshes_written} />
        <Stat label="Portraits" value={summary.portraits_written} />
        <Stat label="Textures" value={summary.textures_written} />
        <Stat label="Copied" value={summary.raw_copied} />
      </div>
      {summary.skipped_unrecognized > 0 ? (
        <p className="mt-2 text-xs text-slate-500">
          {summary.skipped_unrecognized.toLocaleString()} file
          {summary.skipped_unrecognized === 1 ? "" : "s"} skipped (not a recognized asset — enable
          “Copy unrecognized files” to include them).
        </p>
      ) : null}
      {summary.failures.length > 0 ? (
        <details className="mt-3 text-xs">
          <summary className="cursor-pointer select-none text-amber-500">
            {summary.failures.length} skipped or failed
          </summary>
          <ul className="mt-1 max-h-56 space-y-0.5 overflow-y-auto text-slate-400">
            {summary.failures.map((f, i) => (
              <li key={i}>{f}</li>
            ))}
          </ul>
        </details>
      ) : (
        <p className="mt-3 text-xs text-slate-500">No failures.</p>
      )}
    </Card>
  );
}

function Check({
  label,
  checked,
  onChange,
  hint,
}: {
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
  hint?: string;
}) {
  return (
    <label className="flex cursor-pointer items-start gap-2 text-slate-300">
      <input
        type="checkbox"
        className="mt-1"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
      />
      <span>
        {label}
        {hint ? <span className="block text-xs text-slate-500">{hint}</span> : null}
      </span>
    </label>
  );
}
