import { useState, type ReactNode } from "react";
import { Link, useParams, useNavigate } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { Button, EmptyState, Spinner } from "../components/primitives";
import { DetailTabs, type DetailTab } from "../components/DetailTabs";
import { Section } from "../components/FieldProfile";
import { ReadonlyField } from "../components/EditableField";
import { HistoricalTeamHeader } from "../components/HistoricalTeamHeader";
import {
  HistoricalDepthChart,
  HistoricalJerseyCaptaincy,
  Swatch,
  blockTitle,
  hockeyAge,
  seasonLabel,
} from "../components/HistoricalTeamViews";
import { useCrumbLabel } from "../lib/breadcrumbs";
import { useProject } from "../lib/project";
import { useRosterCart } from "../lib/rosterCart";
import type { HistoricalTeamBlockDto } from "../bindings/HistoricalTeamBlockDto";
import type { HistoricalTeamMemberDto } from "../bindings/HistoricalTeamMemberDto";
import type { HistoricalTeamAffiliateDto } from "../bindings/HistoricalTeamAffiliateDto";
import { teamLabel } from "../lib/team";
import { useHistoricalRatings } from "../lib/historicalRatings";
import { byOverall, ratingKey, type EstimatedRating } from "../lib/rosterSuggestions";

/// Historical Team detail — franchise-consolidated, laid out as a mirror of the EA Game Database
/// team view: the same banner shape (`HistoricalTeamHeader`) over a page-level subtab strip carrying
/// the EA tab set the corpus can fill — Team Data, Depth Chart, Roster, Jersey Numbers & Captaincy,
/// Contracts. (Lines / Strategies / Draft Picks / Uniforms have no historical source, so they're
/// omitted the way the EA page omits tabs whose surface a title doesn't carry.) Read-only.
///
/// The `:id` route param is a `kind~key` composite (`franchise:<id>` or `team:<team_key>`). Because
/// the historical view is season-scoped, the banner carries a season dropdown; when concurrent clubs
/// co-existed under one franchise that season (e.g. the Seals + North Stars under the Dallas
/// lineage) it also carries a club picker, and the tabs below show the picked club.
export function HistoricalTeamDetail() {
  const { id } = useParams<{ id: string }>();
  const raw = id ?? "";
  // The route segment is `kind~key` (a colon separator would read as a URL scheme to react-router).
  const sep = raw.indexOf("~");
  const kind = sep >= 0 ? raw.slice(0, sep) : "team";
  const key = sep >= 0 ? raw.slice(sep + 1) : raw;

  const [selectedSeason, setSelectedSeason] = useState<number | null>(null);
  const [selectedClub, setSelectedClub] = useState(0);

  const q = useQuery({
    queryKey: ["hist_franchise", kind, key, selectedSeason],
    queryFn: () => call("historical_franchise", { kind, key, seasonYear: selectedSeason }),
    enabled: key !== "",
  });
  useCrumbLabel(q.data?.display_name);

  if (q.isLoading) return <Spinner label="Loading…" />;
  if (q.isError) return <p className="text-rose-400">{String(q.error)}</p>;
  const f = q.data;
  if (!f) return <p className="text-sm text-slate-400">Select a team.</p>;

  const season = selectedSeason ?? f.season_year;
  // A season change can shrink the constituent list out from under the picker — fall back to the
  // first club rather than blanking the page.
  const clubIndex = selectedClub < f.constituents.length ? selectedClub : 0;
  const block = f.constituents[clubIndex] ?? null;

  // The EA team view's subtab set, in its order (the tabs with no historical source — Lines,
  // Strategies, Draft Picks, Uniforms — simply drop out), with the historical-only Roster tab
  // appended after them. Tab keys stay stable across season/club changes so the strip keeps its
  // selection while you page through a franchise's history.
  const tabs: DetailTab[] = block
    ? [
        {
          key: "team-data",
          label: "Team Data",
          content: <TeamDataTab kind={f.kind} franchiseName={f.display_name} block={block} />,
        },
        {
          key: "depth-chart",
          label: "Depth Chart",
          content: (
            <HistoricalDepthChart
              roster={block.roster}
              empty={<EmptyState title="No roster on file for this season." />}
            />
          ),
        },
        {
          key: "jerseys-captaincy",
          label: "Jersey Numbers & Captaincy",
          content: (
            <HistoricalJerseyCaptaincy
              roster={block.roster}
              seasonYear={block.season_year}
              empty={<EmptyState title="No players are listed under this club's roster" />}
            />
          ),
        },
        {
          key: "contracts",
          label: "Contracts",
          badge: block.contracts.length || undefined,
          content: <ContractsTab block={block} />,
        },
        {
          key: "roster",
          label: "Roster",
          badge: block.roster.length,
          content: <RosterTab block={block} />,
        },
      ]
    : [];

  return (
    <div className="space-y-3">
      {/* Header stays capped for readability; each tab caps at max-w-6xl inside DetailTabs. */}
      <div className="max-w-6xl">
        <HistoricalTeamHeader
          franchise={f}
          block={block}
          season={season}
          onSeasonChange={(y) => setSelectedSeason(y)}
          constituentIndex={clubIndex}
          onConstituentChange={setSelectedClub}
        />
      </div>
      {block ? (
        <DetailTabs tabs={tabs} />
      ) : (
        <div className="max-w-6xl">
          <EmptyState title={`No club data for ${seasonLabel(season)}.`} />
        </div>
      )}
    </div>
  );
}

/** One read-only cell — the EA Team Data grid's `ReadonlyField`, with a nullable value. */
function F({ label, value }: { label: string; value: ReactNode }) {
  return <ReadonlyField label={label}>{value ?? ""}</ReadonlyField>;
}

// ---------------------------------------------------------------------------
// Team Data — the EA page's curated section stack (Identity / Classification /
// Venue / Relations) in the same 4-column grid. The corpus has no venue, so
// Colours takes that slot: it's the club identity data the historical side
// actually carries, and it's what the Legends builder reads when it dresses a
// team's kit.
// ---------------------------------------------------------------------------

function TeamDataTab({
  kind,
  franchiseName,
  block,
}: {
  kind: string;
  franchiseName: string;
  block: HistoricalTeamBlockDto;
}) {
  const affiliates = block.affiliates.filter((a) => a.relation === "affiliate");
  const parents = block.affiliates.filter((a) => a.relation === "parent");
  const name = blockTitle(kind, block);

  return (
    <div className="space-y-6">
      <Section title="Identity">
        <F label="Full Name" value={name} />
        <F label="Nickname" value={block.nickname} />
        <F label="Abbreviation" value={block.abbr} />
        <F label="Franchise" value={franchiseName !== name ? franchiseName : null} />
        <F label="Season" value={seasonLabel(block.season_year)} />
        <F label="FHM Team Key" value={block.team_key} />
      </Section>

      <Section title="Classification">
        <F label="League" value={block.league_abbr} />
        <F label="Conference" value={block.conference_name} />
        <F label="Division" value={block.division_name} />
      </Section>

      <Section title="Colours">
        <F label="Primary" value={<Swatch color={block.primary_color} />} />
        <F label="Secondary" value={<Swatch color={block.secondary_color} />} />
        <F label="Text" value={<Swatch color={block.text_color} />} />
      </Section>

      <Section title="Relations">
        <F label="Parent Club" value={<AffiliateList rows={parents} />} />
        <F label="Affiliates" value={<AffiliateList rows={affiliates} />} />
      </Section>
    </div>
  );
}

function AffiliateList({ rows }: { rows: HistoricalTeamAffiliateDto[] }) {
  if (rows.length === 0) return null;
  return (
    <>
      {rows
        .map((a) => {
          const n = a.name ?? `Team ${a.src_team_id}`;
          const tag = a.league_abbr ?? a.abbr;
          return tag ? `${n} (${tag})` : n;
        })
        .join(", ")}
    </>
  );
}

// ---------------------------------------------------------------------------
// Roster — the full club list with the corpus columns the depth chart can't
// carry (age, nationality, potential), plus the bulk send-to-builder control.
// ---------------------------------------------------------------------------

function RosterTab({ block }: { block: HistoricalTeamBlockDto }) {
  const estimates = useHistoricalRatings(block.roster.map(r => ({ personId: r.person_id, seasonYear: block.season_year, position: r.position, isGoalie: r.is_goalie })));
  const sorted = [...block.roster].sort((a, b) => byOverall(estimates.ratings.get(ratingKey(a.person_id, block.season_year))?.overall, estimates.ratings.get(ratingKey(b.person_id, block.season_year))?.overall) || a.last_name.localeCompare(b.last_name) || a.person_id - b.person_id);
  if (block.roster.length === 0) {
    return <EmptyState title="No roster on file for this season." />;
  }
  return (
    <div className="space-y-4">
      <SendTeamSeason block={{ ...block, roster: sorted }} ratingsReady={estimates.ready} />
      {!estimates.ready && <p className="text-xs text-amber-300">{estimates.error ? "Couldn’t calculate estimated OVR with the active formula." : "Calculating estimated OVR…"}</p>}
      <RosterTable title={`Roster (${sorted.length})`} rows={sorted} season={block.season_year} ratings={estimates.ratings} />
    </div>
  );
}

/** Bulk "Send to Builder" for a whole team-season: pick the EA team this historical club is replacing,
 *  then queue every roster member (that season) into the Roster Builder cart already aimed at that team
 *  — so a full-team import needs one placement, not one per player. Each member also carries its
 *  historical jersey number over. Dedups by (person, season), so members already queued are skipped.
 *  Gated on an open EA project. */
function SendTeamSeason({ block, ratingsReady }: { block: HistoricalTeamBlockDto; ratingsReady: boolean }) {
  const navigate = useNavigate();
  const { project } = useProject();
  const cart = useRosterCart();
  const projectId = project?.project_id ?? null;
  const [added, setAdded] = useState<number | null>(null);
  const [targetTeamId, setTargetTeamId] = useState<number | null>(null);

  const teams = useQuery({
    queryKey: ["list_teams", projectId],
    queryFn: () => call("list_teams", { projectId: projectId! }),
    enabled: projectId != null,
  });

  if (!project) return null;

  // How many members aren't already in the cart for this season.
  const remaining = block.roster.filter((m) => !cart.has(m.person_id, block.season_year)).length;

  const send = () => {
    let n = 0;
    for (const m of block.roster) {
      const ok = cart.add({
        personId: m.person_id,
        seasonYear: block.season_year,
        personName: `${m.first_name} ${m.last_name}`.trim(),
        seasonLabel: seasonLabel(block.season_year),
        position: m.position ?? (m.is_goalie ? "G" : null),
        targetTeamId,
        jersey: m.jersey,
      });
      if (ok) n += 1;
    }
    setAdded(n);
    navigate("/app/historical/builder");
  };

  const selectCls =
    "rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-xs text-slate-100 focus:border-sky-600 focus:outline-none";

  const teamList = teams.data ?? [];
  return (
    <div className="flex flex-wrap items-center gap-2">
      <label className="text-[11px] uppercase tracking-wider text-slate-500">Replaces</label>
      <select
        className={selectCls}
        value={targetTeamId ?? ""}
        onChange={(e) => {
          setTargetTeamId(e.target.value ? Number(e.target.value) : null);
          setAdded(null);
        }}
        disabled={teams.isLoading || teamList.length === 0}
      >
        <option value="">
          {teams.isLoading ? "Loading teams…" : teamList.length === 0 ? "No teams found" : "Select EA team…"}
        </option>
        {teamList.map((t) => (
          <option key={t.team_id} value={t.team_id}>
            {teamLabel(t)}
          </option>
        ))}
      </select>
      <Button variant="secondary" onClick={send} disabled={remaining === 0 || targetTeamId == null || !ratingsReady}>
        {remaining === 0 ? "All queued" : `Send team-season to Builder (${remaining})`}
      </Button>
      {teams.isError && (
        <span className="text-xs text-rose-400">Couldn’t load the project’s teams.</span>
      )}
      {targetTeamId == null && remaining > 0 && !teams.isError && teamList.length > 0 && (
        <span className="text-xs text-amber-400">Choose the EA team to place this roster onto.</span>
      )}
      {added != null && (
        <span className="text-xs text-slate-400">
          {added > 0 ? `Added ${added} player${added === 1 ? "" : "s"} to the cart.` : "Already in the cart."}
        </span>
      )}
    </div>
  );
}

function RosterTable({
  title,
  rows,
  season,
  ratings,
}: {
  title: string;
  rows: HistoricalTeamMemberDto[];
  season: number;
  ratings: Map<string, EstimatedRating>;
}) {
  if (rows.length === 0) return null;
  return (
    <div>
      <h4 className="mb-1.5 text-[11px] font-bold uppercase tracking-wider text-slate-400">{title}</h4>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-xs">
          <thead>
            <tr className="text-left font-semibold uppercase tracking-wider text-slate-500">
              <th className="px-2 py-1 text-right">#</th>
              <th className="px-2 py-1">Name</th>
              <th className="px-2 py-1">Pos</th>
              <th className="px-2 py-1 text-right">Est. OVR</th>
              <th className="px-2 py-1 text-right">Age</th>
              <th className="px-2 py-1">Nat</th>
              <th className="px-2 py-1 text-right">Ability</th>
              <th className="px-2 py-1 text-right">Potential</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((r) => (
              <tr key={r.person_id} className="border-t border-slate-800/60">
                <td className="px-2 py-1 text-right tabular-nums text-slate-500">
                  {r.jersey ?? ""}
                </td>
                <td className="px-2 py-1">
                  <Link
                    to={`/app/historical/players/${r.person_id}`}
                    className="text-sky-400 hover:underline"
                  >
                    {r.last_name}, {r.first_name}
                  </Link>
                  {r.captaincy && (
                    <span
                      className="ml-1.5 rounded bg-amber-800/40 px-1 text-[9px] font-bold text-amber-300"
                      title={r.captaincy === "C" ? "Captain" : "Alternate captain"}
                    >
                      {r.captaincy}
                    </span>
                  )}
                </td>
                <td className="px-2 py-1 text-slate-300">{r.position ?? "—"}</td>
                <td className="px-2 py-1 text-right tabular-nums text-sky-300">{ratings.get(ratingKey(r.person_id, season))?.overall ?? "—"}</td>
                <td className="px-2 py-1 text-right tabular-nums text-slate-400">
                  {hockeyAge(r.dob, season)}
                </td>
                <td className="px-2 py-1 text-slate-400">{r.nationality ?? "—"}</td>
                <td className="px-2 py-1 text-right tabular-nums text-slate-300">
                  {r.ability != null ? r.ability.toFixed(1) : "—"}
                </td>
                <td className="px-2 py-1 text-right tabular-nums text-slate-300">
                  {r.potential != null ? r.potential.toFixed(1) : "—"}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Contracts — the club's cap sheet for the selected season.
// ---------------------------------------------------------------------------

function fmtSalary(n: number | null): string {
  if (n == null) return "—";
  return `$${n.toLocaleString()}`;
}

function ContractsTab({ block }: { block: HistoricalTeamBlockDto }) {
  if (block.contracts.length === 0) {
    return <EmptyState title={`No cap sheet on file for ${seasonLabel(block.season_year)}.`} />;
  }
  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-xs">
        <thead>
          <tr className="text-left font-semibold uppercase tracking-wider text-slate-500">
            <th className="px-2 py-1">Name</th>
            <th className="px-2 py-1 text-right">Avg. Salary</th>
            <th className="px-2 py-1">Clauses</th>
          </tr>
        </thead>
        <tbody>
          {block.contracts.map((c) => {
            const flags = [c.ntc && "NTC", c.nmc && "NMC", c.elc && "ELC", c.ufa && "UFA"].filter(
              Boolean,
            );
            return (
              <tr key={c.person_id} className="border-t border-slate-800/60">
                <td className="px-2 py-1">
                  <Link
                    to={`/app/historical/players/${c.person_id}`}
                    className="text-sky-400 hover:underline"
                  >
                    {c.last_name}, {c.first_name}
                  </Link>
                </td>
                <td className="px-2 py-1 text-right tabular-nums text-emerald-300/80">
                  {fmtSalary(c.avg_salary)}
                </td>
                <td className="px-2 py-1 text-slate-400">{flags.join(" · ") || "—"}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
