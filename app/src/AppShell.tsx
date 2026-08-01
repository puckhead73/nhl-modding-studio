import { useEffect, useRef, useState } from "react";
import { Link, NavLink, Navigate, Outlet, useLocation, useNavigate } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useProject } from "./lib/project";
import { call } from "./lib/client";
import { Button, Pill } from "./components/primitives";
import { SECTIONS, activeSection } from "./nav";
import { BreadcrumbProvider, useBreadcrumbs } from "./lib/breadcrumbs";
import headerLogo from "./assets/ui/NHL Modding Studio Header.png";
import type { ProjectSummaryDto } from "./bindings/ProjectSummaryDto";
import type { AutocorrectSummaryDto } from "./bindings/AutocorrectSummaryDto";
import type { RepairBlocksSummaryDto } from "./bindings/RepairBlocksSummaryDto";

export function AppShell() {
  const { project, dbPath } = useProject();
  if (!project || !dbPath) return <Navigate to="/" replace />;
  return <Shell project={project} dbPath={dbPath} />;
}

function Shell({ project, dbPath }: { project: ProjectSummaryDto; dbPath: string }) {
  const { update, close } = useProject();
  const navigate = useNavigate();
  const qc = useQueryClient();
  const { pathname } = useLocation();
  const section = activeSection(pathname);

  // On-load auto-fix sequencing: the roster-block repair runs first (both files), because every
  // other on-load fix and the corruption guard read the post-repair state — the jersey autocorrect
  // would roll back on a still-corrupt DB, and the guard must only block when the repair failed.
  const [blocksRepaired, setBlocksRepaired] = useState(false);

  // A workspace with a roster attached saves both files in place; otherwise write the single db back.
  const save = useMutation({
    mutationFn: () =>
      project.roster_attached
        ? call("save_workspace", { projectId: project.project_id })
        : call("save_project", { projectId: project.project_id, outPath: dbPath }),
    onSuccess: async () => {
      update(await call("project_summary", { projectId: project.project_id }));
      qc.invalidateQueries();
    },
  });

  const subClass = ({ isActive }: { isActive: boolean }) =>
    "rounded-md px-3 py-1.5 text-sm font-medium transition-colors " +
    (isActive
      ? "bg-sky-900/40 text-sky-100"
      : "text-slate-300 hover:bg-slate-800/60 hover:text-slate-100");

  return (
    <BreadcrumbProvider>
    <div className="flex h-full flex-col bg-slate-950 text-slate-100">
      <BlockRepairAutofix projectId={project.project_id} onDone={() => setBlocksRepaired(true)} />
      <CorruptionGuard projectId={project.project_id} enabled={blocksRepaired} />
      <JerseyAutocorrect projectId={project.project_id} start={blocksRepaired} />
      <header className="flex h-12 shrink-0 items-center gap-6 border-b border-slate-800 bg-slate-900/60 px-4">
        <img
          src={headerLogo}
          alt="NHL Modding Studio"
          draggable={false}
          className="h-7 w-auto shrink-0 select-none object-contain"
        />
        {/* Primary section bar — active by which section owns the URL, not exact-path match. */}
        <nav className="flex flex-1 items-center gap-1">
          {SECTIONS.map((s) => {
            const active = s.key === section.key;
            return (
              <Link
                key={s.key}
                to={s.index ?? s.key}
                className={
                  "rounded-md px-3 py-1.5 text-sm font-semibold transition-colors " +
                  (active
                    ? "bg-slate-800 text-slate-50"
                    : "text-slate-400 hover:bg-slate-800/60 hover:text-slate-100")
                }
              >
                {s.label}
              </Link>
            );
          })}
        </nav>
        <div className="flex items-center gap-2">
          <NavLink
            to="database/pending"
            className={({ isActive }) =>
              "flex items-center gap-1.5 rounded-md border px-3 py-1 text-xs font-medium transition-colors " +
              (isActive
                ? "border-sky-700 bg-sky-900/40 text-sky-100"
                : project.dirty
                  ? "border-amber-800/70 bg-amber-900/30 text-amber-100 hover:bg-amber-900/50"
                  : "border-slate-700 text-slate-300 hover:bg-slate-800/60")
            }
          >
            Pending Edits
            {(project.undo_depth > 0 || project.dirty) && (
              <Pill tone={project.dirty ? "warn" : "neutral"}>{project.undo_depth}</Pill>
            )}
          </NavLink>
          <span className="text-xs text-slate-400">
            {project.platform} · {project.role} · {project.source}
          </span>
          {project.roster_attached && <DisplaySourceToggle project={project} />}
          {project.roster_attached && !project.roster_aligned && !project.display_roster && (
            <Pill tone="warn">Roster: nhlng-only</Pill>
          )}
          <button
            type="button"
            onClick={() => save.mutate()}
            disabled={save.isPending || !project.dirty}
            title={project.roster_attached ? "Saves nhlng.db and the roster save together" : undefined}
            className="rounded-md border border-sky-500/40 bg-sky-600 px-3 py-1 text-xs font-medium text-white transition-colors hover:bg-sky-500 disabled:cursor-not-allowed disabled:opacity-50"
          >
            {save.isPending ? "Saving…" : "Save"}
          </button>
          <button
            type="button"
            onClick={() => {
              close();
              navigate("/", { replace: true });
            }}
            className="rounded-md border border-slate-700 px-2.5 py-1 text-xs text-slate-300 hover:bg-slate-800"
          >
            Close
          </button>
        </div>
      </header>
      {/* Roster-only mode notice: the displayed roster save doesn't line up with the main database,
          so edits go to the roster file alone and nhlng-backed areas are unavailable. */}
      {project.display_roster && !project.roster_aligned && (
        <div className="flex shrink-0 items-center gap-2 border-b border-amber-900/60 bg-amber-950/40 px-4 py-1.5 text-xs text-amber-200">
          <span aria-hidden>⚠️</span>
          <span>
            This roster save doesn't match the main database, so you're viewing and editing the
            roster file on its own. Not all areas are editable without an aligned main database
            (nhlng.db) — anything the roster save doesn't carry, like career stats, is unavailable.
          </span>
        </div>
      )}
      {/* History trail — the views clicked through, under the primary tabs; click to jump back. */}
      <Breadcrumbs />
      {/* Contextual subtab bar — only when the active section has subtabs. */}
      {section.subtabs.length > 0 && (
        <nav className="flex shrink-0 items-center gap-1 border-b border-slate-800 bg-slate-950/60 px-4 py-1.5">
          {section.subtabs.map((sub) => (
            <NavLink key={sub.to} to={sub.to} className={subClass}>
              {sub.label}
            </NavLink>
          ))}
        </nav>
      )}
      <main className="flex min-h-0 flex-1 flex-col overflow-hidden">
        <div className="min-h-0 flex-1 overflow-y-auto px-6 py-6">
          {save.isError && (
            <p className="mb-3 rounded-md border border-rose-800 bg-rose-950/60 px-3 py-2 text-sm text-rose-100">
              {String(save.error)}
            </p>
          )}
          <Outlet />
        </div>
      </main>
    </div>
    </BreadcrumbProvider>
  );
}

/** Which file the read views display: the canonical nhlng.db (default) or the mirrored roster save
 *  — for users whose game only loads the roster save. On an aligned pair this is display-only
 *  (edits fan out to both files either way); on a misaligned pair, choosing the roster puts the
 *  workspace in roster-only mode — edits apply strictly to the roster file, and the banner above
 *  the content area explains what isn't editable. Rendered whenever a mirror is attached. */
function DisplaySourceToggle({ project }: { project: ProjectSummaryDto }) {
  const { update } = useProject();
  const qc = useQueryClient();
  const toggle = useMutation({
    mutationFn: (on: boolean) =>
      call("set_display_roster", { projectId: project.project_id, on }),
    onSuccess: (summary) => {
      update(summary);
      qc.invalidateQueries();
    },
  });
  const side = (on: boolean, label: string) => (
    <button
      type="button"
      onClick={() => on !== project.display_roster && toggle.mutate(on)}
      disabled={toggle.isPending}
      className={
        "px-2 py-1 font-medium transition-colors disabled:cursor-wait " +
        (project.display_roster === on
          ? "bg-sky-900/50 text-sky-100"
          : "text-slate-400 hover:bg-slate-800/60 hover:text-slate-200")
      }
    >
      {label}
    </button>
  );
  return (
    <div
      title={
        project.roster_aligned
          ? "Which file the views display — edits and Save always apply to both files"
          : "Which file the views display — this pair is out of sync, so roster-save edits apply to the roster file only"
      }
      className="flex items-center overflow-hidden rounded-md border border-slate-700 text-[11px]"
    >
      <span className="border-r border-slate-700 bg-slate-900/60 px-2 py-1 text-[10px] uppercase tracking-wide text-slate-500">
        Viewing
      </span>
      {side(false, "Main DB")}
      {side(true, "Roster save")}
    </div>
  );
}

/** The history-trail bar: the distinct views the user has clicked through, oldest → current. Each
 *  earlier crumb is a button that navigates back to that view (truncating forward history); the
 *  current view is shown but not clickable. Hidden until there's somewhere to go back to. */
function Breadcrumbs() {
  const trail = useBreadcrumbs();
  const navigate = useNavigate();
  if (trail.length < 2) return null;
  return (
    <nav
      aria-label="Recent views"
      className="flex shrink-0 items-center gap-1 overflow-x-auto border-b border-slate-800 bg-slate-950/40 px-4 py-1 text-xs"
    >
      <span className="mr-1 shrink-0 text-[10px] font-semibold uppercase tracking-wide text-slate-600">
        Recent
      </span>
      {trail.map((c, i) => {
        const last = i === trail.length - 1;
        return (
          <span key={c.path} className="flex shrink-0 items-center gap-1">
            {i > 0 && <span className="text-slate-700">›</span>}
            {last ? (
              <span className="max-w-[16rem] truncate font-medium text-slate-200" title={c.label}>
                {c.label}
              </span>
            ) : (
              <button
                type="button"
                onClick={() => navigate(c.path)}
                title={`Back to ${c.label}`}
                className="max-w-[16rem] truncate rounded px-1 text-slate-400 transition-colors hover:bg-slate-800/60 hover:text-sky-200"
              >
                {c.label}
              </button>
            )}
          </span>
        );
      })}
    </nav>
  );
}

/** On load, repair a corrupted roster-block layout automatically — the order-only `eGlu`
 *  permutation, applied to **both** `nhlng.db` and the attached roster save (each is checked and
 *  corrected from its own bytes). A clean pair is a silent no-op; a repair shows a summary and is
 *  staged as a pending, undoable edit like the jersey autocorrect. Runs before the CorruptionGuard
 *  and the jersey autocorrect (`onDone` gates them), so those act on the healed state. */
function BlockRepairAutofix({ projectId, onDone }: { projectId: number; onDone: () => void }) {
  const qc = useQueryClient();
  const { update } = useProject();
  const started = useRef(false);
  const [summary, setSummary] = useState<RepairBlocksSummaryDto | null>(null);

  const run = useMutation({
    mutationFn: () => call("repair_roster_blocks", { projectId }),
    onSuccess: async (s) => {
      if (s.moved_slots === 0) return;
      setSummary(s);
      update(await call("project_summary", { projectId }));
      qc.invalidateQueries();
    },
    // On error the corruption persists; the CorruptionGuard (enabled by onDone) takes over.
    onSettled: () => onDone(),
  });

  useEffect(() => {
    if (started.current) return;
    started.current = true;
    run.mutate();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (!summary) return null;
  const teams = summary.teams;

  return (
    <div className="fixed inset-0 z-[55] flex items-center justify-center bg-slate-950/80 p-6 backdrop-blur-sm">
      <div className="w-full max-w-lg rounded-lg border border-sky-800/60 bg-slate-900 p-6 shadow-2xl">
        <div className="mb-2 flex items-center gap-2">
          <span className="text-lg">🔧</span>
          <h2 className="text-lg font-semibold text-sky-200">Repaired the roster layout</h2>
        </div>
        <p className="mb-3 text-sm text-slate-300">
          On load, some teams' roster blocks were stored out of order — the corruption older tools
          left behind. The Studio moved {summary.moved_slots.toLocaleString()} roster slot
          {summary.moved_slots === 1 ? "" : "s"} back into place (no players changed teams, numbers,
          or lines). The fix covers the database and any attached roster save, and is staged as a
          pending edit — <strong>Save</strong> to keep it, or undo.
        </p>
        {teams.length > 0 && (
          <div className="max-h-[40vh] overflow-auto rounded-md border border-slate-800 bg-slate-950/50 p-3">
            <div className="mb-1 text-xs font-semibold text-slate-400">
              Affected teams ({teams.length})
            </div>
            <div className="flex flex-wrap gap-1.5">
              {teams.map((t, i) => (
                <span
                  key={i}
                  className="rounded-full border border-slate-700 bg-slate-800/60 px-2 py-0.5 text-[11px] text-slate-300"
                >
                  {t}
                </span>
              ))}
            </div>
          </div>
        )}
        <div className="mt-5 flex justify-end">
          <Button onClick={() => setSummary(null)}>Got it</Button>
        </div>
      </div>
    </div>
  );
}

/** After the auto-repair has run, audit the open project. If structural roster corruption *remains*
 *  (the automatic repair failed or refused), block the app with an explanation and recommend
 *  starting from a fresh DB — with an acknowledged, unsupported "override" path that lifts the edit
 *  guard (via `set_edit_override`) after a disclaimer. */
function CorruptionGuard({ projectId, enabled }: { projectId: number; enabled: boolean }) {
  const navigate = useNavigate();
  const [acknowledged, setAcknowledged] = useState(false);
  const [step, setStep] = useState<"warn" | "confirm">("warn");

  const report = useQuery({
    queryKey: ["validation", projectId],
    queryFn: () => call("validate_project", { projectId }),
    // Audit only after the auto-repair attempt, so a repairable DB never flashes the guard.
    enabled,
  });

  const override = useMutation({
    mutationFn: () => call("set_edit_override", { projectId, allow: true }),
    onSuccess: () => setAcknowledged(true),
  });

  // Structural roster-block corruption is the irreparable, edit-blocking case.
  const corrupt = (report.data?.findings ?? []).filter(
    (f) => f.severity === "error" && f.predicate === "towx_coherence",
  );
  if (acknowledged || corrupt.length === 0) return null;

  function leave() {
    // Land on the game/platform picker, not the startup auto-open — otherwise the saved default would
    // reopen this same corrupted database and route straight back here, looping. `pick` disables the
    // one-shot auto-open for this navigation only (the saved default itself is left untouched).
    //
    // Deliberately do NOT close() here: closing sets the project to null, and AppShell's own
    // `!project` fallback would fire `<Navigate to="/" />` WITHOUT this state, clobbering `pick` and
    // reviving the loop. OpenProject closes the lingering workspace itself once it's safely on "/".
    navigate("/", { replace: true, state: { pick: true } });
  }

  return (
    <div className="fixed inset-0 z-[60] flex items-center justify-center bg-slate-950/85 p-6 backdrop-blur-sm">
      <div className="w-full max-w-xl rounded-lg border border-rose-800/60 bg-slate-900 p-6 shadow-2xl">
        {step === "warn" ? (
          <>
            <div className="mb-3 flex items-center gap-2">
              <span className="text-lg">⚠️</span>
              <h2 className="text-lg font-semibold text-rose-200">This database is corrupted</h2>
            </div>
            <div className="space-y-3 text-sm text-slate-300">
              <p>
                On load, the Studio found <strong>structural roster corruption</strong> — one or more
                teams' roster blocks are stored out of order, broken up instead of forming the single
                continuous block the game reads a roster from — and <strong>the automatic repair
                could not fix it</strong>.
              </p>
              <p>
                Because the layout can't be reconstructed reliably, we strongly recommend starting
                your work from a fresh, known-good database rather than this one.
              </p>
              {corrupt.length > 0 && (
                <div className="rounded-md border border-slate-800 bg-slate-950/50 p-3">
                  <div className="mb-1 text-xs font-semibold text-slate-400">
                    Affected ({corrupt.length})
                  </div>
                  <ul className="space-y-0.5 text-xs text-slate-300">
                    {corrupt.slice(0, 6).map((f, i) => (
                      <li key={i}>• {f.location ?? f.title}</li>
                    ))}
                    {corrupt.length > 6 && (
                      <li className="text-slate-500">…and {corrupt.length - 6} more.</li>
                    )}
                  </ul>
                </div>
              )}
              <p className="text-xs text-slate-500">
                To protect your work, editing is blocked on this database.
              </p>
            </div>
            <div className="mt-5 flex items-center justify-between gap-3">
              <button
                type="button"
                onClick={() => setStep("confirm")}
                className="text-xs text-slate-500 underline-offset-2 hover:text-slate-300 hover:underline"
              >
                Override anyway (advanced)
              </button>
              <Button onClick={leave}>Back to start</Button>
            </div>
          </>
        ) : (
          <>
            <div className="mb-3 flex items-center gap-2">
              <span className="text-lg">🛑</span>
              <h2 className="text-lg font-semibold text-rose-200">Proceed on a corrupted database?</h2>
            </div>
            <div className="space-y-3 text-sm text-slate-300">
              <p>
                You're about to enable editing on a database that is <strong>already corrupted</strong>.
                This is <strong>unsupported</strong>.
              </p>
              <p>
                Any changes you make are likely to cause unforeseen problems in-game — crashes, missing
                or scrambled rosters, or a save the game refuses to load — and those problems may not
                surface until later. Proceed only if you understand and accept this risk.
              </p>
              {override.isError && (
                <p className="rounded-md border border-rose-800 bg-rose-950/60 px-3 py-2 text-xs text-rose-100">
                  {String(override.error)}
                </p>
              )}
            </div>
            <div className="mt-5 flex items-center justify-end gap-3">
              <Button variant="secondary" onClick={() => setStep("warn")} disabled={override.isPending}>
                Cancel
              </Button>
              <button
                type="button"
                onClick={() => override.mutate()}
                disabled={override.isPending}
                className="rounded-md border border-rose-500/50 bg-rose-700/80 px-3 py-1.5 text-sm font-medium text-white transition-colors hover:bg-rose-600 disabled:cursor-not-allowed disabled:opacity-50"
              >
                {override.isPending ? "Enabling…" : "I understand — enable editing"}
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

/** On load, auto-correct any duplicate jersey numbers (keep the first wearer, renumber the rest to the
 *  lowest free number) and show a summary of what changed. The corrections are staged as a pending,
 *  undoable edit — the user reviews and saves them like any other change. Silent when nothing needs
 *  fixing, or when the DB is too corrupt to edit (the CorruptionGuard covers that case). Waits for
 *  `start` (the roster-block auto-repair finishing) — on a block-corrupt DB this op's own audit
 *  would roll it back, so it must run on the healed state. */
function JerseyAutocorrect({ projectId, start }: { projectId: number; start: boolean }) {
  const qc = useQueryClient();
  const { update } = useProject();
  const started = useRef(false);
  const [summary, setSummary] = useState<AutocorrectSummaryDto | null>(null);

  const run = useMutation({
    mutationFn: () => call("autocorrect_jerseys", { projectId }),
    onSuccess: async (s) => {
      if (s.corrections.length === 0) return;
      setSummary(s);
      update(await call("project_summary", { projectId }));
      qc.invalidateQueries();
    },
    // A structural-audit rollback (still-corrupt DB) is expected; the CorruptionGuard handles that.
  });

  useEffect(() => {
    if (!start || started.current) return;
    started.current = true;
    run.mutate();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [start]);

  if (!summary || summary.corrections.length === 0) return null;
  const n = summary.corrections.length;

  return (
    <div className="fixed inset-0 z-[55] flex items-center justify-center bg-slate-950/80 p-6 backdrop-blur-sm">
      <div className="w-full max-w-lg rounded-lg border border-sky-800/60 bg-slate-900 p-6 shadow-2xl">
        <div className="mb-2 flex items-center gap-2">
          <span className="text-lg">🔧</span>
          <h2 className="text-lg font-semibold text-sky-200">
            Fixed {n} duplicate jersey number{n === 1 ? "" : "s"}
          </h2>
        </div>
        <p className="mb-3 text-sm text-slate-300">
          On load, some teams had two players wearing the same number. The first keeps it; the others
          were reassigned the lowest free number. These are staged as pending edits — review them in
          Pending Edits and <strong>Save</strong> to keep them (or undo).
        </p>
        <div className="max-h-[45vh] space-y-1 overflow-auto rounded-md border border-slate-800 bg-slate-950/50 p-3">
          {summary.corrections.map((c, i) => (
            <div key={i} className="flex items-center justify-between gap-3 text-[13px]">
              <span className="text-slate-300">
                <span className="text-slate-400">{c.team}</span> · {c.player}
              </span>
              <span className="shrink-0 font-mono text-slate-200">
                #{c.old_number} <span className="text-slate-500">→</span> #{c.new_number}
              </span>
            </div>
          ))}
        </div>
        <div className="mt-5 flex justify-end">
          <Button onClick={() => setSummary(null)}>Got it</Button>
        </div>
      </div>
    </div>
  );
}
