import { useState, type ReactNode } from "react";
import { useParams } from "react-router-dom";
import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { DetailTabs, type DetailTab } from "../components/DetailTabs";
import { EmptyState, Spinner } from "../components/primitives";
import { ComputedRatings } from "../components/ComputedRatings";
import { useCrumbLabel } from "../lib/breadcrumbs";
import { POS_BADGE, POS_FULL } from "../lib/position";
import type { HistoricalPersonDto } from "../bindings/HistoricalPersonDto";
import type { HistoricalCareerRowDto } from "../bindings/HistoricalCareerRowDto";

/// Historical Player detail — read-only, brought to EA-player-view parity: a header + the same tab
/// set as the EA DB player view (Player Data / Career Stats / Attributes / Appearance). Data is the
/// FHM person: canonical bio, per-season timeline, career, computed EA ratings + raw FHM attributes.
export function HistoricalPlayerDetail() {
  const { id } = useParams<{ id: string }>();
  const personId = Number(id);

  const q = useQuery({
    queryKey: ["hist_player", personId],
    queryFn: () => call("historical_player", { personId }),
    enabled: Number.isFinite(personId),
  });
  useCrumbLabel(q.data && `${q.data.first_name} ${q.data.last_name}`.trim());

  if (q.isLoading) return <Spinner label="Loading…" />;
  if (q.isError) return <p className="text-rose-400">{String(q.error)}</p>;
  const p = q.data;
  if (!p) return <p className="text-sm text-slate-400">Select a player.</p>;

  const tabs: DetailTab[] = [
    { key: "data", label: "Player Data", content: <PlayerDataTab p={p} /> },
    {
      key: "career",
      label: "Career Stats",
      badge: p.career.length || undefined,
      content: <CareerTab career={p.career} />,
    },
    { key: "attributes", label: "Attributes", content: <AttributesTab p={p} /> },
    {
      key: "appearance",
      label: "Appearance",
      content: (
        <EmptyState
          title="Appearance — preview only"
          hint="Portrait match and head/appearance import arrive with the Roster Builder + Production Asset tools in a later stage."
        />
      ),
    },
  ];

  return (
    <div className="max-w-6xl space-y-3">
      <Header p={p} />
      <DetailTabs tabs={tabs} />
    </div>
  );
}

function Header({ p }: { p: HistoricalPersonDto }) {
  return (
    <div className="flex flex-wrap items-baseline gap-x-3 gap-y-1 rounded-lg border border-slate-800 bg-slate-900/50 px-4 py-3">
      <h2 className="text-lg font-semibold text-slate-100">
        {p.first_name} {p.last_name}
      </h2>
      {p.nick_name && <span className="text-sm text-slate-400">“{p.nick_name}”</span>}
      {p.position && (
        <span
          className={
            "inline-flex items-center rounded px-1.5 py-0.5 text-[10px] font-semibold " +
            (POS_BADGE[p.position] ?? "bg-slate-800 text-slate-300")
          }
          title={POS_FULL[p.position] ?? p.position}
        >
          {p.position}
        </span>
      )}
      <span className="text-xs text-slate-400">
        {p.first_year ?? "?"}–{p.active ? "Present" : p.last_year ?? "?"}
      </span>
      {p.retired && <span className="text-[10px] font-semibold uppercase text-slate-500">Retired</span>}
    </div>
  );
}

/** A titled block whose cells flow in a 4-column grid (mirrors the EA Player Data layout). */
function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section>
      <h3 className="mb-3 border-b border-slate-800 pb-1 text-[11px] font-bold uppercase tracking-wider text-slate-400">
        {title}
      </h3>
      <div className="grid grid-cols-2 items-start gap-x-8 gap-y-3 sm:grid-cols-3 lg:grid-cols-4">
        {children}
      </div>
    </section>
  );
}

/** One read-only field cell. */
function F({ label, value }: { label: string; value: ReactNode }) {
  const empty = value == null || value === "";
  return (
    <div className="min-w-0">
      <div className="text-[10px] uppercase tracking-wide text-slate-500">{label}</div>
      <div className="truncate text-sm text-slate-100">{empty ? <span className="text-slate-600">—</span> : value}</div>
    </div>
  );
}

function heightStr(inches: number | null): string {
  if (inches == null || inches <= 0) return "";
  return `${Math.floor(inches / 12)}'${inches % 12}"`;
}
function salaryStr(dollars: number | null): string {
  if (dollars == null) return "";
  return dollars >= 1000 ? `$${dollars.toLocaleString()}` : `$${dollars}`;
}

function PlayerDataTab({ p }: { p: HistoricalPersonDto }) {
  const birthplace = [p.birthcity, p.birthstate].filter(Boolean).join(", ");
  return (
    <div className="space-y-6">
      <Section title="Biographical Data">
        <F label="First Name" value={p.first_name} />
        <F label="Last Name" value={p.last_name} />
        <F label="Nickname" value={p.nick_name} />
        <F label="Date of Birth" value={p.dob === "?" ? null : p.dob} />
        <F label="Birthplace" value={birthplace} />
        <F label="Nationality" value={p.nationality} />
        <F label="Height" value={heightStr(p.height)} />
        <F label="Weight" value={p.weight ? `${p.weight} lb` : null} />
        <F label="Shoots/Catches" value={p.hand} />
        <F label="Position" value={p.position ? (POS_FULL[p.position] ?? p.position) : null} />
      </Section>

      <Section title="Status">
        <F label="First Season" value={p.first_year} />
        <F label="Last Season" value={p.last_year} />
        <F label="Seasons Played" value={p.seasons.length} />
        <F label="Retired" value={p.retired ? "Yes" : "No"} />
        <F label="Goaltender" value={p.is_goalie ? "Yes" : "No"} />
      </Section>

      {p.drafts.length > 0 && <DraftData drafts={p.drafts} />}

      {p.contract && (
        <Section title="Contract">
          <F label="Team" value={p.contract.team_abbr} />
          <F label="Avg Salary" value={salaryStr(p.contract.avg_salary)} />
          <F label="Years" value={p.contract.length} />
          <F
            label="Clauses"
            value={
              [p.contract.ntc && "NTC", p.contract.nmc && "NMC", p.contract.elc && "ELC", p.contract.ufa && "UFA"]
                .filter(Boolean)
                .join(", ") || "None"
            }
          />
          <F label="Scholarship" value={p.contract.scholarship} />
        </Section>
      )}

      {p.awards.length > 0 && (
        <section>
          <h3 className="mb-2 border-b border-slate-800 pb-1 text-[11px] font-bold uppercase tracking-wider text-slate-400">
            Awards ({p.awards.length})
          </h3>
          <ul className="grid grid-cols-1 gap-x-8 gap-y-0.5 text-sm text-slate-200 sm:grid-cols-2">
            {p.awards.map((a, i) => (
              <li key={i}>
                <span className="tabular-nums text-slate-400">{a.year ?? "?"}</span> {a.name}
                {a.league_abbr ? ` (${a.league_abbr})` : ""}
              </li>
            ))}
          </ul>
        </section>
      )}

      <SeasonTimeline p={p} />
    </div>
  );
}

/** Draft Data — leads with the player's primary (NHL/major) draft, position stated as overall since
 *  FHM often lacks the pick-within-round. Any additional junior/import drafts are disclosed below. */
function DraftData({ drafts }: { drafts: HistoricalPersonDto["drafts"] }) {
  const [primary, ...rest] = drafts;
  const overall = primary.overall != null ? `#${primary.overall} overall` : null;
  return (
    <div className="space-y-3">
      <Section title="Draft Data">
        <F label="Draft" value={primary.draft_name} />
        <F label="Draft Year" value={primary.year} />
        <F label="Overall" value={overall} />
        <F label="Round" value={primary.round} />
        {primary.pick != null && <F label="Pick in Round" value={primary.pick} />}
        <F label="Drafted By" value={primary.team} />
      </Section>
      {rest.length > 0 && (
        <details className="text-sm">
          <summary className="cursor-pointer text-[11px] uppercase tracking-wide text-slate-500 hover:text-slate-300">
            Other draft selections ({rest.length})
          </summary>
          <ul className="mt-2 space-y-1 text-slate-300">
            {rest.map((d, i) => (
              <li key={i} className="flex flex-wrap gap-x-2">
                <span className="tabular-nums text-slate-400">{d.year ?? "?"}</span>
                <span>{d.draft_name ?? "Draft"}</span>
                <span className="text-slate-500">
                  {d.overall != null ? `#${d.overall} overall` : d.round != null ? `Rd ${d.round}` : ""}
                  {d.team ? ` — ${d.team}` : ""}
                </span>
              </li>
            ))}
          </ul>
        </details>
      )}
    </div>
  );
}

function SeasonTimeline({ p }: { p: HistoricalPersonDto }) {
  return (
    <section>
      <h3 className="mb-2 border-b border-slate-800 pb-1 text-[11px] font-bold uppercase tracking-wider text-slate-400">
        Season Timeline ({p.seasons.length})
      </h3>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-xs">
          <thead>
            <tr className="text-left font-semibold uppercase tracking-wider text-slate-500">
              <th className="px-2 py-1">Season</th>
              <th className="px-2 py-1">Team</th>
              <th className="px-2 py-1">Lg</th>
              <th className="px-2 py-1 text-right">Abil</th>
              <th className="px-2 py-1 text-right">Pot</th>
              <th className="px-2 py-1 text-right">Ht</th>
              <th className="px-2 py-1 text-right">Wt</th>
            </tr>
          </thead>
          <tbody>
            {p.seasons.map((s) => (
              <tr key={s.season_year} className="border-t border-slate-800/60">
                <td className="px-2 py-1 tabular-nums text-slate-300">{s.season_label}</td>
                <td className="px-2 py-1 text-slate-200">{s.team_name ?? s.team_abbr ?? "—"}</td>
                <td className="px-2 py-1 text-slate-400">{s.league_abbr ?? "—"}</td>
                <td className="px-2 py-1 text-right tabular-nums text-slate-300">
                  {s.ability != null ? s.ability.toFixed(1) : "—"}
                </td>
                <td className="px-2 py-1 text-right tabular-nums text-slate-300">
                  {s.potential != null ? s.potential.toFixed(1) : "—"}
                </td>
                <td className="px-2 py-1 text-right tabular-nums text-slate-500">{s.height ?? "—"}</td>
                <td className="px-2 py-1 text-right tabular-nums text-slate-500">{s.weight ?? "—"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}

// Season-dropdown sentinels for the cross-career compilations.
const FULL_CAREER = "career";
const PEAK3 = "peak3";

/** Attributes tab — season/compilation selector → computed EA ratings, position pills, raw FHM
 *  attrs. Always computes against the *active* formula (edited on the Formulas tab); beyond the
 *  per-season views the dropdown offers Full Career (mean of every played season) and Peak (3) (mean
 *  of the 3 best), and stars ★ the player's single best season. */
function AttributesTab({ p }: { p: HistoricalPersonDto }) {
  // Default to the newest played season; sentinels select the compilations.
  const [sel, setSel] = useState<string>(() =>
    p.seasons[0] ? String(p.seasons[0].season_year) : FULL_CAREER,
  );
  const selYear = /^\d+$/.test(sel) ? Number(sel) : null;

  // The active (working) formula — set by the user on the Formulas tab, never picked here.
  const profile = useQuery({
    queryKey: ["active_formula"],
    queryFn: () => call("get_active_formula", {}),
  });
  // Per-season compute (only when a specific season is selected). Keyed on the active formula so
  // edits made on the Formulas tab flow through here.
  const ratings = useQuery({
    queryKey: ["compute_ratings", p.person_id, selYear, profile.data],
    queryFn: () =>
      call("compute_ratings", { personId: p.person_id, seasonYear: selYear!, profile: profile.data! }),
    enabled: !!profile.data && selYear != null,
    placeholderData: keepPreviousData,
  });
  // Cross-career compilations (Full Career / Peak 3 / best-season star).
  const career = useQuery({
    queryKey: ["compute_career_ratings", p.person_id, profile.data],
    queryFn: () => call("compute_career_ratings", { personId: p.person_id, profile: profile.data! }),
    enabled: !!profile.data,
    placeholderData: keepPreviousData,
  });

  const best = career.data?.best_season_year ?? null;
  const view =
    sel === FULL_CAREER ? career.data?.career_average ?? null
    : sel === PEAK3 ? career.data?.peak3 ?? null
    : ratings.data ?? null;
  const loading =
    profile.isLoading || (selYear != null ? ratings.isLoading : career.isLoading);

  const selectCls =
    "rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-xs text-slate-100 focus:border-sky-600 focus:outline-none";

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center gap-2">
        <label className="text-[11px] uppercase tracking-wide text-slate-500">Season</label>
        <select value={sel} onChange={(e) => setSel(e.target.value)} className={selectCls}>
          <optgroup label="Compilations">
            <option value={FULL_CAREER}>Full Career</option>
            {best != null && <option value={PEAK3}>Peak (3)</option>}
          </optgroup>
          <optgroup label="Seasons">
            {p.seasons.map((s) => (
              <option key={s.season_year} value={s.season_year}>
                {s.season_label}
                {s.season_year === best ? " ★" : ""}
              </option>
            ))}
          </optgroup>
        </select>
        {sel === FULL_CAREER && (
          <span className="text-[11px] text-slate-500">Mean of all {p.seasons.length} played seasons</span>
        )}
        {sel === PEAK3 && <span className="text-[11px] text-slate-500">Mean of the 3 best seasons</span>}
      </div>

      {loading ? (
        <Spinner label="Computing…" />
      ) : view ? (
        <ComputedRatings data={view} />
      ) : (
        <p className="text-sm text-slate-500">
          No ratings for this selection (no FHM ratings/position on record).
        </p>
      )}
    </div>
  );
}

const GROUPS: { kind: string; phase: string; label: string }[] = [
  { kind: "skater", phase: "rs", label: "Skater — Regular Season" },
  { kind: "skater", phase: "po", label: "Skater — Playoffs" },
  { kind: "goalie", phase: "rs", label: "Goalie — Regular Season" },
  { kind: "goalie", phase: "po", label: "Goalie — Playoffs" },
];

function CareerTab({ career }: { career: HistoricalCareerRowDto[] }) {
  const sections = GROUPS.map((g) => ({
    ...g,
    rows: career.filter((r) => r.kind === g.kind && r.phase === g.phase),
  })).filter((s) => s.rows.length > 0);

  if (sections.length === 0) return <p className="text-sm text-slate-500">No career stats.</p>;

  return (
    <div className="space-y-6">
      {sections.map((s) => {
        const cols = s.rows[0].cells.map((c) => c.label);
        return (
          <section key={s.label}>
            <h3 className="mb-2 text-[11px] font-bold uppercase tracking-wider text-slate-400">{s.label}</h3>
            <div className="overflow-x-auto">
              <table className="w-full border-collapse text-xs">
                <thead>
                  <tr className="text-left font-semibold uppercase tracking-wider text-slate-500">
                    <th className="px-2 py-1">Year</th>
                    <th className="px-2 py-1">Team</th>
                    <th className="px-2 py-1">Lg</th>
                    {cols.map((c) => (
                      <th key={c} className="px-2 py-1 text-right">
                        {c}
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {s.rows.map((r, i) => {
                    const byLabel = new Map(r.cells.map((c) => [c.label, c.value]));
                    return (
                      <tr key={i} className="border-t border-slate-800/60">
                        <td className="px-2 py-1 tabular-nums text-slate-300">{r.year}</td>
                        <td className="px-2 py-1 text-slate-200">{r.team_abbr ?? "—"}</td>
                        <td className="px-2 py-1 text-slate-400">{r.league_abbr ?? "—"}</td>
                        {cols.map((c) => (
                          <td key={c} className="px-2 py-1 text-right tabular-nums text-slate-300">
                            {byLabel.get(c) ?? ""}
                          </td>
                        ))}
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          </section>
        );
      })}
    </div>
  );
}
