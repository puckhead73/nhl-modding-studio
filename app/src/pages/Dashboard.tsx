import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { Card, Stat } from "../components/primitives";
import type { FindingDto } from "../bindings/FindingDto";

// Severity → pill styling (badge, not bare text, so the category reads at a glance).
const SEV_BADGE: Record<string, string> = {
  error: "border-rose-700 bg-rose-950/60 text-rose-300",
  warning: "border-amber-700 bg-amber-950/50 text-amber-300",
  info: "border-slate-700 bg-slate-800/60 text-slate-400",
};

export function Dashboard() {
  const { project } = useProject();
  const p = project!;
  const projectId = p.project_id;

  const report = useQuery({
    queryKey: ["validation", projectId],
    queryFn: () => call("validate_project", { projectId }),
  });

  return (
    <div className="max-w-5xl space-y-4">
      <h2 className="text-lg font-semibold text-slate-100">Dashboard</h2>

      <div className="flex flex-wrap gap-2">
        <Stat label="Platform" value={p.platform} />
        <Stat label="Role" value={p.role} />
        <Stat label="Source" value={p.source} />
        <Stat label="Tables" value={p.table_count} />
        <Stat label="Size" value={`${p.db_size.toLocaleString()} B`} />
        <Stat label="Header CRC" value={p.header_crc_ok ? "OK" : "MISMATCH"} />
        <Stat label="Unsaved" value={p.dirty ? "yes" : "no"} />
      </div>

      <Card title="Validation">
        {report.isLoading && <p className="text-xs text-slate-400">Running audit…</p>}
        {report.isError && <p className="text-rose-400">{String(report.error)}</p>}
        {report.data && (
          <>
            <p className="mb-2 text-sm">
              <span className={report.data.clean ? "font-semibold text-emerald-400" : "font-semibold text-rose-400"}>
                {report.data.clean ? "Clean — safe to write" : "Errors present"}
              </span>{" "}
              <span className="text-slate-400">
                · {report.data.error_count} errors · {report.data.warning_count} warnings ·{" "}
                {report.data.info_count} info
              </span>
            </p>
            <div className="max-h-[50vh] space-y-1.5 overflow-auto">
              {report.data.findings.length === 0 && (
                <p className="text-xs text-slate-400">No findings.</p>
              )}
              {report.data.findings.slice(0, 200).map((f, i) => (
                <FindingRow key={i} f={f} />
              ))}
              {report.data.findings.length > 200 && (
                <p className="py-1 text-xs text-slate-500">
                  …and {report.data.findings.length - 200} more.
                </p>
              )}
            </div>
          </>
        )}
      </Card>
    </div>
  );
}

/** One integrity finding: human-readable title/location/message up front, raw ids tucked into a
 *  collapsible "Technical details" disclosure (the human-readable-surfaces rule). */
function FindingRow({ f }: { f: FindingDto }) {
  return (
    <div className="rounded-md border border-slate-800/60 bg-slate-900/40 px-3 py-2 text-[13px]">
      <div className="flex items-center gap-2">
        <span
          className={
            "inline-flex shrink-0 items-center rounded-full border px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide " +
            (SEV_BADGE[f.severity] ?? SEV_BADGE.info)
          }
        >
          {f.severity}
        </span>
        <span className="font-medium text-slate-100">{f.title}</span>
        {f.location && <span className="text-slate-400">· {f.location}</span>}
      </div>
      <p className="mt-1 text-slate-300">{f.message}</p>
      <details className="mt-1 group">
        <summary className="cursor-pointer list-none text-[11px] text-slate-500 hover:text-slate-300">
          <span className="group-open:hidden">Technical details</span>
          <span className="hidden group-open:inline">Hide details</span>
        </summary>
        <div className="mt-1 space-y-0.5 border-l border-slate-800 pl-2 font-mono text-[11px] text-slate-500">
          <div>
            <span className="text-slate-600">check:</span> {f.predicate}
          </div>
          <div>
            <span className="text-slate-600">location:</span> {f.table}.{f.field}
            {f.record != null && ` · record ${f.record}`}
          </div>
          <div>
            <span className="text-slate-600">detail:</span> {f.technical}
          </div>
        </div>
      </details>
    </div>
  );
}
