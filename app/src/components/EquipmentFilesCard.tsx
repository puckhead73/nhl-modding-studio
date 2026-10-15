import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import type { AssetRootsArgsDto } from "../bindings/AssetRootsArgsDto";
import { Pill } from "./primitives";

/// Phase 4 (21 Sep 2026) — the player's equipment ids checked against the install's rendering
/// tree (research 2026-09-21 handoff 12: on the gen7 titles the catalogue *is* the file names).
/// Per slot: the assigned id's files, the family's catalogue size / id range on this title, and
/// an explicit "no files for this id" when the database points at art the title does not ship.
/// Read-only; the field rows beside it stay the editing surface.
export function EquipmentFilesCard({ projectId, playerId, roots }: { projectId: number; playerId: number; roots: AssetRootsArgsDto }) {
  const q = useQuery({
    queryKey: ["player-equipment-files", projectId, playerId, roots.install_dir, roots.platform],
    queryFn: () => call("player_equipment_files", { projectId, playerId, roots }),
    retry: false,
  });
  if (q.isLoading) return null;
  if (q.isError) {
    return (
      <p className="text-xs text-rose-300" data-testid="equipment-files-error">
        {String(q.error)}
      </p>
    );
  }
  const rows = q.data ?? [];
  if (rows.length === 0) return null;
  return (
    <div className="rounded-md border border-slate-800 bg-slate-900/40 px-3 py-2" data-testid="equipment-files">
      <h4 className="text-xs font-semibold text-slate-200">Art files on this install</h4>
      <p className="text-[11px] text-slate-500">Which render files the assigned ids own in this title's tree; an id without files has no art in the game.</p>
      <ul className="mt-1 space-y-1 text-xs">
        {rows.map((r) => {
          const missing = r.id != null && r.file_count === 0 && r.catalogue_ids > 0;
          return (
            <li key={r.slot} className="flex flex-wrap items-baseline gap-2" data-testid={`equipment-files-${r.slot}`}>
              <span className="w-24 text-slate-300">{r.label}</span>
              <span className="font-mono text-slate-400">
                id {r.id == null ? "—" : String(r.id)}
                {r.model_label ? ` · ${r.model_label}` : ""}
              </span>
              <Pill tone={missing ? "warn" : r.file_count > 0 ? "ok" : "neutral"}>{missing ? "no files" : r.file_count > 0 ? `${r.file_count} file${r.file_count === 1 ? "" : "s"}` : "no folder"}</Pill>
              <span className="text-slate-500" title={r.files.join("\n")}>
                {r.note}
              </span>
              {/* Phase 4 (22 Sep 2026): the family's component set (research R52) — complete, or
                  what the id lacks; nothing when the title's set is not measured. */}
              {r.component_groups.length > 0 ? (
                r.component_groups.map((g) => (
                  <span key={g.group} data-testid={`equipment-set-${r.slot}-${g.group}`} title={r.set_rule}>
                    <Pill tone={g.status === "complete" ? "ok" : "warn"}>
                      {g.group} set {g.status === "complete" ? "complete" : `missing ${g.missing.join(", ")}`}
                    </Pill>
                  </span>
                ))
              ) : r.set_contract === "not_measured" && r.file_count > 0 ? (
                <span className="text-slate-600" data-testid={`equipment-set-${r.slot}-unmeasured`} title={r.set_rule}>
                  set not measured on this title
                </span>
              ) : null}
            </li>
          );
        })}
      </ul>
    </div>
  );
}
