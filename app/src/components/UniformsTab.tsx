import { uniformColorMap } from "../lib/uniformTextures";
// Team Uniforms tab — the merged jersey/uniform surface. A sidebar lists the team's uniform kits
// (from the `xjgw` jersey registry, unioned with any kits found only in the art tree); selecting a
// kit shows its editable DB metadata and its textures — jersey body, number font, pants, socks, and
// the nameplate template — decoded lazily so only the chosen kit's art is loaded. Paintable textures
// offer Replace (the before/after → write loop); nameplate sheets offer the name/number/patch
// location editor. Writes target the extracted file in place; the operator repacks/installs their way.

import { useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useAssetRootArgs, useProject } from "../lib/project";
import { useSidebarCollapse } from "../lib/sidebarCollapse";
import { Button, EmptyState, Spinner } from "./primitives";
import type { AssetRootsArgsDto } from "../bindings/AssetRootsArgsDto";
import type { CloneJerseyResultDto } from "../bindings/CloneJerseyResultDto";
import type { CloneKitTexturesSummaryDto } from "../bindings/CloneKitTexturesSummaryDto";
import { TextureThumbnail, useTextureReplace } from "./TextureReplace";
import { NameplateEditor } from "./NameplateEditor";
import { JerseyMetadataCard } from "./JerseyMetadataCard";
import { TeamKitsCard } from "./TeamKitsCard";
import { TeamKitViewer } from "./KitMeshViewer";
import { EquipmentDetail } from "./EquipmentTab";
import { TeamGoalieMasksCard } from "./TeamGoalieMasksCard";
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
  textureLabel?: string;
  textures: OutfitTextureDto[];
}

const OTHER_KEY = "__other__";

/** PS4 names identify texture families, not measured jersey-registry identities. */
export function outfitNamedGroup(fileName: string): string | null {
  return /^(?:jersey|font|pant|sock|name)_(.+)_(?:cm|nm|sm|dm|tm)\.gnf$/i.exec(fileName)?.[1].toLowerCase() ?? null;
}

/** The `(style, variant)` a texture file belongs to: the outfit files are `PREFIX_{style}_{team}_…_
 *  {variant}[_cm].rx2` (or `.rpsgl`); drop the prefix word and map suffix, then style is the first number and variant
 *  the last. Matches a kit without relying on the team-id token (art id vs. db id ambiguity). */
export function outfitStyleVariant(fileName: string): { style: number; variant: number } | null {
  const stem = fileName.replace(/\.(?:rx2|rpsgl)$/i, "").replace(/_(?:cm|nm|sm|dm|tm)$/i, "");
  const toks = stem.split("_");
  toks.shift(); // prefix word: texlib / name / jersey / pant / sock
  const nums = toks.map((t) => Number(t)).filter((n) => Number.isFinite(n));
  if (nums.length < 3) return null; // need style, team, variant
  return { style: nums[0], variant: nums[nums.length - 1] };
}

/** What the third subwindow is editing — drives both the preview autofocus and
 *  which data/textures are shown. */
type KitPart = "jersey" | "helmet" | "gloves" | "pants" | "socks";

const KIT_PARTS: { key: KitPart; label: string }[] = [
  { key: "jersey", label: "Jersey" },
  { key: "helmet", label: "Helmet" },
  { key: "gloves", label: "Gloves" },
  { key: "pants", label: "Pants" },
  { key: "socks", label: "Socks" },
];

/** Preview piece kinds each part frames, in preference order (goalie kits swap
 *  in mask/trapper/blocker; the trapper is the catch hand — the "left hand"). */
const PART_FOCUS: Record<KitPart, string[]> = {
  jersey: ["jersey"],
  helmet: ["helmet", "mask"],
  gloves: ["gloves", "trapper", "blocker"],
  pants: ["pants", "pads"],
  socks: ["socks"],
};

export function UniformsTab({ projectId, teamId }: { projectId: number; teamId: number }) {
  const { installDir, rootArgs } = useAssetRootArgs();
  const [includeAmbiguous, setIncludeAmbiguous] = useState(false);

  // Entering the Uniforms tab hands the full width to the kit view — collapse
  // the team-select sidebar if it isn't already.
  const collapseSidebar = useSidebarCollapse();
  useEffect(() => {
    collapseSidebar(true);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const jerseys = useQuery({
    queryKey: ["jerseys", projectId, teamId],
    queryFn: () => call("list_team_jerseys", { projectId, teamId }),
  });
  const outfits = useQuery({
    queryKey: ["teamOutfits", projectId, teamId, installDir, rootArgs, includeAmbiguous],
    queryFn: () => call("team_outfits", { projectId, roots: rootArgs, teamId, ...(includeAmbiguous ? { includeAmbiguous: true } : {}) }),
    enabled: !!installDir,
    staleTime: Infinity,
    // Extraction or an external texture editor may change files while this tab is closed.
    refetchOnMount: "always",
  });

  const qc = useQueryClient();
  const replace = useTextureReplace(installDir, () => {
    void outfits.refetch();
    // The jersey-select thumbnail replaces through the same flow — refresh its cache too.
    void qc.invalidateQueries({ queryKey: ["jerseyThumb"] });
  });
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [addingJersey, setAddingJersey] = useState(false);
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
      const named = rootArgs.platform === "ps4" ? outfitNamedGroup(t.file_name) : null;
      if (named) {
        const kit = ensure(`gnf:${named}`, Infinity, Infinity);
        kit.textureLabel = `${named.replace(/_/g, " ")} (textures)`;
        kit.textures.push(t);
        continue;
      }
      const sv = outfitStyleVariant(t.file_name);
      if (sv) ensure(`${sv.style}_${sv.variant}`, sv.style, sv.variant).textures.push(t);
      else ensure(OTHER_KEY, Infinity, Infinity).textures.push(t); // unparseable → catch-all
    }
    return [...map.values()].sort((a, b) => a.style - b.style || a.variant - b.variant);
  }, [jerseys.data, outfits.data, rootArgs.platform]);

  // Keep a valid selection. Default to a *registered* kit (the team's default jersey first) —
  // for alternate teams sharing a parent club's art id, the file-derived kit list carries the
  // parent's other colorways too, and those shouldn't win the initial pick.
  useEffect(() => {
    // Wait for PS4 textures rather than selecting an empty numeric registration first.
    if (rootArgs.platform === "ps4" && outfits.isLoading) return;
    if (kits.length === 0) {
      setSelectedKey(null);
    } else if (selectedKey === null || !kits.some((k) => k.key === selectedKey)) {
      const preferred =
        (rootArgs.platform === "ps4" ? kits.find((k) => k.textureLabel) : undefined) ??
        kits.find((k) => k.jersey?.is_default) ?? kits.find((k) => k.jersey) ?? kits[0];
      setSelectedKey(preferred.key);
    }
  }, [kits, selectedKey, rootArgs.platform, outfits.isLoading]);

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
  // Membership-only teams (Winter Classic / all-star / national) have no main-team players —
  // their roster comes from the eGlu block instead.
  const rosterIds = useQuery({
    queryKey: ["teamRoster", projectId, teamId],
    queryFn: () => call("team_roster_ids", { projectId, teamId }),
  });
  const teamPlayers = useMemo(() => {
    const all = roster.data ?? [];
    const main = all.filter((p) => p.team_id === teamId);
    if (main.length > 0) return main;
    const ids = new Set(rosterIds.data ?? []);
    return all.filter((p) => ids.has(p.player_id));
  }, [roster.data, rosterIds.data, teamId]);
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

  // The third subwindow's selection: which part of the kit is being edited.
  // Autofocus only engages once the user actively picks a part (the initial
  // view frames the whole figure). Reset when the kit changes.
  const [part, setPart] = useState<KitPart>("jersey");
  const [partTouched, setPartTouched] = useState(false);
  useEffect(() => {
    setPart("jersey");
    setPartTouched(false);
  }, [current?.key]);

  if (jerseys.isLoading) return <Spinner label="Loading kits…" />;
  if (jerseys.isError) return <p className="text-sm text-rose-400">{String(jerseys.error)}</p>;
  if (installDir && outfits.isLoading && (kits.length === 0 || rootArgs.platform === "ps4")) return <Spinner label="Finding this team's uniform textures…" />;
  if (installDir && outfits.isError && kits.length === 0) return <p className="text-sm text-rose-400">Couldn't read uniform textures: {String(outfits.error)}</p>;

  const candidateControl = installDir && rootArgs.platform === "ps4" ? <div className="space-y-1 text-xs">
    <label className="flex items-center gap-2 text-slate-300">
      <input type="checkbox" checked={includeAmbiguous} onChange={e => { setIncludeAmbiguous(e.target.checked); setSelectedKey(null); }} />
      Show ambiguous filename matches
    </label>
    {includeAmbiguous && <p className="text-amber-200">These filenames may match several teams, including alumni teams. Inspect the texture before replacing it; this does not establish which team or kit uses it in game.</p>}
  </div> : null;

  if (kits.length === 0) {
    return (
      <div className="space-y-3">{candidateControl}<EmptyState
        title="No matched uniform kits"
        hint={
          installDir
            ? "No uniform assets were found for this team. Extract the game assets in Settings, or check that the correct assets folder is linked."
            : "Link a game assets folder to find uniform textures. Studio found no registered jersey rows for this team in the current database."
        }
      />{installDir && <Link to="/app/settings/general" className="inline-block rounded border border-sky-700 px-3 py-2 text-sm text-sky-300">Open asset extraction settings</Link>}</div>
    );
  }

  const nameTexture = current.textures.find(t => t.piece === "name");
  const showPreview =
    !!installDir && Number.isFinite(current.style) && Number.isFinite(current.variant);

  // Texture previews for a subset of the kit's pieces (the part-specific view).
  const texturesFor = (include: string[]) =>
    !installDir ? (
      <p className="rounded-md border border-slate-800 bg-slate-900/40 px-3 py-2 text-xs text-slate-400">
        No install tree for this project — jersey metadata and equipment are editable, but textures
        need a linked game assets folder to preview and replace.
      </p>
    ) : outfits.isLoading ? (
      <Spinner label="Finding this kit's textures…" />
    ) : (
      <KitTextures
        installDir={installDir}
        kit={current}
        include={include}
        busyRel={replace.pendingRel}
        onZoom={setZoom}
        onReplace={(t) => void replace.pick(t.rel_path, t.label, t.texture_index)}
        onEditLocations={(t) => setNameplate({ tex: t, kit: current })}
      />
    );

  // What the third subwindow shows for the selected part: equipment editors for
  // the moulded gear (helmet/gloves — goalie kits swap in blocker/trapper/pads),
  // part-specific textures for the cloth pieces.
  const partContent: Record<KitPart, React.ReactNode> = {
    jersey: texturesFor(["jersey", "font"]),
    helmet: (
      <div className="space-y-4">
        <KitEquipment
          projectId={projectId}
          teamId={teamId}
          variant={current.variant}
          wanted={[{ type: 8, title: "Helmet" }]}
          emptyHint="This team's hHbv table carries no helmet (type 8) rows."
        />
        {/* Equipment 4.5 G2 (receipt 42): the team's goalie mask paints (NHL 10 → PS4 21). */}
        <TeamGoalieMasksCard projectId={projectId} teamId={teamId} roots={rootArgs} />
      </div>
    ),
    gloves: (
      <KitEquipment
        projectId={projectId}
        teamId={teamId}
        variant={current.variant}
        wanted={
          previewIsGoalie
            ? [
                { type: 2, title: "Trapper" },
                { type: 1, title: "Blocker" },
              ]
            : [{ type: 5, title: "Gloves" }]
        }
        emptyHint={
          previewIsGoalie
            ? "This team's hHbv table carries no trapper (type 2) or blocker (type 1) rows."
            : "This team's hHbv table carries no gloves (type 5) rows."
        }
      />
    ),
    pants: (
      <div className="space-y-4">
        {previewIsGoalie && (
          <KitEquipment
            projectId={projectId}
            teamId={teamId}
            variant={current.variant}
            wanted={[{ type: 3, title: "Pads" }]}
            emptyHint="This team's hHbv table carries no goalie pads (type 3) rows."
          />
        )}
        {texturesFor(["pant"])}
      </div>
    ),
    socks: texturesFor(["sock"]),
  };

  return (
    <div className="flex flex-col gap-3">
      {candidateControl}
      <div className="flex flex-wrap items-start gap-3">
        {/* Subwindow 1 — the player preview, tall on the left. */}
        <section className={!showPreview
          ? "w-full min-w-0"
          : "min-w-[30rem] flex-1 basis-[42rem]"}>
          <div className={showPreview ? "h-[78vh] min-h-[34rem]" : undefined}>
          {showPreview ? (
            <TeamKitViewer
              projectId={projectId}
              roots={rootArgs}
              teamId={teamId}
              style={current.style}
              variant={current.variant}
              playerId={playerId}
              onPlayerIdChange={setPlayerId}
              focusKinds={partTouched ? PART_FOCUS[part] : null}
            />
          ) : (
            <p className="rounded-md border border-slate-800 bg-slate-900/40 px-3 py-2 text-xs text-slate-400">
              {installDir
                ? "These textures are available below. Their link to a registered 3D kit has not been verified."
                : "Link this project's extracted game folder to preview the kit in 3D."}
            </p>
          )}
          </div>
          <div className="mt-3"><Button disabled={!nameTexture} onClick={() => nameTexture && setNameplate({ tex: nameTexture, kit: current })}>Edit Jersey Layout</Button></div>
        </section>

        {/* Side column — subwindow 2 (kit details) stacked over subwindow 3 (kit editables). */}
        <div className="min-w-[26rem] flex-1 basis-[34rem] space-y-3">
          {/* Subwindow 2 — kit selection + kit-specific details. */}
          <section className="space-y-3 rounded-lg border border-slate-800 bg-slate-900/40 p-3">
            <div className="flex flex-wrap items-end gap-2">
              <div className="min-w-0 flex-1">
                <KitDropdown kits={kits} selectedKey={current.key} onSelect={setSelectedKey} />
              </div>
              <button
                type="button"
                onClick={() => setAddingJersey(true)}
                disabled={!current.jersey}
                title={
                  current.jersey
                    ? "Clone this kit into a new jersey registration"
                    : "Select a kit with a registered jersey row to clone from"
                }
                className="shrink-0 rounded-md border border-emerald-700/50 bg-emerald-900/30 px-3 py-1.5 text-xs font-medium text-emerald-100 transition-colors hover:bg-emerald-900/60 disabled:cursor-not-allowed disabled:opacity-50"
              >
                + Add New Jersey
              </button>
            </div>
            {current.jersey ? (
              <JerseyMetadataCard projectId={projectId} record={current.jersey} />
            ) : null}
            {current.jersey && installDir ? (
              <JerseySelectPreview
                rootArgs={rootArgs}
                jersey={current.jersey}
                busyRel={replace.pendingRel}
                onReplace={(relPath) => void replace.pick(relPath, "In-game jersey select preview")}
              />
            ) : null}
            {/* Phase 2.6 (22 Sep 2026): the measured kit → texture join, read-only. */}
            <TeamKitsCard
              projectId={projectId}
              teamId={teamId}
              rootArgs={rootArgs}
              installDir={installDir}
              style={Number.isFinite(current.style) ? current.style : null}
              variant={Number.isFinite(current.variant) ? current.variant : null}
              onReplaceLoose={(relPath, label) => void replace.pick(relPath, label)}
            />
          </section>

          {/* Subwindow 3 — what's being edited: part selector on top, part-specific
              data below. Switching parts swaps the content and re-frames the preview. */}
          <section className="rounded-lg border border-slate-800 bg-slate-900/40 p-3">
            <div
              role="tablist"
              aria-label="Kit part"
              className="mb-3 flex flex-wrap items-center gap-1 border-b border-slate-800 pb-2"
            >
              {KIT_PARTS.map((p) => {
                const on = p.key === part;
                return (
                  <button
                    key={p.key}
                    type="button"
                    role="tab"
                    aria-selected={on}
                    onClick={() => {
                      setPart(p.key);
                      setPartTouched(true);
                    }}
                    className={
                      "rounded-md px-3 py-1 text-xs font-semibold transition-colors " +
                      (on
                        ? "bg-sky-900/40 text-sky-100"
                        : "text-slate-400 hover:bg-slate-800/60 hover:text-slate-200")
                    }
                  >
                    {p.label}
                  </button>
                );
              })}
            </div>
            {replace.result}
            {partContent[part]}
          </section>
        </div>
      </div>

      {replace.dialog}

      {addingJersey && current.jersey ? (
        <AddJerseyDialog
          projectId={projectId}
          teamId={teamId}
          source={current.jersey}
          kits={kits}
          rootArgs={rootArgs}
          installDir={installDir}
          onClose={() => setAddingJersey(false)}
          onCreated={(style, variant) => setSelectedKey(`${style}_${variant}`)}
        />
      ) : null}

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

/** Equipment customization linked to the selected kit — the requested `hHbv` equipment types
 *  (picked by the kit's variant, falling back to the first of each type). Zone-color / info edits
 *  go through the shared `EquipmentDetail` editor and invalidate the queries, so the 3D preview
 *  (which paints the gear from these same zone colors) updates live. Goalie type codes 1/2/3
 *  (blocker/trapper/pads) are a FLAGGED best-guess (must match GOALIE_TYPE_* in registry.rs). */
function KitEquipment({
  projectId,
  teamId,
  variant,
  wanted,
  emptyHint,
}: {
  projectId: number;
  teamId: number;
  variant: number;
  /** The equipment types this part shows, in display order (e.g. `[{type: 8, title: "Helmet"}]`). */
  wanted: { type: number; title: string }[];
  emptyHint: string;
}) {
  const q = useQuery({
    queryKey: ["equipment", projectId, teamId],
    queryFn: () => call("list_team_equipment", { projectId, teamId }),
  });

  const pick = (type: number): TeamEquipmentDto | undefined => {
    const ofType = (q.data ?? []).filter((r) => r.equipment_type_raw === type);
    return ofType.find((r) => r.variant_raw === variant) ?? ofType[0];
  };
  const groups = wanted
    .map((w) => ({ ...w, record: pick(w.type) }))
    .filter((g): g is typeof g & { record: TeamEquipmentDto } => g.record != null);

  if (q.isLoading) return <Spinner label="Loading equipment…" />;
  if (q.isError) return <p className="text-sm text-rose-400">{String(q.error)}</p>;

  return (
    <div className="space-y-4">
      {groups.length > 0 ? (
        groups.map((g) => (
          <EquipmentGroup key={g.type} title={g.title} projectId={projectId} record={g.record} />
        ))
      ) : (
        <EmptyState title="No matching equipment registered" hint={emptyHint} />
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

function kitLabel(k: Kit): string {
  if (k.textureLabel) return k.textureLabel;
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
              {Number.isFinite(k.style) ? ` (style ${k.style} · var ${k.variant})` : ""}
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
  include,
  busyRel,
  onZoom,
  onReplace,
  onEditLocations,
}: {
  installDir: string;
  kit: Kit;
  /** Piece keys to show (the part-specific view); omitted = every piece. */
  include?: string[];
  busyRel: string | null;
  onZoom: (t: OutfitTextureDto) => void;
  onReplace: (t: OutfitTextureDto) => void;
  onEditLocations: (t: OutfitTextureDto) => void;
}) {
  const grouped = useMemo(() => {
    const m = new Map<string, OutfitTextureDto[]>();
    for (const t of kit.textures) {
      if (include && !include.includes(t.piece)) continue;
      const list = m.get(t.piece) ?? [];
      list.push(t);
      m.set(t.piece, list);
    }
    return m;
  }, [kit, include]);

  if (grouped.size === 0) {
    return (
      <p className="rounded-md border border-dashed border-slate-800 px-3 py-6 text-center text-xs text-slate-500">
        No textures found in the art tree for this part of the kit.
      </p>
    );
  }

  // Any piece not in the known order (defensive) still renders, after the known ones.
  const order = [...PIECES.map((p) => p.key), ...[...grouped.keys()].filter((k) => !PIECES.some((p) => p.key === k))];

  return (
    <div className="space-y-4">
      <p className="text-xs text-slate-500">
        Click a texture to zoom. In a project, replacements are staged for deployment.
        Review the destination and any partial-replacement notes before deploying to the game.
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
                    preview={t.is_nameplate ? uniformColorMap(kit.textures,"jersey") ?? null : t}
                    busy={busyRel === t.rel_path}
                    onZoom={() => {const image=t.is_nameplate?uniformColorMap(kit.textures,"jersey"):t;if(image)onZoom(image);}}
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
  preview,
  installDir,
  outfit,
  busy,
  onZoom,
  onReplace,
  onEditLocations,
}: {
  installDir: string;
  outfit: OutfitTextureDto;
  preview: OutfitTextureDto | null;
  busy: boolean;
  onZoom: () => void;
  onReplace: () => void;
  onEditLocations?: () => void;
}) {
  return (
    <div className="flex flex-col gap-2 rounded-lg border border-slate-800 bg-slate-900/50 p-2">
      <button type="button" onClick={onZoom} title="Zoom" className="block">
        {preview ? <TextureThumbnail
          installDir={installDir}
          relPath={preview.rel_path}
          textureIndex={preview.texture_index}
          maxEdge={512}
          className="aspect-square h-auto w-full cursor-zoom-in"
        /> : <span className="text-xs text-slate-400">Color map unavailable</span>}
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
  const body = uniformColorMap(kit.textures, "jersey");
  const font = uniformColorMap(kit.textures, "font");
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

/** The in-game jersey-select preview image (the FE thumbnail the game's jersey menu shows),
 *  resolved through the game's own fallback chain. The kit's own stock archive is replaceable
 *  through the shared texture-replace flow; the shared `custom*.big` fallbacks display but are
 *  never offered as a write target (they back every custom kit). */
function JerseySelectPreview({
  rootArgs,
  jersey,
  busyRel,
  onReplace,
}: {
  rootArgs: AssetRootsArgsDto;
  jersey: TeamJerseyDto;
  busyRel: string | null;
  onReplace: (relPath: string) => void;
}) {
  const thumb = useQuery({
    queryKey: [
      "jerseyThumb",
      rootArgs.install_dir,
      jersey.style_raw,
      jersey.team_art_id,
      jersey.variant_raw,
    ],
    queryFn: () =>
      call("team_jersey_thumbnail", {
        roots: rootArgs,
        style: jersey.style_raw,
        artId: jersey.team_art_id,
        variant: jersey.variant_raw,
      }),
    staleTime: Infinity,
    retry: false,
  });

  const assetId = `${jersey.style_raw}_${jersey.team_art_id}_${jersey.variant_raw}`;
  return (
    <div className="rounded-md border border-slate-800 bg-slate-950/40 p-2">
      <div className="mb-1 flex items-baseline justify-between gap-2">
        <span className="text-[11px] font-semibold uppercase tracking-wide text-slate-400">
          In-game jersey select preview
        </span>
        <span className="font-mono text-[10px] text-slate-600" title="FE thumbnail asset id">
          jersey_{assetId}.big
        </span>
      </div>
      <div className="flex items-start gap-3">
        <div
          className="grid h-28 w-28 shrink-0 place-items-center overflow-hidden rounded border border-slate-800"
          style={{
            backgroundImage: "repeating-conic-gradient(#1e293b 0% 25%, #0f172a 0% 50%)",
            backgroundSize: "16px 16px",
          }}
        >
          {thumb.isLoading ? (
            <Spinner label="" />
          ) : thumb.data?.data_url ? (
            <img
              src={thumb.data.data_url}
              alt="In-game jersey select preview"
              className="max-h-full max-w-full object-contain"
              style={{ imageRendering: "pixelated" }}
            />
          ) : (
            <span className="px-2 text-center text-[10px] text-slate-600">no FE thumbnail</span>
          )}
        </div>
        <div className="min-w-0 flex-1 space-y-1.5 text-[11px] text-slate-400">
          {thumb.data?.found ? (
            <p>
              {thumb.data.width}×{thumb.data.height}
              {!thumb.data.stock && (
                <span className="ml-2 rounded-full border border-amber-800 bg-amber-950/50 px-2 py-0.5 text-[10px] text-amber-300">
                  shared custom fallback
                </span>
              )}
            </p>
          ) : (
            <p>
              This title's FE tree ships no jersey-select image for this kit — the game falls back
              to its shared custom art.
            </p>
          )}
          {thumb.data?.rel_path ? (
            <button
              type="button"
              onClick={() => onReplace(thumb.data!.rel_path!)}
              disabled={busyRel === thumb.data.rel_path}
              className="rounded border border-slate-700 px-2 py-1 text-xs font-medium text-sky-400 transition-colors hover:bg-slate-800 hover:text-sky-300 disabled:text-slate-600"
              title="Preview and write an edited image into the jersey-select thumbnail (keeps a .bak)"
            >
              Replace…
            </button>
          ) : thumb.data?.found && !thumb.data.stock ? (
            <p className="text-slate-500">
              The shared fallback backs every custom kit, so it can't be overwritten from here —
              clone the kit's textures to give it its own stock archive.
            </p>
          ) : null}
        </div>
      </div>
    </div>
  );
}

/** "Add New Jersey" — clone the selected kit's `xjgw` registration into a new kit (the reference
 *  app's wizard, ported): the DB row is appended + copied + stamped with the new
 *  `{style}_{art}_{variant}` identity (staged as a pending, undoable edit), and optionally the
 *  kit's texture files are cloned onto the new asset id (same-length ids only — the containers
 *  reference the id in fixed-length name slots). */
function AddJerseyDialog({
  projectId,
  teamId,
  source,
  kits,
  rootArgs,
  installDir,
  onClose,
  onCreated,
}: {
  projectId: number;
  teamId: number;
  source: TeamJerseyDto;
  kits: Kit[];
  rootArgs: AssetRootsArgsDto;
  installDir: string | null;
  onClose: () => void;
  onCreated: (style: number, variant: number) => void;
}) {
  const qc = useQueryClient();
  const { update } = useProject();
  const [style, setStyle] = useState(source.style_raw);
  const [artId, setArtId] = useState(source.team_art_id);
  // "Last variant + 1" convention from the reference app (fall back to 0 past the 5-bit ceiling).
  const [variant, setVariant] = useState(source.variant_raw >= 31 ? 0 : source.variant_raw + 1);
  const [cloneTextures, setCloneTextures] = useState(true);
  const [result, setResult] = useState<{
    db: CloneJerseyResultDto;
    tex: CloneKitTexturesSummaryDto | null;
  } | null>(null);

  const srcId = `${source.style_raw}_${source.team_art_id}_${source.variant_raw}`;
  const dstId = `${style}_${artId}_${variant}`;
  const sameLen = srcId.length === dstId.length;
  const sameId = srcId === dstId;
  const taken = kits.some(
    (k) =>
      k.jersey &&
      k.jersey.style_raw === style &&
      k.jersey.team_art_id === artId &&
      k.jersey.variant_raw === variant,
  );
  const willCloneTextures = cloneTextures && !!installDir && sameLen && !sameId;

  const run = useMutation({
    mutationFn: async () => {
      const db = await call("clone_jersey", {
        projectId,
        sourceRecord: source.record,
        team: teamId,
        style,
        artId,
        variant,
      });
      let tex: CloneKitTexturesSummaryDto | null = null;
      if (willCloneTextures) {
        tex = await call("clone_kit_textures", {
          roots: rootArgs,
          srcStyle: source.style_raw,
          srcArtId: source.team_art_id,
          srcVariant: source.variant_raw,
          dstStyle: style,
          dstArtId: artId,
          dstVariant: variant,
        });
      }
      return { db, tex };
    },
    onSuccess: async (r) => {
      setResult(r);
      update(await call("project_summary", { projectId }));
      await qc.invalidateQueries();
      onCreated(style, variant);
    },
  });

  const num = (
    label: string,
    value: number,
    set: (v: number) => void,
    hint: string,
  ) => (
    <label className="block">
      <span className="mb-1 block text-[11px] font-medium text-slate-400" title={hint}>
        {label}
      </span>
      <input
        type="number"
        min={0}
        value={value}
        disabled={run.isPending || result != null}
        onChange={(e) => set(Math.max(0, Number(e.target.value) || 0))}
        className="w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1.5 font-mono text-sm text-slate-100 focus:border-sky-600 focus:outline-none"
      />
    </label>
  );

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/80 p-6 backdrop-blur-sm">
      <div className="w-full max-w-lg rounded-lg border border-slate-700 bg-slate-900 p-5 shadow-2xl">
        {result == null ? (
          <>
            <h2 className="mb-1 text-lg font-semibold text-slate-100">Add New Jersey</h2>
            <p className="mb-4 text-xs text-slate-400">
              Clones the selected kit — every registry value is copied onto a new jersey row, then
              stamped with the identity below. The row is staged as a pending, undoable edit.
            </p>

            <div className="mb-3 grid grid-cols-3 gap-3">
              {num("Style (cVLR)", style, setStyle, "0 = 90s/CCM/Retro cut, 1 = Reebok/Modern cut")}
              {num("Team Art ID (GDQL)", artId, setArtId, "The art id token in the kit's texture filenames")}
              {num("Variant (FccT)", variant, setVariant, "New kit's variant token (FE variant mirrors it)")}
            </div>

            <div className="mb-3 rounded-md border border-slate-800 bg-slate-950/50 px-3 py-2 text-xs">
              <div className="flex items-center justify-between">
                <span className="text-slate-400">New asset id</span>
                <span className="font-mono text-slate-100">{dstId}</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-slate-500">cloned from</span>
                <span className="font-mono text-slate-400">{srcId}</span>
              </div>
            </div>

            {sameId && (
              <p className="mb-2 text-xs text-rose-400">
                The new identity equals the source kit's — change the style, art id or variant.
              </p>
            )}
            {taken && !sameId && (
              <p className="mb-2 text-xs text-rose-400">
                A kit with asset id {dstId} is already registered on this team.
              </p>
            )}
            {!sameLen && (
              <p className="mb-2 text-xs text-amber-300">
                The new id has a different character length than the source ({dstId.length} vs{" "}
                {srcId.length}), so the texture files can't be cloned (their containers reference
                the id in fixed-length name slots). The registry row can still be created.
              </p>
            )}

            <label
              className="mb-4 flex items-center gap-2 text-xs text-slate-300"
              title="Copy the source kit's jersey/pants/socks textures (and its FE jersey-select thumbnail) under the new asset id, rewriting the embedded id strings"
            >
              <input
                type="checkbox"
                checked={cloneTextures && !!installDir && sameLen}
                disabled={!installDir || !sameLen || run.isPending}
                onChange={(e) => setCloneTextures(e.target.checked)}
              />
              Also clone the kit's texture files onto {dstId}
              {!installDir && <span className="text-slate-500">(needs an install tree)</span>}
            </label>

            {run.isError && (
              <p className="mb-3 rounded-md border border-rose-800 bg-rose-950/60 px-3 py-2 text-xs text-rose-100">
                {String(run.error)}
              </p>
            )}

            <div className="flex justify-end gap-2">
              <Button variant="secondary" onClick={onClose} disabled={run.isPending}>
                Cancel
              </Button>
              <button
                type="button"
                onClick={() => run.mutate()}
                disabled={run.isPending || sameId || taken}
                className="rounded-md border border-emerald-600/50 bg-emerald-700/80 px-3 py-1.5 text-sm font-medium text-white transition-colors hover:bg-emerald-600 disabled:cursor-not-allowed disabled:opacity-50"
              >
                {run.isPending ? "Creating…" : "Create jersey"}
              </button>
            </div>
          </>
        ) : (
          <>
            <h2 className="mb-2 text-lg font-semibold text-emerald-200">
              New jersey {dstId} created
            </h2>
            <div className="space-y-2 text-xs text-slate-300">
              <p>
                Registry row {result.db.record} was staged as a pending edit —{" "}
                <strong>Save</strong> to keep it, or undo.
              </p>
              {result.tex ? (
                <div className="max-h-[40vh] overflow-auto rounded-md border border-slate-800 bg-slate-950/50 p-2">
                  <p className="mb-1 font-semibold text-slate-400">
                    Cloned {result.tex.files.length} texture file
                    {result.tex.files.length === 1 ? "" : "s"}
                  </p>
                  <ul className="space-y-0.5 font-mono text-[10px] text-slate-400">
                    {result.tex.files.map((f) => (
                      <li key={f.dst} title={`${f.rewrites} embedded id rewrites`}>
                        {f.dst}
                      </li>
                    ))}
                  </ul>
                  {result.tex.skipped_existing.length > 0 && (
                    <p className="mt-1 text-[10px] text-amber-300">
                      Skipped (already existed): {result.tex.skipped_existing.join(", ")}
                    </p>
                  )}
                  {result.tex.files.length === 0 && (
                    <p className="text-[10px] text-slate-500">
                      No texture files matched the source asset id in this install tree.
                    </p>
                  )}
                </div>
              ) : (
                <p className="text-slate-500">Texture files were not cloned.</p>
              )}
            </div>
            <div className="mt-4 flex justify-end">
              <Button onClick={onClose}>Done</Button>
            </div>
          </>
        )}
      </div>
    </div>
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
