// Player Equipment tab — the per-player gear surface (replaces the old Portrait placeholder). Reads
// the player's own equipment fields (early biography or later companion), resolved to named, editable
// field rows: jersey/helmet/visor/stick for skaters, the goalie gear + 9-zone RGB for goalies (the
// same set the reference app's Equipment tab exposed). A 3D gear preview — the shared kit viewer,
// locked to this player so it shows this record's gear — sits alongside when the project has an
// install tree, and refreshes live as fields are edited (set_field invalidates its query).

import { useAssetRootArgs, useProject } from "../lib/project";
import { Nhl07PlayerAssignments } from "./Nhl07PlayerAssignments";
import { EquipmentFilesCard } from "./EquipmentFilesCard";
import { Ps4GoalieMaskPairCard } from "./Ps4GoalieMaskPairCard";
import { Ps4SkaterGearCard } from "./Ps4SkaterGearCard";
import { FieldRow } from "./FieldRow";
import { TeamKitViewer } from "./KitMeshViewer";
import { EmptyState } from "./primitives";
import type { PlayerViewDto } from "../bindings/PlayerViewDto";

export function PlayerEquipmentTab({
  projectId,
  player,
}: {
  projectId: number;
  player: PlayerViewDto;
}) {
  const { installDir, rootArgs } = useAssetRootArgs();
  const { project } = useProject();
  if (project?.game_id === "nhl07" && ["xbox", "xbox360"].includes(project.platform)) {
    return <Nhl07PlayerAssignments projectId={projectId} playerId={player.player_id} />;
  }
  const teamId = player.header.primary_team_id;
  const isGoalie = player.kind === "goalie";

  const fields = (
    <div className="min-w-[22rem] flex-1 basis-[26rem] space-y-3">
      <div className="flex items-baseline justify-between">
        <h3 className="text-sm font-semibold text-slate-100">
          {isGoalie ? "Goalie equipment" : "Equipment"}
        </h3>
        <span
          className="text-[10px] uppercase tracking-wide text-slate-500"
          title={`Per-player gear (${player.equipment_table})`}
        >
          {isGoalie ? "goalie equipment" : "sticks · gloves · protective"}
        </span>
      </div>

      {player.equipment_record == null || player.equipment.length === 0 ? (
        <EmptyState
          title="No equipment fields available"
          hint="Studio could not resolve editable equipment fields for this player."
        />
      ) : (
        <div className="rounded-md border border-slate-800 bg-slate-900/40 px-3 py-2 sm:grid sm:grid-cols-2 sm:gap-x-4">
          {player.equipment.map((f) => (
            <FieldRow
              key={f.studio_field}
              projectId={projectId}
              record={player.equipment_record!}
              field={f}
              labelWidth="120px"
            />
          ))}
        </div>
      )}

      {installDir && player.equipment_record != null && player.equipment.length > 0 && (
        <EquipmentFilesCard projectId={projectId} playerId={player.player_id} roots={rootArgs} />
      )}
      {/* Receipt 52: PS4 skaters — gear numbers against the title's own Creation Zone lists. */}
      {!isGoalie && rootArgs.platform === "ps4" && player.equipment_record != null && (
        <Ps4SkaterGearCard
          projectId={projectId}
          playerId={player.player_id}
          roots={rootArgs}
          fieldsKey={player.equipment
            .filter((f) => ["helmetbrand", "gloves", "skatebrand", "stick", "visortype"].includes(f.name))
            .map((f) => String(f.decoded))
            .join("/")}
        />
      )}
      {/* G3 (receipt 51): PS4 goalies — the (mask, helmet type) pair against the title's catalogue. */}
      {isGoalie && rootArgs.platform === "ps4" && player.equipment_record != null && (
        <Ps4GoalieMaskPairCard
          projectId={projectId}
          playerId={player.player_id}
          roots={rootArgs}
          fieldsKey={player.equipment.filter((f) => f.name === "maskid" || f.name === "masktype").map((f) => String(f.decoded)).join("/")}
        />
      )}
    </div>
  );

  return (
    <div className="flex flex-wrap items-start gap-3">
      {/* The gear preview is a team-scoped render (gear textures are team-keyed), locked to this
          player so the goalie/skater mesh set and handedness match the record being edited. It needs
          an install tree and a team to anchor the kit; free agents / no-install fall back to a note. */}
      <section className="h-[72vh] min-h-[34rem] min-w-[30rem] flex-1 basis-[40rem]">
        {installDir && teamId != null ? (
          <TeamKitViewer
            projectId={projectId}
            roots={rootArgs}
            teamId={teamId}
            style={0}
            variant={0}
            playerId={player.player_id}
            onPlayerIdChange={() => {}}
            hidePicker
            playerGear
          />
        ) : (
          <p className="rounded-md border border-slate-800 bg-slate-900/40 px-3 py-2 text-xs text-slate-400">
            {installDir
              ? "This player is a free agent (no team) — the gear preview needs a team kit to render against. Equipment fields are still editable."
              : "Link an extracted game folder to preview gear. Equipment fields are still editable."}
          </p>
        )}
      </section>

      {fields}
    </div>
  );
}
