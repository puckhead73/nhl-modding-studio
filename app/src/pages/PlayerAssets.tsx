import { assetRoot } from "../lib/assetRoot";
// Player Assets — browse the art the game ships for each player: the front-end portrait, the
// head mesh, and the head's textures. Master list; the preview lives in `PlayerAssetDetail`.
//
// Two facts from the data shape this page (see `studio-domain`'s `assets` module):
//
//  * Portraits and head models are DIFFERENT sets. On the Xbox 360 anchor 4259 of 5632 players
//    have a portrait but only 246 have their own head model, so one "has assets" filter would
//    mislead. The filter chips name each set explicitly, with live counts.
//  * A head model is only *this player's* when `pPaD == lOnr`. Everyone else shares a generic
//    pool head, which the backend refuses to attribute to them — so those rows read "pool",
//    not a checkmark.

import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { PlayerContainmentCard } from "../components/PlayerContainmentCard";
import { useProject } from "../lib/project";
import { EntityList, type Column } from "../components/EntityList";
import { EmptyState, SelectPrompt } from "../components/primitives";
import { EXTRACTION_GATE_REASON } from "../components/GateHint";
import type { PlayerAssetsDto } from "../bindings/PlayerAssetsDto";

type Filter = "all" | "portrait" | "head" | "exportable";

const FILTERS: { key: Filter; label: string; hint: string }[] = [
  { key: "all", label: "All", hint: "Every player in the database" },
  { key: "portrait", label: "Portrait", hint: "A front-end portrait ships for this player" },
  { key: "head", label: "Own head", hint: "A head model authored for this player alone" },
  {
    key: "exportable",
    label: "Exportable",
    hint: "Has art, and a name + birthdate to build an export filename from",
  },
];

/** A player is exportable when they have art *and* a slug to name the files by. */
export function isExportable(p: PlayerAssetsDto): boolean {
  return !!p.slug && (!!p.portrait_path || !!p.mesh_path || p.texture_paths.length > 0);
}

/** Shared by this page and the Export page so both scan under the same cache key. */
export function usePlayerAssets() {
  const { project, install } = useProject();
  const projectId = project!.project_id;
  const installDir = assetRoot(install) ?? null;

  // `platform` drives the on-disk art layout server-side (RenderWare vs the PS4
  // kettle tree), so it must travel with every asset call.
  const rootArgs = {
    install_dir: installDir!,
    platform: install?.platform ?? undefined,
    rendering_override: null,
    fe_override: null,
  };

  const roots = useQuery({
    queryKey: ["assetRoots", installDir],
    queryFn: () => call("asset_roots", { roots: rootArgs }),
    enabled: !!installDir,
  });

  const assets = useQuery({
    queryKey: ["playerAssets", projectId, installDir],
    queryFn: () => call("list_player_assets", { projectId, roots: rootArgs }),
    // Only scan once the art is actually on disk — otherwise every path probe misses and the
    // list is a wall of dashes that looks like a bug.
    enabled: !!installDir && !!roots.data?.complete,
  });

  return { projectId, installDir, rootArgs, roots, assets };
}

/** The "no install" / "not extracted yet" gates both Asset pages share. */
export function AssetGate({
  installDir,
  roots,
}: {
  installDir: string | null;
  roots: ReturnType<typeof usePlayerAssets>["roots"];
}) {
  if (!installDir) {
    return (
      <EmptyState
        title="No install folder for this project"
        hint="Choose a game source folder under Project Overview → Game source folders to find player art. Your edits stay in this project."
      />
    );
  }
  if (roots.data && !roots.data.complete) {
    return (
      <EmptyState
        title="This install's art isn't extracted yet"
        hint={
          <>
            Expected <code className="text-slate-300">{roots.data.rendering}</code> and{" "}
            <code className="text-slate-300">{roots.data.fe}</code>. Asset editing requires all of
            this install's <code>.big</code> archives to be extracted in place. {EXTRACTION_GATE_REASON}{" "}
            Database editing is independent of this. Note: mesh (head model) editing is only supported on Xbox 360.
          </>
        }
      />
    );
  }
  return null;
}

export function PlayerAssetsPage() {
  const { installDir, roots, assets } = usePlayerAssets();
  const [filter, setFilter] = useState<Filter>("exportable");

  const all = useMemo(() => assets.data ?? [], [assets.data]);
  const counts = useMemo(
    () => ({
      all: all.length,
      portrait: all.filter((p) => p.portrait_path).length,
      head: all.filter((p) => p.mesh_path).length,
      exportable: all.filter(isExportable).length,
    }),
    [all],
  );

  const rows = useMemo(() => {
    switch (filter) {
      case "portrait":
        return all.filter((p) => p.portrait_path);
      case "head":
        return all.filter((p) => p.mesh_path);
      case "exportable":
        return all.filter(isExportable);
      default:
        return all;
    }
  }, [all, filter]);

  const gate = AssetGate({ installDir, roots });
  if (gate) return gate;

  const columns: Column<PlayerAssetsDto>[] = [
    {
      key: "name",
      label: "Player",
      width: "1fr",
      sort: (r) => r.full_name,
      render: (r) => r.full_name || "—",
    },
    {
      key: "portrait",
      label: "Portrait",
      width: "60px",
      sort: (r) => (r.portrait_path ? 0 : 1),
      render: (r) => (r.portrait_path ? "✓" : "—"),
      cellClass: (r) => (r.portrait_path ? "text-emerald-400" : "text-slate-600"),
    },
    {
      key: "head",
      label: "Head",
      width: "60px",
      sort: (r) => (r.mesh_path ? 0 : r.pool_head || r.head_shared ? 1 : 2),
      render: (r) =>
        r.mesh_path ? "✓" : r.pool_head || r.head_shared ? "shared" : "—",
      cellClass: (r) => (r.mesh_path ? "text-emerald-400" : "text-slate-600"),
    },
    {
      key: "slug",
      label: "Export name",
      width: "1.5fr",
      sort: (r) => r.slug ?? "",
      render: (r) => r.slug ?? "no birthdate",
      cellClass: (r) => (r.slug ? "text-slate-400" : "text-amber-500/70"),
    },
  ];

  return (
    <EntityList
      title="Player Assets"
      rows={rows}
      getId={(r) => r.bio_record}
      columns={columns}
      searchOf={(r) => `${r.full_name} ${r.slug ?? ""}`}
      searchPlaceholder="Search players…"
      isLoading={assets.isLoading || roots.isLoading}
      error={assets.error ?? roots.error}
      total={all.length}
      initialSort="name"
      panelWidth="w-[30rem]"
      toolbar={
        <div className="flex items-center gap-1">
          {FILTERS.map((f) => (
            <button
              key={f.key}
              title={f.hint}
              onClick={() => setFilter(f.key)}
              className={`rounded px-2 py-0.5 text-xs ${
                filter === f.key
                  ? "bg-sky-600 text-white"
                  : "bg-slate-800 text-slate-400 hover:text-slate-200"
              }`}
            >
              {f.label}
              <span className="ml-1 opacity-60">{counts[f.key]}</span>
            </button>
          ))}
        </div>
      }
    />
  );
}

/** The index pane of the Player assets list (no player picked): the title-level portrait / head
 *  containment summary (Phase 3, 22 Sep 2026) above the pick prompt. Read-only. */
export function PlayerAssetsIndex() {
  const { projectId, installDir, rootArgs, roots } = usePlayerAssets();
  return (
    <div className="space-y-3 p-3">
      {installDir && roots.data?.complete ? (
        <PlayerContainmentCard projectId={projectId} rootArgs={rootArgs} installDir={installDir} />
      ) : null}
      <SelectPrompt what="player" />
    </div>
  );
}

