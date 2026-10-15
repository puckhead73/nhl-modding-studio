import { Link } from "react-router-dom";
import type { ProjectOverviewChangeDto } from "../bindings/ProjectOverviewChangeDto";

export function ChangeTree({ changes, depth = 0, onRevert, reverting = false }: { changes: ProjectOverviewChangeDto[]; depth?: number; onRevert?: (change: ProjectOverviewChangeDto) => void; reverting?: boolean }) {
  const grouped = new Map<string, ProjectOverviewChangeDto[]>();
  const leaves: ProjectOverviewChangeDto[] = [];
  for (const change of changes) {
    const label = change.groups[depth];
    if (label === undefined) leaves.push(change);
    else { const group = grouped.get(label) ?? []; group.push(change); grouped.set(label, group); }
  }
  return <div className="space-y-2">
    {[...grouped].sort(([a], [b]) => a.localeCompare(b)).map(([label, items]) => <details key={label} className="rounded-md border border-slate-800 bg-slate-950/20">
      <summary className="cursor-pointer px-3 py-3 text-sm text-slate-200 hover:bg-slate-800/40">{label}<span className="ml-2 text-xs text-slate-500">{items.length}</span></summary>
      <div className="border-t border-slate-800 p-3"><ChangeTree changes={items} depth={depth + 1} onRevert={onRevert} reverting={reverting} /></div>
    </details>)}
    {leaves.map(change => {
      const content = <><span className="font-medium text-slate-200">{change.label}</span><span className="mt-1 block break-words text-xs"><span className="text-slate-400">{change.before || "Empty"}</span><span aria-label="changed to" className="mx-2 text-slate-500">→</span><span className="text-sky-200">{change.after || "Empty"}</span></span></>;
      return <div key={change.id} className="flex items-center gap-2">
        {change.route
          ? <Link to={change.route} className="min-w-0 flex-1 rounded px-3 py-2 text-sm hover:bg-sky-950/40" title="Open editor">{content}</Link>
          : <div className="min-w-0 flex-1 px-3 py-2 text-sm">{content}</div>}
        {onRevert && <button type="button" disabled={reverting} onClick={() => onRevert(change)}
            aria-label={"Revert " + change.label} title={change.groups.some(group => ["Equipment", "Appearance", "Portrait and commentary"].includes(group)) ? "Revert this assignment and its linked values to the project originals" : "Revert to the original project value"}
          className="rounded px-3 py-2 text-lg text-slate-400 hover:bg-rose-950/40 hover:text-rose-300 disabled:opacity-40">×</button>}
      </div>;
    })}
  </div>;
}
