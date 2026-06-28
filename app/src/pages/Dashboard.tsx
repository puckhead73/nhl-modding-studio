import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { Card, Stat } from "../components/primitives";

const SEV_CLASS: Record<string, string> = {
  error: "text-rose-400 font-semibold",
  warning: "text-amber-400",
  info: "text-slate-400",
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
            <div className="max-h-[50vh] overflow-auto">
              {report.data.findings.length === 0 && (
                <p className="text-xs text-slate-400">No findings.</p>
              )}
              {report.data.findings.slice(0, 200).map((f, i) => (
                <div
                  key={i}
                  className="grid grid-cols-[70px_150px_1fr] gap-2 border-b border-slate-800/60 py-1 text-[13px]"
                >
                  <span className={SEV_CLASS[f.severity] ?? "text-slate-400"}>{f.severity}</span>
                  <span className="font-mono text-xs text-sky-400">{f.predicate}</span>
                  <span className="text-slate-200">{f.message}</span>
                </div>
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
