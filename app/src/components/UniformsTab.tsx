// Team Uniforms tab — the merged jersey/uniform surface. A sidebar lists the team's uniform kits
// (from the `xjgw` jersey registry, unioned with any kits found only in the art tree); selecting a
// kit shows its editable DB metadata and its textures — jersey body, number font, pants, socks, and
// the nameplate template — decoded lazily so only the chosen kit's art is loaded. Paintable textures
// offer Replace (the before/after → write loop); nameplate sheets offer the name/number/patch
// location editor. Writes target the extracted file in place; the operator repacks/installs their way.

import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useAssetRootArgs } from "../lib/project";
import { EmptyState, Spinner } from "./primitives";
import { TextureThumbnail, useTextureReplace } from "./TextureReplace";
import { NameplateEditor } from "./NameplateEditor";
import { JerseyMetadataCard } from "./JerseyMetadataCard";
import { TeamKitViewer } from "./KitMeshViewer";
import { EquipmentDetail } from "./EquipmentTab";
import { equipmentTypeLabel } from "../lib/teamPresentation";
import type { TeamEquipmentDto } from "../bindings/TeamEquipmentDto";
import { jerseyStyleLabel, locStringLabel } from "../lib/teamPresentation";
import type { OutfitTextureDto } from "../bindings/OutfitTextureDto";
import type { TeamJerseyDto } from "../bindings/TeamJerseyDto";

/** Display order + heading for each outfit piece within a kit. */
const PIECES: { key: string; label: string }[] = [
  { key: "jersey", label: "Jersey body" },
  { key: "font", label: "Numbers (font)" },
  { key: "name", label: "Nameplate — name / number / patch locations" },
  { key: "pant", label: "Pants" },
  { key: "sock", label: "Socks" },
];

/** A uniform kit: a `(style, variant)` pair, its `xjgw` row (for metadata, if any), and its textures. */
interface Kit {
  key: string;
  style: number;
  variant: number;
  jersey?: TeamJerseyDto;
  textures: OutfitTextureDto[];
}

const OTHER_KEY = "__other__";

/** The `(style, variant)` a texture file belongs to: the outfit files are `PREFIX_{style}_{team}_…_
 *  {variant}[_cm].rx2`; drop the prefix word and any `_cm`, then style is the first number and variant
 *  the last. Matches a kit without relying on the team-id token (art id vs. db id ambiguity). */
function outfitStyleVariant(fileName: string): { style: number; variant: number } | null {
  const stem = fileName.replace(/\.rx2$/i, "").replace(/_cm$/i, "");
  const toks = stem.split("_");
  toks.shift(); // prefix word: texlib / name / jersey / pant / sock
  const nums = toks.map((t) => Number(t)).filter((n) => Number.isFinite(n));
  if (nums.length < 3) return null; // need style, team, variant
  return { style: nums[0], variant: nums[nums.length - 1] };
}

export function UniformsTab({ projectId, teamId }: { projectId: number; teamId: number }) {
  const { installDir, rootArgs } = useAssetRootArgs();
  const wide = useWideViewport();

  const jerseys = useQuery({
    queryKey: ["jerseys", projectId, teamId],
    queryFn: () => call("list_team_jerseys", { projectId, teamId }),
  });
  const outfits = useQuery({
    queryKey: ["teamOutfits", projectId, teamId, installDir],
    queryFn: () => call("team_outfits", { projectId, roots: rootArgs, teamId }),
    enabled: !!installDir,
    staleTime: Infinity,
  });

  const replace = useTextureReplace(installDir, () => void outfits.refetch());
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [zoom, setZoom] = useState<OutfitTextureDto | null>(null);
  const [nameplate, setNameplate] = useState<{ tex: OutfitTextureDto; kit: Kit } | null>(null);

  // Build the kit list: union of DB jersey rows and (style, variant) pairs found in the art tree.
  const kits = useMemo<Kit[]>(() => {
    const map = new Map<string, Kit>();
    const ensure = (key: string, style: number, variant: number) => {
      let k = map.get(key);
      if (!k) {
        k = { key, style, variant, textures: [] };
        map.set(key, k);
      }
      return k;
    };
    for (const j of jerseys.data ?? []) {
      const k = ensure(`${j.style_raw}_${j.variant_raw}`, j.style_raw, j.variant_raw);
      if (!k.jersey) k.jersey = j;
    }
    for (const t of outfits.data ?? []) {
      const sv = outfitStyleVariant(t.file_name);
      if (sv) ensure(`${sv.style}_${sv.variant}`, sv.style, sv.variant).textures.push(t);
      else ensure(OTHER_KEY, Infinity, Infinity).textures.push(t); // unparseable → catch-all
    }
    return [...map.values()].sort((a, b) => a.style - b.style || a.variant - b.variant);
  }, [jerseys.data, outfits.data]);

  // Keep a valid selection.
  useEffect(() => {
    if (kits.length === 0) {
      setSelectedKey(null);
    } else if (selectedKey === null || !kits.some((k) => k.key === selectedKey)) {
      setSelectedKey(kits[0].key);
    }
  }, [kits, selectedKey]);

  const current = kits.find((k) => k.key === selectedKey) ?? kits[0];

  // The previewed player, shared by the 3D viewer and the equipment panel so both
  // react to the same pick. null = the backend's top-overall default; we mirror
  // that here (top-overall active on the team) to know whether it's a goalie —
  // which swaps the equipment cards from skater gloves/helmet to blocker/trapper/
  // pads. Reset to the default whenever the kit changes. (These hooks live above
  // the early returns so hook order stays stable.)
  const [playerId, setPlayerId] = useState<number | null>(null);
  useEffect(() => setPlayerId(null), [current?.key]);
  const roster = useQuery({
    queryKey: ["players", projectId],
    queryFn: () => call("list_players", { projectId }),
  });
  const teamPlayers = useMemo(
    () => (roster.data ?? []).filter((p) => p.team_id === teamId),
    [roster.data, teamId],
  );
  const selectedPlayer = useMemo(() => {
    if (playerId != null) return teamPlayers.find((p) => p.player_id === playerId);
    // Mirror the backend default: top-overall active player.
    return teamPlayers
      .filter((p) => p.is_active !== false)
      .reduce<(typeof teamPlayers)[number] | undefined>(
        (best, p) => (best == null || (p.overall ?? -1) > (best.overall ?? -1) ? p : best),
        undefined,
      );
  }, [teamPlayers, playerId]);
  const previewIsGoalie = selectedPlayer?.is_goalie ?? false;

  if (jerseys.isLoading) return <Spinner label="Loading kits…" />;
  if (jerseys.isError) return <p className="text-sm text-rose-400">{String(jerseys.error)}</p>;

  if (kits.length === 0) {
    return (
      <EmptyState
        title="No uniform kits for this team"
        hint={
          installDir
            ? "This title registers no jerseys and ships no team-keyed uniform textures for this team."
            : "This project has no install tree, so only jersey metadata would show — and this team has no registered jerseys."
        }
      />
    );
  }

  const showPreview =
    !!installDir && Number.isFinite(current.style) && Number.isFinite(current.variant);

  // The kit's texture previews — a 3-column grid that lives on the right when the
  // window is wide (≥75% of the monitor), or below the preview when narrower.
  const texturesPanel = (
    <div className="min-w-0 space-y-3">
      {replace.result}
      {!installDir ? (
        <p className="rounded-md border border-slate-800 bg-slate-900/40 px-3 py-2 text-xs text-slate-400">
          No install tree for this project — jersey metadata and equipment are editable, but textures
          need a root-folder project to preview and replace.
        </p>
      ) : outfits.isLoading ? (
        <Spinner label="Finding this kit's textures…" />
      ) : (
        <KitTextures
          installDir={installDir}
          kit={current}
          busyRel={replace.pendingRel}
          onZoom={setZoom}
          onReplace={(t) => void replace.pick(t.rel_path, t.label, t.texture_index)}
          onEditLocations={(t) => setNameplate({ tex: t, kit: current })}
        />
      )}
    </div>
  );

  return (
    <div className="flex flex-col gap-3">
      {/* Header — kit selector + metadata, spanning the full width. */}
      <div className="flex flex-wrap items-start gap-3">
        <KitDropdown kits={kits} selectedKey={current.key} onSelect={setSelectedKey} />
        {current.jersey ? (
          <JerseyMetadataCard projectId={projectId} record={current.jersey} />
        ) : null}
      </div>

      {/* Body — preview (left, tall) · equipment customization (middle) · textures
          (right when wide, below when narrow). */}
      <div className="flex flex-col gap-3">
        <div className="flex flex-wrap items-start gap-3">
          <section className="h-[72vh] min-h-[34rem] min-w-[30rem] flex-1 basis-[40rem]">
            {showPreview ? (
              <TeamKitViewer
                projectId={projectId}
                roots={rootArgs}
                teamId={teamId}
                style={current.style}
                variant={current.variant}
                playerId={playerId}
                onPlayerIdChange={setPlayerId}
              />
            ) : (
              <p className="rounded-md border border-slate-800 bg-slate-900/40 px-3 py-2 text-xs text-slate-400">
                No install tree for this project — open a root-folder project to preview the kit in
                3D.
              </p>
            )}
          </section>

          <section className="min-w-[22rem] flex-1 basis-[26rem]">
            <KitEquipment
              projectId={projectId}
              teamId={teamId}
              variant={current.variant}
              isGoalie={previewIsGoalie}
            />
          </section>

          {wide ? (
            <section className="min-w-[20rem] flex-1 basis-[24rem]">{texturesPanel}</section>
          ) : null}
        </div>

        {!wide ? texturesPanel : null}
      </div>

      {replace.dialog}

      {zoom && installDir ? (
        <ZoomLightbox installDir={installDir} outfit={zoom} onClose={() => setZoom(null)} />
      ) : null}

      {nameplate && installDir ? (
        <NameplateEditorForKit
          projectId={projectId}
          teamId={teamId}
          installDir={installDir}
          tex={nameplate.tex}
          kit={nameplate.kit}
          onClose={() => setNameplate(null)}
          onSaved={() => void outfits.refetch()}
        />
      ) : null}
    </div>
  );
}

/** Equipment customization linked to the selected kit — the team's glove and
 *  helmet records (from `hHbv`), picked by the kit's variant (fallback: the
 *  first of each type). Zone-color / info edits go through the shared
 *  `EquipmentDetail` editor and invalidate the queries, so the 3D preview (which
 *  paints gloves/helmet from these same zone colors) updates live. Replaces the
 *  former standalone Equipment tab + its record selector. */
/** The equipment types shown for each player kind, in display order. Goalie types
 *  are a FLAGGED best-guess (must match GOALIE_TYPE_* in registry.rs). */
const SKATER_EQUIPMENT: { type: number; title: string }[] = [
  { type: 5, title: "Gloves" },
  { type: 8, title: "Helmet" },
];
const GOALIE_EQUIPMENT: { type: number; title: string }[] = [
  { type: 1, title: "Blocker" },
  { type: 2, title: "Trapper" },
  { type: 3, title: "Pads" },
];

function KitEquipment({
  projectId,
  teamId,
  variant,
  isGoalie,
}: {
  projectId: number;
  teamId: number;
  variant: number;
  /** The previewed player is a goalie → show blocker/trapper/pads instead of glove/helmet. */
  isGoalie: boolean;
}) {
  const q = useQuery({
    queryKey: ["equipment", projectId, teamId],
    queryFn: () => call("list_team_equipment", { projectId, teamId }),
  });

  const pick = (type: number): TeamEquipmentDto | undefined => {
    const ofType = (q.data ?? []).filter((r) => r.equipment_type_raw === type);
    return ofType.find((r) => r.variant_raw === variant) ?? ofType[0];
  };
  const wanted = isGoalie ? GOALIE_EQUIPMENT : SKATER_EQUIPMENT;
  const groups = wanted
    .map((w) => ({ ...w, record: pick(w.type) }))
    .filter((g): g is typeof g & { record: TeamEquipmentDto } => g.record != null);

  if (q.isLoading) return <Spinner label="Loading equipment…" />;
  if (q.isError) return <p className="text-sm text-rose-400">{String(q.error)}</p>;

  return (
    <div className="space-y-4">
      <div className="flex items-baseline justify-between">
        <h3 className="text-sm font-semibold text-slate-100">
          {isGoalie ? "Goalie equipment" : "Equipment"}
        </h3>
        <span className="text-[10px] uppercase tracking-wide text-slate-500">linked to kit</span>
      </div>
      {groups.length > 0 ? (
        groups.map((g) => (
          <EquipmentGroup key={g.type} title={g.title} projectId={projectId} record={g.record} />
        ))
      ) : (
        <EmptyState
          title={`No ${isGoalie ? "goalie" : "glove or helmet"} equipment registered`}
          hint={
            isGoalie
              ? "This team's hHbv table carries no blocker (type 1), trapper (type 2) or pads (type 3) rows."
              : "This team's hHbv table carries no gloves (type 5) or helmet (type 8) rows."
          }
        />
      )}
    </div>
  );
}

/** One equipment type's editable card (info + zone colors), titled by type. */
function EquipmentGroup({
  title,
  projectId,
  record,
}: {
  title: string;
  projectId: number;
  record: TeamEquipmentDto;
}) {
  return (
    <div>
      <div className="mb-1 flex items-baseline gap-2">
        <span className="text-xs font-semibold text-slate-200">{title}</span>
        <span className="text-[10px] text-slate-500">
          {equipmentTypeLabel(record.equipment_type_raw)} · #{record.record} · v{record.variant_raw}
        </span>
      </div>
      <EquipmentDetail projectId={projectId} record={record} />
    </div>
  );
}

/** True when the app window is at least 75% of the monitor's width — the
 *  breakpoint at which the texture previews move to their own right-hand column
 *  (otherwise they stack below the preview). Recomputed on resize. */
function useWideViewport(): boolean {
  const [wide, setWide] = useState(
    () => typeof window !== "undefined" && window.innerWidth >= window.screen.width * 0.75,
  );
  useEffect(() => {
    const onResize = () => setWide(window.innerWidth >= window.screen.width * 0.75);
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);
  return wide;
}

function kitLabel(k: Kit): string {
  if (k.key === OTHER_KEY) return "Other textures";
  if (k.jersey) {
    const loc = locStringLabel(k.jersey.loc_string_raw);
    return `${jerseyStyleLabel(k.jersey.style_raw)}${loc ? ` · ${loc}` : ""}`;
  }
  return `Style ${k.style} · Variant ${k.variant}`;
}

/** Kit selector — a dropdown attached to the top of the metadata view. Each
 *  option shows the kit label, a default marker, and its style/variant tokens. */
function KitDropdown({
  kits,
  selectedKey,
  onSelect,
}: {
  kits: Kit[];
  selectedKey: string;
  onSelect: (key: string) => void;
}) {
  return (
    <div className="w-72 shrink-0 rounded-lg border border-slate-800 bg-slate-900/40 p-3">
      <label className="flex flex-col gap-1">
        <span className="flex items-baseline justify-between">
          <span className="text-[11px] font-semibold uppercase tracking-wide text-slate-400">
            Kit
          </span>
          <span className="text-[10px] text-slate-500">
            {kits.length} {kits.length === 1 ? "kit" : "kits"}
          </span>
        </span>
        <select
          value={selectedKey}
          onChange={(e) => onSelect(e.target.value)}
          className="w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1.5 text-xs font-medium text-slate-100 focus:border-sky-600 focus:outline-none"
        >
          {kits.map((k) => (
            <option key={k.key} value={k.key}>
              {kitLabel(k)}
              {k.jersey?.is_default ? " · default" : ""}
              {k.key !== OTHER_KEY ? ` (style ${k.style} · var ${k.variant})` : ""}
            </option>
          ))}
        </select>
      </label>
    </div>
  );
}

function KitTextures({
  installDir,
  kit,
  busyRel,
  onZoom,
  onReplace,
  onEditLocations,
}: {
  installDir: string;
  kit: Kit;
  busyRel: string | null;
  onZoom: (t: OutfitTextureDto) => void;
  onReplace: (t: OutfitTextureDto) => void;
  onEditLocations: (t: OutfitTextureDto) => void;
}) {
  const grouped = useMemo(() => {
    const m = new Map<string, OutfitTextureDto[]>();
    for (const t of kit.textures) {
      const list = m.get(t.piece) ?? [];
      list.push(t);
      m.set(t.piece, list);
    }
    return m;
  }, [kit]);

  if (kit.textures.length === 0) {
    return (
      <p className="rounded-md border border-dashed border-slate-800 px-3 py-6 text-center text-xs text-slate-500">
        No textures found in the art tree for this kit.
      </p>
    );
  }

  // Any piece not in the known order (defensive) still renders, after the known ones.
  const order = [...PIECES.map((p) => p.key), ...[...grouped.keys()].filter((k) => !PIECES.some((p) => p.key === k))];

  return (
    <div className="space-y-4">
      <p className="text-xs text-slate-500">
        Click a texture to zoom. Replace writes the edited image into the extracted file in place
        (keeping a <code>.bak</code>); run it through your own repack/install step to see it in-game.
      </p>
      {order
        .filter((key) => grouped.has(key))
        .map((key) => {
          const heading = PIECES.find((p) => p.key === key)?.label ?? key;
          return (
            <section key={key}>
              <h3 className="mb-2 text-xs font-semibold uppercase tracking-wide text-slate-400">
                {heading}
              </h3>
              <div className="grid grid-cols-3 gap-3">
                {grouped.get(key)!.map((t) => (
                  <OutfitCard
                    key={`${t.rel_path}#${t.texture_index}`}
                    installDir={installDir}
                    outfit={t}
                    busy={busyRel === t.rel_path}
                    onZoom={() => onZoom(t)}
                    onReplace={() => onReplace(t)}
                    onEditLocations={t.is_nameplate ? () => onEditLocations(t) : undefined}
                  />
                ))}
              </div>
            </section>
          );
        })}
    </div>
  );
}

function OutfitCard({
  installDir,
  outfit,
  busy,
  onZoom,
  onReplace,
  onEditLocations,
}: {
  installDir: string;
  outfit: OutfitTextureDto;
  busy: boolean;
  onZoom: () => void;
  onReplace: () => void;
  onEditLocations?: () => void;
}) {
  return (
    <div className="flex flex-col gap-2 rounded-lg border border-slate-800 bg-slate-900/50 p-2">
      <button type="button" onClick={onZoom} title="Zoom" className="block">
        <TextureThumbnail
          installDir={installDir}
          relPath={outfit.rel_path}
          textureIndex={outfit.texture_index}
          maxEdge={512}
          className="aspect-square h-auto w-full cursor-zoom-in"
        />
      </button>
      <p className="min-w-0 truncate text-xs font-medium text-slate-200" title={outfit.file_name}>
        {outfit.label}
      </p>
      {onEditLocations ? (
        <div className="flex flex-col gap-1">
          <button
            type="button"
            onClick={onEditLocations}
            className="rounded border border-sky-700/50 bg-sky-900/40 px-2 py-1 text-xs font-medium text-sky-100 transition-colors hover:bg-sky-900/70"
            title="Edit where the name, number and patches sit on the jersey"
          >
            Edit locations…
          </button>
          <button
            type="button"
            onClick={onReplace}
            disabled={busy}
            className="text-[11px] text-slate-400 hover:text-slate-200 disabled:text-slate-600"
            title="Replace the raw sheet image (advanced)"
          >
            Replace raw sheet…
          </button>
        </div>
      ) : (
        <button
          type="button"
          onClick={onReplace}
          disabled={busy}
          className="rounded border border-slate-700 px-2 py-1 text-xs font-medium text-sky-400 transition-colors hover:bg-slate-800 hover:text-sky-300 disabled:text-slate-600"
          title="Preview and write an edited image into this texture (keeps a .bak)"
        >
          Replace…
        </button>
      )}
    </div>
  );
}

/** Open the nameplate editor with the kit's jersey body as the alignment
 *  reference and its number `font` map, so the live preview can render the
 *  selected player's name/number through the real font atlas. */
function NameplateEditorForKit({
  projectId,
  teamId,
  installDir,
  tex,
  kit,
  onClose,
  onSaved,
}: {
  projectId: number;
  teamId: number;
  installDir: string;
  tex: OutfitTextureDto;
  kit: Kit;
  onClose: () => void;
  onSaved: () => void;
}) {
  const body = kit.textures.find((t) => t.piece === "jersey");
  const font = kit.textures.find((t) => t.piece === "font");
  return (
    <NameplateEditor
      projectId={projectId}
      teamId={teamId}
      installDir={installDir}
      relPath={tex.rel_path}
      textureIndex={tex.texture_index}
      label={tex.label}
      referenceRelPath={body?.rel_path ?? null}
      referenceTextureIndex={body?.texture_index}
      fontRelPath={font?.rel_path ?? null}
      fontTextureIndex={font?.texture_index}
      kitStyle={Number.isFinite(kit.style) ? kit.style : null}
      kitVariant={Number.isFinite(kit.variant) ? kit.variant : null}
      onClose={onClose}
      onSaved={onSaved}
    />
  );
}

/** Full-size zoom of a texture, decoded via `texture_preview`. */
function ZoomLightbox({
  installDir,
  outfit,
  onClose,
}: {
  installDir: string;
  outfit: OutfitTextureDto;
  onClose: () => void;
}) {
  const preview = useQuery({
    queryKey: ["texturePreview", installDir, outfit.rel_path, 0, outfit.texture_index],
    queryFn: () =>
      call("texture_preview", {
        installDir,
        relPath: outfit.rel_path,
        maxEdge: 0,
        textureIndex: outfit.texture_index,
      }),
    staleTime: Infinity,
    retry: false,
  });
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 p-6" onClick={onClose}>
      <div
        className="flex max-h-full max-w-3xl flex-col overflow-hidden rounded-lg border border-slate-700 bg-slate-900 shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <header className="flex items-center justify-between gap-4 border-b border-slate-800 px-4 py-2">
          <div className="min-w-0">
            <p className="truncate text-sm font-medium text-slate-100">{outfit.label}</p>
            <p className="truncate text-[11px] text-slate-500" title={outfit.file_name}>
              {outfit.file_name}
              {preview.data ? ` · ${preview.data.width}×${preview.data.height}` : ""}
            </p>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="shrink-0 rounded border border-slate-700 px-2 py-1 text-xs text-slate-200 hover:bg-slate-800"
          >
            Close
          </button>
        </header>
        <div
          className="grid min-h-[240px] place-items-center overflow-auto p-4"
          style={{
            backgroundImage: "repeating-conic-gradient(#1e293b 0% 25%, #0f172a 0% 50%)",
            backgroundSize: "16px 16px",
          }}
        >
          {preview.isLoading ? (
            <Spinner label="Decoding…" />
          ) : preview.error ? (
            <p className="text-xs text-rose-400">{String(preview.error)}</p>
          ) : preview.data ? (
            <img
              src={preview.data.data_url}
              alt={outfit.label}
              className="max-h-[70vh] max-w-full object-contain"
              style={{ imageRendering: "pixelated" }}
            />
          ) : null}
        </div>
      </div>
    </div>
  );
}
