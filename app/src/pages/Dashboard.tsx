import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { Button, Card, Spinner, Stat } from "../components/primitives";
import type { FindingDto } from "../bindings/FindingDto";

// The raw `eGlu` jersey field id (FindingDto.field) that marks the *duplicate-jersey* flavour of the
// dup-on-team error — the one the whole-DB `autocorrect_jerseys` op can repair. The other dup-on-team
// flavour (a player in two slots, field `XSWT`) has no automated op, so it stays a manual fix.
const SLOT_JERSEY_FIELD = "sVRt";

/** Does this finding group have a real op that repairs it? Returns a repair descriptor or null. */
function repairFor(items: FindingDto[]): { kind: "jerseys"; label: string } | null {
  if (items.some((f) => f.predicate === "dup_on_team" && f.field === SLOT_JERSEY_FIELD)) {
    return { kind: "jerseys", label: "Fix duplicate jerseys" };
  }
  return null;
}

// Severity → pill styling (badge, not bare text, so the category reads at a glance).
const SEV_BADGE: Record<string, string> = {
  error: "border-rose-700 bg-rose-950/60 text-rose-300",
  warning: "border-amber-700 bg-amber-950/50 text-amber-300",
  info: "border-slate-700 bg-slate-800/60 text-slate-400",
};

// Ordered so the filter chips and grouped sections read worst-first.
const SEVERITIES = ["error", "warning", "info"] as const;
type Severity = (typeof SEVERITIES)[number];

export function Dashboard() {
  const { project, update } = useProject();
  const qc = useQueryClient();
  const p = project!;
  const projectId = p.project_id;

  const report = useQuery({
    queryKey: ["validation", projectId],
    queryFn: () => call("validate_project", { projectId }),
  });

  // The one automated repair wired here: whole-DB duplicate-jersey autocorrect. Runs as an audited,
  // undoable edit, then refreshes the audit + project summary (dirty) so the finding clears.
  const repairJerseys = useMutation({
    mutationFn: () => call("autocorrect_jerseys", { projectId }),
    onSuccess: async () => {
      update(await call("project_summary", { projectId }));
      await qc.invalidateQueries();
      report.refetch();
    },
  });
  const runRepair = (kind: "jerseys") => {
    if (kind === "jerseys") repairJerseys.mutate();
  };

  // Which severities are shown. Off = filtered out; all on by default.
  const [hidden, setHidden] = useState<Set<Severity>>(new Set());
  const toggle = (s: Severity) =>
    setHidden((prev) => {
      const next = new Set(prev);
      next.has(s) ? next.delete(s) : next.add(s);
      return next;
    });

  const counts: Record<Severity, number> = {
    error: report.data?.error_count ?? 0,
    warning: report.data?.warning_count ?? 0,
    info: report.data?.info_count ?? 0,
  };

  // Group the visible findings by their friendly category (`title`) so many instances of the same
  // issue collapse into one scannable section rather than a wall of near-identical rows.
  const groups = useMemo(() => {
    const findings = (report.data?.findings ?? []).filter(
      (f) => !hidden.has(f.severity as Severity),
    );
    const byTitle = new Map<string, FindingDto[]>();
    for (const f of findings) {
      const arr = byTitle.get(f.title);
      arr ? arr.push(f) : byTitle.set(f.title, [f]);
    }
    // Worst severity first, then largest group.
    return [...byTitle.entries()]
      .map(([title, items]) => ({ title, items, worst: worstSeverity(items) }))
      .sort((a, b) => sevRank(a.worst) - sevRank(b.worst) || b.items.length - a.items.length);
  }, [report.data, hidden]);

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

      <Card
        title="Validation"
        actions={
          <Button
            variant="secondary"
            onClick={() => report.refetch()}
            disabled={report.isFetching}
          >
            {report.isFetching ? "Auditing…" : "Re-run audit"}
          </Button>
        }
      >
        {report.isLoading && <Spinner label="Running audit…" />}
        {report.isError && <p className="text-rose-400">{String(report.error)}</p>}
        {report.data && (
          <>
            <div className="mb-3 flex flex-wrap items-center gap-2">
              <span
                className={
                  report.data.clean
                    ? "text-sm font-semibold text-emerald-400"
                    : "text-sm font-semibold text-rose-400"
                }
              >
                {report.data.clean ? "Clean — safe to write" : "Errors present"}
              </span>
              {/* Severity chips double as filter toggles. */}
              {SEVERITIES.map((s) => {
                const off = hidden.has(s);
                return (
                  <button
                    key={s}
                    type="button"
                    onClick={() => toggle(s)}
                    title={off ? `Show ${s}` : `Hide ${s}`}
                    className={
                      "rounded-full border px-2.5 py-0.5 text-[11px] font-medium capitalize transition-colors " +
                      (SEV_BADGE[s] ?? SEV_BADGE.info) +
                      (off ? " opacity-40 line-through" : "")
                    }
                  >
                    {counts[s]} {s}
                    {counts[s] === 1 ? "" : "s"}
                  </button>
                );
              })}
            </div>

            <div className="max-h-[55vh] space-y-2 overflow-auto">
              {report.data.findings.length === 0 && (
                <p className="text-xs text-slate-400">No findings.</p>
              )}
              {report.data.findings.length > 0 && groups.length === 0 && (
                <p className="text-xs text-slate-400">All findings hidden by the severity filter.</p>
              )}
              {groups.map((g) => (
                <FindingGroup
                  key={g.title}
                  title={g.title}
                  worst={g.worst}
                  items={g.items}
                  onRepair={runRepair}
                  repairing={repairJerseys.isPending}
                />
              ))}
            </div>
          </>
        )}
      </Card>
    </div>
  );
}

function sevRank(s: string): number {
  return s === "error" ? 0 : s === "warning" ? 1 : 2;
}
function worstSeverity(items: FindingDto[]): string {
  return items.reduce((w, f) => (sevRank(f.severity) < sevRank(w) ? f.severity : w), "info");
}

/** A collapsible category of same-kind findings (e.g. every "Duplicate jersey number"). Collapsed by
 *  default when large so the audit reads as a summary first, details on demand. When a real op can
 *  repair the category, a repair button sits on the header; error categories with no automated op are
 *  tagged "Manual fix" so the absence of a button reads as intentional. */
function FindingGroup({
  title,
  worst,
  items,
  onRepair,
  repairing,
}: {
  title: string;
  worst: string;
  items: FindingDto[];
  onRepair: (kind: "jerseys") => void;
  repairing: boolean;
}) {
  const repair = repairFor(items);
  return (
    <details className="group rounded-md border border-slate-800/60 bg-slate-900/40" open={items.length <= 5}>
      <summary className="flex cursor-pointer list-none items-center gap-2 px-3 py-2">
        <span
          className={
            "inline-flex shrink-0 items-center rounded-full border px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide " +
            (SEV_BADGE[worst] ?? SEV_BADGE.info)
          }
        >
          {worst}
        </span>
        <span className="font-medium text-slate-100">{title}</span>
        {repair ? (
          <button
            type="button"
            disabled={repairing}
            onClick={(e) => {
              e.preventDefault(); // don't toggle the <details>
              onRepair(repair.kind);
            }}
            className="ml-2 rounded border border-sky-700 bg-sky-900/50 px-2 py-0.5 text-[11px] font-medium text-sky-100 hover:bg-sky-800/60 disabled:opacity-40"
          >
            {repairing ? "Fixing…" : repair.label}
          </button>
        ) : (
          worst === "error" && (
            <span className="ml-2 rounded border border-slate-700 px-2 py-0.5 text-[10px] uppercase tracking-wide text-slate-500">
              Manual fix
            </span>
          )
        )}
        <span className="ml-auto text-xs text-slate-500">
          {items.length}
          <span className="ml-2 text-slate-600 group-open:hidden">▸</span>
          <span className="ml-2 hidden text-slate-600 group-open:inline">▾</span>
        </span>
      </summary>
      <div className="space-y-1.5 border-t border-slate-800/60 px-2 py-2">
        {items.map((f, i) => (
          <FindingRow key={i} f={f} />
        ))}
      </div>
    </details>
  );
}

/** One integrity finding within its category group: resolved location/message up front, raw ids
 *  tucked into a collapsible "Technical details" disclosure (the human-readable-surfaces rule). The
 *  severity and category are shown once on the parent group header. */
function FindingRow({ f }: { f: FindingDto }) {
  return (
    <div className="rounded-md border border-slate-800/60 bg-slate-950/40 px-3 py-2 text-[13px]">
      {f.location && <div className="font-medium text-slate-200">{f.location}</div>}
      <p className={f.location ? "mt-0.5 text-slate-300" : "text-slate-300"}>{f.message}</p>
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
