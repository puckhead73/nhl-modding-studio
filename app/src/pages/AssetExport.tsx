// Bulk asset export — write the game's player art out as a library named by player.
//
// The set is chosen by the same named filters the browse page uses, so "what you see" and "what
// you get" stay the same idea. Everything is named by slug (`lastname_firstname_dd_mm_yyyy`),
// which is also the key the historical corpus joins on.

import { useMemo, useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { call } from "../lib/client";
import { Button, Card, ErrorBanner, PageScroll, Spinner, Stat } from "../components/primitives";
import { AssetGate, isExportable, usePlayerAssets } from "./PlayerAssets";
import type { AssetExportSummaryDto } from "../bindings/AssetExportSummaryDto";
import type { AssetRootsArgsDto } from "../bindings/AssetRootsArgsDto";
import type { HeadCatalogSummaryDto } from "../bindings/HeadCatalogSummaryDto";
import type { PlayerAssetsDto } from "../bindings/PlayerAssetsDto";

type Scope = "exportable" | "portrait" | "head";

const SCOPES: { key: Scope; label: string; hint: string }[] = [
  {
    key: "exportable",
    label: "Everything exportable",
    hint: "Every player with art and a name to file it under",
  },
  { key: "portrait", label: "Players with a portrait", hint: "The portrait library" },
  { key: "head", label: "Players with their own head model", hint: "The head-mesh library" },
];

export function AssetExportPage() {
  const { projectId, installDir, rootArgs, roots, assets } = usePlayerAssets();

  const [scope, setScope] = useState<Scope>("exportable");
  const [portraits, setPortraits] = useState(true);
  const [meshes, setMeshes] = useState(true);
  const [textures, setTextures] = useState(true);
  const [raw, setRaw] = useState(true);
  const [crop, setCrop] = useState(false);
  const [summary, setSummary] = useState<AssetExportSummaryDto | null>(null);

  const all = useMemo(() => assets.data ?? [], [assets.data]);
  const selected = useMemo(() => {
    const inScope = (p: PlayerAssetsDto) =>
      scope === "portrait" ? !!p.portrait_path : scope === "head" ? !!p.mesh_path : true;
    // A player without a slug can't be named, so they're never in an export set.
    return all.filter((p) => isExportable(p) && inScope(p));
  }, [all, scope]);

  const run = useMutation({
    mutationFn: async () => {
      const outDir = await openDialog({
        multiple: false,
        directory: true,
        title: "Choose a folder to export player assets into",
      });
      if (typeof outDir !== "string") return null;
      return call("export_assets", {
        projectId,
        roots: rootArgs,
        outDir,
        bioRecords: selected.map((p) => p.bio_record),
        options: {
          portraits,
          meshes_glb: meshes,
          textures,
          raw_rx2: raw,
          crop_portraits: crop,
        },
      });
    },
    onSuccess: (s) => s && setSummary(s),
  });

  const gate = AssetGate({ installDir, roots });
  if (gate) return gate;
  if (assets.isLoading) return <Spinner label="Scanning player assets…" />;
  if (assets.error) return <ErrorBanner title="Couldn't scan assets" message={String(assets.error)} />;

  const noSlug = all.filter((p) => !p.slug).length;

  return (
    <PageScroll>
      <div className="max-w-2xl space-y-3 p-3">
        <Card
          title="Export player assets"
          subtitle="Portraits, head meshes and head textures, named by the player they belong to."
        >
          <div className="space-y-4 text-sm">
            <Field label="Which players">
              <div className="space-y-1">
                {SCOPES.map((s) => (
                  <label key={s.key} className="flex cursor-pointer items-center gap-2 text-slate-300">
                    <input
                      type="radio"
                      name="scope"
                      checked={scope === s.key}
                      onChange={() => setScope(s.key)}
                    />
                    <span>{s.label}</span>
                    <span className="text-xs text-slate-500">— {s.hint}</span>
                  </label>
                ))}
              </div>
            </Field>

            <Field label="What to write">
              <div className="space-y-1">
                <Check label="Portraits (.png)" checked={portraits} onChange={setPortraits} />
                <Check
                  label="Head meshes (.glb)"
                  checked={meshes}
                  onChange={setMeshes}
                  hint="Includes the skeleton, joints and skin weights."
                />
                <Check
                  label="Head textures (.dds)"
                  checked={textures}
                  onChange={setTextures}
                  hint="One file per map — colour, normal, and so on."
                />
                <Check
                  label="Original .rx2 files alongside"
                  checked={raw}
                  onChange={setRaw}
                  hint="Archives the untouched source bytes, so later compatibility work never has to re-extract."
                />
                <Check
                  label="Crop portraits to the subject"
                  checked={crop}
                  onChange={setCrop}
                  hint="EA ships portraits on a mostly-empty 512×512 canvas. Leave off to keep them exactly as shipped."
                />
              </div>
            </Field>

            <div className="flex items-center gap-3 border-t border-slate-800 pt-3">
              <Button onClick={() => run.mutate()} disabled={run.isPending || selected.length === 0}>
                {run.isPending ? "Exporting…" : `Export ${selected.length} players…`}
              </Button>
              <p className="text-xs text-slate-500">
                {selected.length} of {all.length} players
                {noSlug > 0 ? ` · ${noSlug} have no birthdate, so they can't be named` : ""}
              </p>
            </div>

            {run.error ? <ErrorBanner title="Export failed" message={String(run.error)} /> : null}
          </div>
        </Card>

        {summary ? <Summary summary={summary} /> : null}

        <CatalogExport projectId={projectId} rootArgs={rootArgs} />
      </div>
    </PageScroll>
  );
}

// Appearance catalog — every head + hair + facial hair (players and non-players),
// written as GLB + PNG with a catalog.json manifest the Appearance Studio reads.
function CatalogExport({
  projectId,
  rootArgs,
}: {
  projectId: number;
  rootArgs: AssetRootsArgsDto;
}) {
  const [textures, setTextures] = useState(true);
  const [summary, setSummary] = useState<HeadCatalogSummaryDto | null>(null);

  const run = useMutation({
    mutationFn: async () => {
      const outDir = await openDialog({
        multiple: false,
        directory: true,
        title: "Choose a folder to build the appearance catalog into",
      });
      if (typeof outDir !== "string") return null;
      return call("export_head_catalog", { projectId, roots: rootArgs, outDir, textures });
    },
    onSuccess: (s) => s && setSummary(s),
  });

  return (
    <Card
      title="Appearance catalog"
      subtitle="Every head, hairstyle and facial-hair mesh — players, generics, referees, coaches and trainers — with a catalog.json the Appearance Studio reads to mix any combination."
    >
      <div className="space-y-4 text-sm">
        <Check
          label="Player-head textures (.png)"
          checked={textures}
          onChange={setTextures}
          hint="Decode each bespoke player's head texlib to PNG alongside the meshes."
        />
        <div className="flex items-center gap-3 border-t border-slate-800 pt-3">
          <Button onClick={() => run.mutate()} disabled={run.isPending}>
            {run.isPending ? "Building catalog…" : "Build appearance catalog…"}
          </Button>
          <p className="text-xs text-slate-500">
            Captures bespoke, generic and staff heads across the whole install.
          </p>
        </div>
        {run.error ? <ErrorBanner title="Catalog build failed" message={String(run.error)} /> : null}

        {summary ? (
          <div className="border-t border-slate-800 pt-3">
            <div className="grid grid-cols-4 gap-2">
              <Stat label="Heads" value={summary.heads_written} />
              <Stat label="Hair" value={summary.hair_written} />
              <Stat label="Facial hair" value={summary.facial_hair_written} />
              <Stat label="Textures" value={summary.textures_written} />
            </div>
            <p className="mt-2 text-xs text-slate-500">
              {summary.entry_count.toLocaleString()} assets ·{" "}
              {formatBytes(summary.bytes_written)}
              {summary.manifest_file ? ` · ${summary.manifest_file}` : ""}
            </p>
            {summary.failures.length > 0 ? (
              <details className="mt-2 text-xs">
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
              <p className="mt-2 text-xs text-slate-500">No failures.</p>
            )}
          </div>
        ) : null}
      </div>
    </Card>
  );
}

function Summary({ summary }: { summary: AssetExportSummaryDto }) {
  return (
    <Card title="Last export" subtitle={`${formatBytes(summary.bytes_written)} written`}>
      <div className="grid grid-cols-4 gap-2">
        <Stat label="Portraits" value={summary.portraits_written} />
        <Stat label="Meshes" value={summary.meshes_written} />
        <Stat label="Textures" value={summary.textures_written} />
        <Stat label="Source .rx2" value={summary.raw_copied} />
      </div>
      {summary.portraits_skipped_generic > 0 && (
        <p className="mt-2 text-xs text-slate-500">
          {summary.portraits_skipped_generic.toLocaleString()} generic
          placeholder{summary.portraits_skipped_generic === 1 ? "" : "s"} skipped
          (shared blank portraits, not a player's own).
        </p>
      )}
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

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div>
      <p className="mb-1 text-xs font-semibold uppercase tracking-wide text-slate-500">{label}</p>
      {children}
    </div>
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

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`;
}
