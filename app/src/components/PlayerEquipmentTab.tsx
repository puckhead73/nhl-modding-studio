// Player Equipment tab — the per-player gear surface (replaces the old Portrait placeholder). Reads
// the player's own equipment companion (`xmja` skater / `fMVl` goalie), resolved to named, editable
// field rows: jersey/helmet/visor/stick for skaters, the goalie gear + 9-zone RGB for goalies (the
// same set the reference app's Equipment tab exposed). A 3D gear preview — the shared kit viewer,
// locked to this player so it shows this record's gear — sits alongside when the project has an
// install tree, and refreshes live as fields are edited (set_field invalidates its query).

import { useAssetRootArgs } from "../lib/project";
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
          {isGoalie ? "goalie gear + zones" : "sticks · gloves · protective"}
        </span>
      </div>

      {player.equipment_record == null || player.equipment.length === 0 ? (
        <EmptyState
          title="No equipment companion for this player"
          hint="This player's record has no linked equipment row to edit."
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
              : "No install tree for this project — equipment fields are editable, but the 3D gear preview needs a root-folder project."}
          </p>
        )}
      </section>

      {fields}
    </div>
  );
}
