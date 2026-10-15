import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import type { AssetRootsArgsDto } from "../bindings/AssetRootsArgsDto";
import type { TeamMaskPaintDto } from "../bindings/TeamMaskPaintDto";
import { Pill } from "./primitives";

/// Equipment 4.5 G2 (receipt 42; research R125 / R125b) — the team's goalie mask paints: per numbered
/// slot, the starter's and the backup's stored paint number with the file check a goalie's own mask
/// gets on the Xbox 360 / PS3 loose trees. Read-only; the numbers are edited through the team's
/// fields. Nothing here says which uniform uses a slot or what the game draws.
export function TeamGoalieMasksCard({ projectId, teamId, roots }: { projectId: number; teamId: number; roots: AssetRootsArgsDto }) {
  const q = useQuery({
    queryKey: ["team-goalie-masks", projectId, teamId, roots.install_dir, roots.platform],
    queryFn: () => call("team_goalie_masks", { projectId, teamId, roots }),
    retry: false,
  });
  if (q.isLoading) return null;
  if (q.isError) {
    return (
      <p className="text-xs text-rose-300" data-testid="team-goalie-masks-error">
        {String(q.error)}
      </p>
    );
  }
  const d = q.data;
  if (!d || !d.applies) return null;
  const slots = Array.from(new Set(d.paints.map((p) => p.slot))).sort((a, b) => a - b);
  const paint = (slot: number, role: string) => d.paints.find((p) => p.slot === slot && p.role === role);
  return (
    <div className="rounded-md border border-slate-800 bg-slate-900/40 px-3 py-2" data-testid="team-goalie-masks">
      <h4 className="text-xs font-semibold text-slate-200">Team goalie mask paints</h4>
      <p className="text-[11px] text-slate-500">
        Stored per numbered slot for the starter and the backup. A paint number names mask art files; which uniform uses which slot is not established.
      </p>
      <table className="mt-1 text-xs">
        <thead>
          <tr className="text-left text-slate-500">
            <th className="pr-3 font-normal">Slot</th>
            <th className="pr-3 font-normal">Starter</th>
            <th className="font-normal">Backup</th>
          </tr>
        </thead>
        <tbody>
          {slots.map((s) => (
            <tr key={s} data-testid={`team-mask-slot-${s}`}>
              <td className="pr-3 text-slate-400">{s}</td>
              <td className="pr-3">
                <PaintCell p={paint(s, "starter")} />
              </td>
              <td>
                <PaintCell p={paint(s, "backup")} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <ul className="mt-1 list-disc pl-4 text-[11px] text-slate-500">
        {d.notes.map((n) => (
          <li key={n}>{n}</li>
        ))}
      </ul>
    </div>
  );
}

const TONE: Record<string, "ok" | "warn" | "neutral"> = {
  files: "ok",
  no_files: "warn",
  shipped_exception: "warn",
  empty: "neutral",
  no_folder: "neutral",
  not_checked: "neutral",
};
const SHORT: Record<string, (p: TeamMaskPaintDto) => string> = {
  files: (p) => `${p.file_count} file${p.file_count === 1 ? "" : "s"}`,
  no_files: () => "no files",
  shipped_exception: () => "shipped exception",
  empty: () => "none",
  no_folder: () => "not checked",
  not_checked: () => "not checked",
};

function PaintCell({ p }: { p: TeamMaskPaintDto | undefined }) {
  if (!p) return <span className="text-slate-600">—</span>;
  return (
    <span className="inline-flex items-baseline gap-1" title={[p.note, p.field_name, ...p.files].join("\n")} data-testid={`team-mask-${p.slot}-${p.role}`}>
      <span className="font-mono text-slate-300">{p.value == null ? "—" : p.value === 0 ? "0" : `paint ${p.value}`}</span>
      <Pill tone={TONE[p.status] ?? "neutral"}>{(SHORT[p.status] ?? (() => p.status))(p)}</Pill>
      {p.helmet_types != null && (
        <span className="text-[10px] text-slate-500" data-testid={`team-mask-types-${p.slot}-${p.role}`}>
          {p.helmet_types.length === 0 ? "no catalogue record" : `helmet type${p.helmet_types.length === 1 ? "" : "s"} ${p.helmet_types.join(", ")}`}
        </span>
      )}
    </span>
  );
}
