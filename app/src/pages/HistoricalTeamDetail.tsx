import { useState } from "react";
import { Link, useParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { Button, Spinner } from "../components/primitives";
import { DetailTabs, type DetailTab } from "../components/DetailTabs";
import { useCrumbLabel } from "../lib/breadcrumbs";
import { useProject } from "../lib/project";
import { useRosterCart } from "../lib/rosterCart";
import type { HistoricalTeamBlockDto } from "../bindings/HistoricalTeamBlockDto";
import type { HistoricalTeamMemberDto } from "../bindings/HistoricalTeamMemberDto";
import type { HistoricalTeamAffiliateDto } from "../bindings/HistoricalTeamAffiliateDto";

/// Historical Team detail — franchise-consolidated. The `:id` route param is a `kind:key` composite
/// (`franchise:<id>` or `team:<team_key>`). A per-team season dropdown re-scopes the view; the shown
/// name is the era-appropriate one. When concurrent clubs co-existed under one franchise that season
/// (e.g. the Seals + North Stars under the Dallas lineage), every constituent is rendered. Read-only.
export function HistoricalTeamDetail() {
  const { id } = useParams<{ id: string }>();
  const raw = id ?? "";
  // The route segment is `kind~key` (a colon separator would read as a URL scheme to react-router).
  const sep = raw.indexOf("~");
  const kind = sep >= 0 ? raw.slice(0, sep) : "team";
  const key = sep >= 0 ? raw.slice(sep + 1) : raw;

  const [selectedSeason, setSelectedSeason] = useState<number | null>(null);

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
  const seasonLabel = (y: number) => `${y - 1}-${y}`;

  // With a single constituent (the common case) the club's own era name is the page title, so the
  // constituent card drops its (duplicate) name heading. With several concurrent constituents the
  // franchise name titles the page and each card keeps its own name to tell them apart.
  const single = f.constituents.length === 1;
  const primary = single ? f.constituents[0] : null;
  const headerTitle = primary ? blockTitle(f.kind, primary) : f.display_name;
  const headerAbbr = primary?.abbr ?? f.abbr;
  const headerLeague = primary?.league_abbr ?? f.league_abbr;
  // When the era name differs from the modern franchise name, note the lineage for context.
  const franchiseSub = primary && headerTitle !== f.display_name ? f.display_name : null;

  return (
    <div className="max-w-5xl space-y-5">
      {/* Franchise header + season selector */}
      <div className="rounded-lg border border-slate-800 bg-slate-900/50 px-4 py-3">
        <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
          <h2 className="text-lg font-semibold text-slate-100">{headerTitle}</h2>
          {headerAbbr && <span className="font-mono text-sm text-teal-300/80">{headerAbbr}</span>}
          <span className="rounded bg-slate-800 px-1.5 py-0.5 text-[10px] font-medium text-slate-400">
            {headerLeague ?? "—"}
          </span>
          {f.is_defunct && (
            <span className="rounded bg-rose-900/40 px-1.5 py-0.5 text-[10px] font-medium text-rose-300">
              defunct
            </span>
          )}
          <div className="ml-auto flex items-center gap-2">
            <label className="text-[11px] uppercase tracking-wider text-slate-500">Season</label>
            <select
              value={season}
              onChange={(e) => setSelectedSeason(Number(e.target.value))}
              className="rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-xs text-slate-100 focus:border-sky-600 focus:outline-none"
            >
              {f.seasons.map((y) => (
                <option key={y} value={y}>
                  {seasonLabel(y)}
                </option>
              ))}
            </select>
          </div>
        </div>
        {franchiseSub && (
          <p className="mt-1 text-xs text-slate-500">{franchiseSub} franchise</p>
        )}
        {f.notes && <p className="mt-1 text-xs text-slate-500">{f.notes}</p>}
        {!single && (
          <p className="mt-2 text-xs text-amber-300/80">
            {f.constituents.length} clubs iced under this franchise in {seasonLabel(season)}.
          </p>
        )}
      </div>

      {f.constituents.length === 0 && (
        <p className="text-sm text-slate-400">No club data for {seasonLabel(season)}.</p>
      )}
      {f.constituents.map((c) => (
        <ConstituentCard key={c.src_team_id} kind={f.kind} block={c} showName={!single} />
      ))}
    </div>
  );
}

/// The page/card title for a club. Franchise era names are already complete ("Minnesota North
/// Stars"); a standalone team's name is just the city, so its nickname is appended.
/** Bulk "Send to Builder" for a whole team-season: queue every roster member (that season) into the
 *  Roster Builder cart. Dedups by (person, season), so members already queued are skipped. Gated on an
 *  open EA project. */
function SendTeamSeason({ block }: { block: HistoricalTeamBlockDto }) {
  const { project } = useProject();
  const cart = useRosterCart();
  const [added, setAdded] = useState<number | null>(null);
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
        seasonLabel: `${block.season_year - 1}-${block.season_year}`,
        position: m.position ?? (m.is_goalie ? "G" : null),
      });
      if (ok) n += 1;
    }
    setAdded(n);
  };

  return (
    <div className="flex flex-wrap items-center gap-2">
      <Button variant="secondary" onClick={send} disabled={remaining === 0}>
        {remaining === 0 ? "All queued" : `Send team-season to Builder (${remaining})`}
      </Button>
      {added != null && (
        <span className="text-xs text-slate-400">
          {added > 0 ? `Added ${added} player${added === 1 ? "" : "s"} to the cart.` : "Already in the cart."}
        </span>
      )}
    </div>
  );
}

function blockTitle(kind: string, block: HistoricalTeamBlockDto): string {
  if (kind === "franchise") return block.name;
  return block.nickname ? `${block.name} ${block.nickname}` : block.name;
}

function ConstituentCard({
  kind,
  block,
  showName,
}: {
  kind: string;
  block: HistoricalTeamBlockDto;
  showName: boolean;
}) {
  const skaters = block.roster.filter((r) => !r.is_goalie);
  const goalies = block.roster.filter((r) => r.is_goalie);
  // The league already shows in the page header when there's a single constituent; keep it in the
  // card's meta line only for the multi-constituent case where the card carries its own identity.
  const meta = [showName ? block.league_abbr : null, block.conference_name, block.division_name]
    .filter(Boolean)
    .join(" · ");

  const tabs: DetailTab[] = [
    {
      key: "roster",
      label: "Roster",
      badge: block.roster.length,
      content: (
        <div className="space-y-4">
          {block.roster.length > 0 && <SendTeamSeason block={block} />}
          <RosterTable title={`Skaters (${skaters.length})`} rows={skaters} season={block.season_year} />
          <RosterTable title={`Goalies (${goalies.length})`} rows={goalies} season={block.season_year} />
          {block.roster.length === 0 && (
            <p className="text-sm text-slate-500">No roster on file for this season.</p>
          )}
        </div>
      ),
    },
  ];
  if (block.contracts.length > 0) {
    tabs.push({
      key: "contracts",
      label: "Contracts",
      badge: block.contracts.length,
      content: <ContractsTable contracts={block.contracts} />,
    });
  }

  return (
    <section className="space-y-4 rounded-lg border border-slate-800 bg-slate-900/30 p-4">
      {/* Team-season identity + colours */}
      <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
        {showName && (
          <>
            <span className="text-slate-600">▸</span>
            <h3 className="text-base font-semibold text-slate-100">{blockTitle(kind, block)}</h3>
            {block.abbr && <span className="font-mono text-xs text-teal-300/80">{block.abbr}</span>}
          </>
        )}
        <div className={"flex items-center gap-2 " + (showName ? "ml-auto" : "")}>
          {!showName && (
            <span className="text-[11px] uppercase tracking-wider text-slate-500">Colours</span>
          )}
          <Swatch label="Primary" color={block.primary_color} />
          <Swatch label="Secondary" color={block.secondary_color} />
          <Swatch label="Text" color={block.text_color} />
        </div>
      </div>
      {meta && <div className="-mt-2 text-xs text-slate-400">{meta}</div>}

      <AffiliateLine
        label="Affiliates"
        rows={block.affiliates.filter((a) => a.relation === "affiliate")}
      />
      <AffiliateLine
        label="Parent club"
        rows={block.affiliates.filter((a) => a.relation === "parent")}
      />

      <DetailTabs tabs={tabs} />
    </section>
  );
}

function AffiliateLine({ label, rows }: { label: string; rows: HistoricalTeamAffiliateDto[] }) {
  if (rows.length === 0) return null;
  return (
    <div className="text-xs text-slate-400">
      <span className="uppercase tracking-wider text-slate-500">{label}: </span>
      {rows.map((a, i) => (
        <span key={`${a.relation}-${a.src_team_id}`}>
          {i > 0 && ", "}
          {a.name ?? `Team ${a.src_team_id}`}
          {a.league_abbr ? ` (${a.league_abbr})` : a.abbr ? ` (${a.abbr})` : ""}
        </span>
      ))}
    </div>
  );
}

function Swatch({ label, color }: { label: string; color: string | null }) {
  if (!color) return null;
  return (
    <span className="flex items-center gap-1" title={`${label}: ${color}`}>
      <span
        className="inline-block h-4 w-4 rounded border border-slate-600"
        style={{ backgroundColor: color }}
      />
      <span className="font-mono text-[10px] text-slate-500">{color}</span>
    </span>
  );
}

function hockeyAge(dob: string, season: number): string {
  const y = Number(dob.slice(0, 4));
  if (!Number.isFinite(y) || y <= 0) return "—";
  return String(season - y);
}

function RosterTable({
  title,
  rows,
  season,
}: {
  title: string;
  rows: HistoricalTeamMemberDto[];
  season: number;
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

function fmtSalary(n: number | null): string {
  if (n == null) return "—";
  return `$${n.toLocaleString()}`;
}

function ContractsTable({
  contracts,
}: {
  contracts: HistoricalTeamBlockDto["contracts"];
}) {
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
            {contracts.map((c) => {
              const flags = [
                c.ntc && "NTC",
                c.nmc && "NMC",
                c.elc && "ELC",
                c.ufa && "UFA",
              ].filter(Boolean);
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
