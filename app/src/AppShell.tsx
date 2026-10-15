import { useEffect, useRef, useState } from "react";
import { Link, NavLink, Navigate, Outlet, useLocation, useNavigate } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useProject } from "./lib/project";
import { call } from "./lib/client";
import { Button, Pill } from "./components/primitives";
import { SAVE_SECTIONS, SECTIONS, activeSection, type NavSection } from "./nav";
import { SaveEditorControls } from "./components/SaveEditorControls";
import { isAvailable } from "./lib/surfaces";
import { BreadcrumbProvider, useBreadcrumbs } from "./lib/breadcrumbs";
import { gameById, img } from "./lib/catalog";
import headerLogo from "./assets/ui/NHL Modding Studio Header.png";
import tabDatabase from "./assets/ui/tabs/database.svg";
import tabHistorical from "./assets/ui/tabs/historical.png";
import tabLegends from "./assets/ui/tabs/legends.png";
import { LegendsPendingBanner } from "./components/LegendsPendingBanner";
import { SessionControls } from "./components/SessionControls";
import tabShop from "./assets/ui/tabs/shop.png";
import tabAssets from "./assets/ui/tabs/assets.png";
import tabSettings from "./assets/ui/tabs/settings.png";
import type { ProjectSummaryDto } from "./bindings/ProjectSummaryDto";
import type { AutocorrectSummaryDto } from "./bindings/AutocorrectSummaryDto";
import type { RepairBlocksSummaryDto } from "./bindings/RepairBlocksSummaryDto";

export function AppShell() {
  const { project, dbPath } = useProject();
  if (!project || !dbPath) return <Navigate to="/" replace />;
  return <Shell project={project} dbPath={dbPath} />;
}

function Shell({ project }: { project: ProjectSummaryDto; dbPath: string }) {
  const [historyTarget, setHistoryTarget] = useState<HTMLDivElement | null>(null);
  const { info, saveFile } = useProject();
  const { pathname } = useLocation();
  // Save Editor sessions get the trimmed navigation (database surfaces + validation only).
  const sections = saveFile ? SAVE_SECTIONS : SECTIONS;
  const section = activeSection(pathname, sections);
  // ONB-003: an assets-only project folder has no edit session; the database-bound on-load fixes
  // and audits do not run for it.
  const assetsOnly = project.role === "assets";

  // ONB-003 §3: the last meaningful view is remembered with the project (debounced; detail routes
  // only matter once they settle) so a reopen can land there.
  useEffect(() => {
    if (!info || !pathname.startsWith("/app/")) return;
    const t = setTimeout(() => {
      call("project_set_view", { projectId: project.project_id, route: pathname }).catch(() => {
        // bookkeeping only
      });
    }, 1000);
    return () => clearTimeout(t);
  }, [pathname, info, project.project_id]);

  // On-load auto-fix sequencing: the roster-block repair runs first (both files), because every
  // other on-load fix and the corruption guard read the post-repair state — the jersey autocorrect
  // would roll back on a still-corrupt DB, and the guard must only block when the repair failed.
  const [blocksRepaired, setBlocksRepaired] = useState(project.role === "assets");

  const subClass = ({ isActive }: { isActive: boolean }) =>
    "rounded-md px-3 py-1.5 text-sm font-medium transition-colors " +
    (isActive
      ? "bg-sky-900/40 text-sky-100"
      : "text-slate-300 hover:bg-slate-800/60 hover:text-slate-100");

  return (
    <BreadcrumbProvider>
    <div className="flex h-full flex-col bg-slate-950 text-slate-100">
      {!assetsOnly && <BlockRepairAutofix projectId={project.project_id} onDone={() => setBlocksRepaired(true)} />}
      {!assetsOnly && <CorruptionGuard projectId={project.project_id} enabled={blocksRepaired} />}
      {!assetsOnly && <JerseyAutocorrect projectId={project.project_id} start={blocksRepaired} />}
      {/* Grow the existing 60px bar and its artwork by 20%. */}
      <header className="flex h-[72px] shrink-0 items-center gap-3 xl:gap-6 border-b border-slate-800 bg-slate-900/60 px-4">
        <img
          src={headerLogo}
          alt="NHL Modding Studio"
          draggable={false}
          className="h-[38.4px] xl:h-[57.6px] w-auto shrink-0 select-none object-contain"
        />
        {saveFile && (
          <div className="flex min-w-0 shrink flex-col leading-tight" title={saveFile.file.path}>
            <span className="text-sm font-semibold text-slate-100">Save Editor</span>
            <span className="truncate text-xs text-slate-400">
              {saveFile.title} · {saveFile.platform_label} · {saveFile.file.kind_label}
            </span>
            {saveFile.supplied_tables > 0 && (
              <span className="truncate text-[11px] text-amber-300/90" title="Data this save does not contain is shown from Studio's copy of the game's default database. It is read-only and never written into the save.">
                Data not stored in this save is read-only
              </span>
            )}
          </div>
        )}
        {/* Primary section bar — image tabs, active by which section owns the URL. */}
        <nav aria-label="Studio sections" className="flex min-w-0 flex-1 items-center gap-1 overflow-x-auto py-1 px-0.5 xl:gap-2">
          {sections.map((s) => (
            <SectionTab key={s.key} section={s} active={s.key === section.key} project={project} />
          ))}
        </nav>
        <div className="flex shrink-0 items-center gap-2">
          <div className="flex max-w-28 flex-col items-end gap-1">
          {!assetsOnly && <ErrorsPill projectId={project.project_id} enabled={blocksRepaired} />}
          {assetsOnly && <Pill tone="info">Assets only</Pill>}
          </div>
          {saveFile
            ? <SaveEditorControls key={project.project_id} historyTarget={historyTarget} />
            : <SessionControls key={project.project_id} historyTarget={historyTarget} />}
        </div>
      </header>
      {/* A Legends calculation owed for the active formula profile — persisted beside the corpus,
          so it survives a restart until it's run or acknowledged. */}
      <LegendsPendingBanner />
      {/* History trail — the views clicked through, under the primary tabs; click to jump back. */}
      <Breadcrumbs />
      {/* Contextual subtab bar — only when the active section has subtabs. Capability-gated subtabs
          (Arenas) drop out when the loaded title's schema doesn't carry that surface. */}
      {(() => {
        const subtabs = section.subtabs.filter(
          (sub) => sub.to !== "database/arenas" || isAvailable(project.surfaces?.arenas),
        );
        return (
          <nav className="flex shrink-0 items-center gap-1 border-b border-slate-800 bg-slate-950/60 px-4 py-1.5">
            {subtabs.map((sub) => (
              <NavLink key={sub.to} to={sub.to} className={subClass}>
                {sub.label}
              </NavLink>
            ))}
            <div ref={setHistoryTarget} className="ml-auto flex shrink-0 items-center gap-2" aria-label="Edit history" />
          </nav>
        );
      })()}
      <main className="flex min-h-0 flex-1 flex-col overflow-hidden">
        <div className="min-h-0 flex-1 overflow-y-auto px-6 py-6">
          <Outlet key={project.project_id} />
        </div>
      </main>
    </div>
    </BreadcrumbProvider>
  );
}

// Supplied section photos; Game Database retains the loaded title's box art.
const TAB_ART: Record<string, string> = {
  database: tabDatabase,
  historical: tabHistorical,
  legends: tabLegends,
  shop: tabShop,
  assets: tabAssets,
  settings: tabSettings,
};

/** Animate only the image, keeping the frame/label still. Active sections retain source color. */
function SectionTab({
  section: s,
  active,
  project,
}: {
  section: NavSection;
  active: boolean;
  project: ProjectSummaryDto;
}) {
  const game = s.key === "database" ? gameById(project.game_id) : null;
  const src = (game && img(game.file)) || TAB_ART[s.key] || tabDatabase;
  const overlay =
    s.key === "database" && game ? `${game.label.toUpperCase()} DB` : s.label.toUpperCase();
  return (
    <Link
      to={s.index ?? s.key}
      title={overlay}
      aria-label={overlay}
      aria-current={active ? "page" : undefined}
      className={
        "group relative shrink-0 overflow-hidden rounded-md outline-none transition-shadow focus-visible:ring-2 focus-visible:ring-sky-400 " +
        (active ? "ring-2 ring-sky-500/80" : "ring-1 ring-slate-700/60 hover:ring-slate-500/80")
      }
    >
      <img
        src={src}
        alt=""
        draggable={false}
        className={
          "h-[52.8px] w-auto select-none object-contain transition-[filter,transform] duration-500 ease-out group-hover:scale-[1.04] group-hover:grayscale-0 group-focus-visible:scale-[1.04] group-focus-visible:grayscale-0 motion-reduce:transform-none motion-reduce:transition-none " +
          (active ? "grayscale-0" : "grayscale")
        }
      />
      <span aria-hidden="true" className="pointer-events-none absolute inset-x-0 bottom-0 bg-gradient-to-t from-slate-950/90 to-transparent px-1 pb-1 pt-3 text-center text-[9px] font-bold uppercase leading-tight tracking-wide text-slate-100">
        {overlay}
      </span>
    </Link>
  );
}

// One-time notice for the display-source pills: shown on the user's first-ever click, then never
// again (persisted on this machine).
/** After the on-load auto-repairs settle, audit the project. When the audit reports
 *  **error-severity** findings (warnings and info never trigger this), a red pill appears in the
 *  top bar; clicking it opens the Dashboard (Settings › Dashboard) to review the findings. The
 *  app itself always lands on the Game Database — no auto-redirect. Hidden when clean. */
function ErrorsPill({ projectId, enabled }: { projectId: number; enabled: boolean }) {
  const report = useQuery({
    queryKey: ["validation", projectId],
    queryFn: () => call("validate_project", { projectId }),
    enabled,
  });
  const errors = (report.data?.findings ?? []).filter((f) => f.severity === "error").length;
  if (errors === 0) return null;
  return (
    <NavLink
      to="settings/dashboard"
      title="The integrity audit found errors — open the Dashboard to review and repair them"
      className={({ isActive }) =>
        "flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-[11px] font-semibold transition-colors " +
        (isActive
          ? "border-rose-600 bg-rose-900/70 text-rose-100"
          : "border-rose-700 bg-rose-950/60 text-rose-200 hover:bg-rose-900/60")
      }
    >
      <span aria-hidden>⚠</span>
      {errors} error{errors === 1 ? "" : "s"} detected
    </NavLink>
  );
}

/** The history-trail bar: the distinct views the user has clicked through, oldest → current. Each
 *  earlier crumb is a button that navigates back to that view (truncating forward history); the
 *  current view is shown but not clickable. Hidden until there's somewhere to go back to. */
function Breadcrumbs() {
  const trail = useBreadcrumbs();
  const navigate = useNavigate();
  if (trail.length < 2) return null;
  const prev = trail[trail.length - 2];
  return (
    <nav
      aria-label="Recent views"
      className="flex shrink-0 items-center gap-1 overflow-x-auto border-b border-slate-800 bg-slate-950/40 px-4 py-1 text-xs"
    >
      {/* Back — one step up the trail (same target as clicking the previous crumb). */}
      <button
        type="button"
        onClick={() => navigate(prev.path)}
        title={`Back to ${prev.label}`}
        aria-label={`Back to ${prev.label}`}
        className="mr-1 shrink-0 rounded border border-slate-800 px-1.5 text-slate-400 transition-colors hover:bg-slate-800/60 hover:text-sky-200"
      >
        ←
      </button>
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
          Project Overview and <strong>Save</strong> to keep them (or undo).
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


