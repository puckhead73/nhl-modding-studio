import { useState, type ReactNode } from "react";
import { useParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { EmptyState, Spinner } from "../components/primitives";
import { DetailTabs, type DetailTab } from "../components/DetailTabs";
import { Section } from "../components/FieldProfile";
import { ReadonlyField } from "../components/EditableField";
import { LegendsTeamHeader } from "../components/LegendsTeamHeader";
import {
  HistoricalDepthChart,
  HistoricalJerseyCaptaincy,
  Swatch,
  blockTitle,
  seasonLabel,
  type RatingCell,
} from "../components/HistoricalTeamViews";
import { franchiseIdOf, teamKey as toTeamKey } from "../lib/legendsKey";
import { ratingToneClass } from "../lib/ratingColor";
import { useCrumbLabel } from "../lib/breadcrumbs";
import type { HistoricalTeamBlockDto } from "../bindings/HistoricalTeamBlockDto";
import type { HistoricalTeamMemberDto } from "../bindings/HistoricalTeamMemberDto";
import type { LegendsMemberDto } from "../bindings/LegendsMemberDto";

/// A Legends build — a team wearing one era's identity, carrying the roster Puck's Picks assembled.
///
/// Two independent knobs:
///  - **Season Identity** (the banner dropdown) picks the era whose colours, name/location and crest
///    the build wears. It reads a team-season out of the historical corpus for those fields only —
///    changing it never touches the roster.
///  - **The roster** comes from `legends_roster`, ranked on the Legends OVR precomputed for the
///    active formula profile. Players qualify on 200+ NHL regular-season games and land on their
///    most-tenured franchise; see `studio-domain`'s `legends` module for the full rule set.
///
/// The two Legends-only teams (European Elite, Windsor Legends) have no franchise lineage, so they
/// carry a roster but no season identity.
export function LegendsTeamDetail() {
  const { id } = useParams<{ id: string }>();
  const key = toTeamKey(id ?? "");
  const franchiseId = franchiseIdOf(key);
  const isSpecial = franchiseId == null;

  // null = "the corpus's newest season for this franchise", resolved server-side.
  const [identitySeason, setIdentitySeason] = useState<number | null>(null);

  const identity = useQuery({
    queryKey: ["legends_identity", franchiseId, identitySeason],
    queryFn: () =>
      call("historical_franchise", {
        kind: "franchise",
        key: String(franchiseId),
        seasonYear: identitySeason,
      }),
    enabled: franchiseId != null,
  });

  const roster = useQuery({
    queryKey: ["legends_roster", key],
    queryFn: () => call("legends_roster", { teamKey: key }),
    enabled: key !== "",
    retry: false,
    // Same as the team list: only a recalculation changes a roster, and that invalidates this key.
    staleTime: Infinity,
  });

  const f = identity.data ?? null;
  const specialName = key === "european-elite" ? "European Elite" : "Windsor Legends";
  useCrumbLabel(isSpecial ? specialName : f?.display_name);

  if (identity.isLoading || roster.isLoading) return <Spinner label="Loading…" />;

  const block: HistoricalTeamBlockDto | null = f?.constituents[0] ?? null;
  const season = identitySeason ?? f?.season_year ?? null;
  const members = roster.data ?? [];
  const adapted = members.map(toRosterMember);

  const tabs: DetailTab[] = [
    {
      key: "team-data",
      label: "Team Data",
      content: isSpecial ? (
        <SpecialTeamData name={specialName} teamKey={key} count={members.length} />
      ) : (
        <TeamDataTab franchiseName={f?.display_name ?? ""} block={block} />
      ),
    },
    {
      key: "depth-chart",
      label: "Depth Chart",
      content: (
        <HistoricalDepthChart
          roster={adapted}
          rating={legendsRating}
          notice={roster.isError ? <RosterUnavailable error={roster.error} /> : undefined}
        />
      ),
    },
    {
      key: "lines",
      label: "Edit Lines",
      content: (
        <EmptyState
          title="Line editing isn't built yet"
          hint="Forward lines, defence pairs and special teams come next, now that the roster exists. The built lines will write onto the EA team this build replaces."
        />
      ),
    },
    {
      key: "jerseys-captaincy",
      label: "Jersey Numbers & Captaincy",
      content: (
        <HistoricalJerseyCaptaincy
          roster={adapted}
          seasonYear={season ?? new Date().getFullYear()}
          notice={
            <p className="text-[11px] text-slate-500">
              Legends players carry no jersey number or captaincy yet — those are assigned when the
              build is placed onto an EA team.
            </p>
          }
        />
      ),
    },
    {
      key: "strategy",
      label: "Strategy",
      content: (
        <EmptyState
          title="Strategy isn't built yet"
          hint="The build's system — forecheck, breakout, power play and penalty kill — comes with the line editor."
        />
      ),
    },
  ];

  return (
    <div className="space-y-3">
      <div className="max-w-6xl">
        {isSpecial ? (
          <SpecialTeamHeader name={specialName} teamKey={key} count={members.length} />
        ) : (
          f && (
            <LegendsTeamHeader
              franchise={f}
              block={block}
              identitySeason={season ?? f.season_year}
              onIdentitySeasonChange={setIdentitySeason}
              rosterCount={members.length}
            />
          )
        )}
      </div>
      <DetailTabs tabs={tabs} />
    </div>
  );
}

/// Adapt a Legends roster member into the shape the shared roster views render. `ability` carries
/// the Legends OVR so the depth chart's existing sort (rating descending) orders the team correctly;
/// `legendsRating` below is what turns it back into an EA-scale display value.
function toRosterMember(m: LegendsMemberDto): HistoricalTeamMemberDto {
  return {
    person_id: m.person_id,
    first_name: m.first_name,
    last_name: m.last_name,
    is_goalie: m.is_goalie,
    position: m.position,
    dob: m.dob,
    nationality: m.nationality,
    // Numbers and captaincy are assigned when the build lands on an EA team, not by the ranking.
    jersey: null,
    captaincy: null,
    ability: m.overall,
    potential: null,
  };
}

/// The depth chart's right-hand number for a Legends team: the Legends OVR on the EA 0..99 scale, so
/// unlike the historical FHM ability it *is* tier-coloured.
function legendsRating(m: HistoricalTeamMemberDto): RatingCell {
  const ovr = m.ability == null ? null : Math.round(m.ability);
  return {
    text: ovr == null ? "—" : String(ovr),
    tone: ratingToneClass(ovr),
    title: "Legends OVR — the EA Overall formula over each attribute's three best seasons",
  };
}

function RosterUnavailable({ error }: { error: unknown }) {
  return (
    <div className="rounded-md border border-dashed border-amber-800/70 bg-amber-950/30 px-3 py-2 text-xs text-amber-200">
      <span className="font-semibold">Roster unavailable.</span> {String(error)}
    </div>
  );
}

/** One read-only cell — the EA Team Data grid's `ReadonlyField`, with a nullable value. */
function F({ label, value }: { label: string; value: ReactNode }) {
  return <ReadonlyField label={label}>{value ?? ""}</ReadonlyField>;
}

/// Team Data — the identity this build wears, sourced entirely from the Season Identity pick. No
/// affiliates, parent clubs, contracts or cap data: a Legends team is a fresh club wearing an era's
/// look, so the corpus's surrounding league bookkeeping isn't part of it.
function TeamDataTab({
  franchiseName,
  block,
}: {
  franchiseName: string;
  block: HistoricalTeamBlockDto | null;
}) {
  if (!block) {
    return <EmptyState title="No club data for the selected season identity." />;
  }
  const name = blockTitle("franchise", block);
  // FHM stores one era name ("Minnesota North Stars") plus the nickname; the location is the
  // remainder. Derived for display only — it is not a stored field.
  const nickname = block.nickname?.trim() || null;
  const location =
    nickname && name.toLowerCase().endsWith(nickname.toLowerCase())
      ? name.slice(0, name.length - nickname.length).trim() || null
      : null;

  return (
    <div className="space-y-6">
      <Section title="Identity">
        <F label="Team Name" value={name} />
        <F label="Location" value={location} />
        <F label="Nickname" value={nickname} />
        <F label="Abbreviation" value={block.abbr} />
        <F label="Franchise" value={franchiseName !== name ? franchiseName : null} />
        <F label="Season Identity" value={seasonLabel(block.season_year)} />
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

      <Section title="Crest">
        <F
          label="Logo"
          value={
            <span className="text-[11px] text-slate-500">
              Pending the logo database — the banner draws the club colours meanwhile.
            </span>
          }
        />
      </Section>
    </div>
  );
}

/// The two Legends-only teams have no franchise lineage to wear, so their banner is a fixed identity
/// with the roster count — no season dropdown, because there is no era to pick.
function SpecialTeamHeader({
  name,
  teamKey,
  count,
}: {
  name: string;
  teamKey: string;
  count: number;
}) {
  const europe = teamKey === "european-elite";
  return (
    <div className="flex items-start gap-4 rounded-lg border border-slate-800 bg-gradient-to-r from-slate-900/80 to-slate-900/30 p-4">
      <div className="grid h-28 w-28 shrink-0 place-items-center overflow-hidden rounded-md border border-slate-700 bg-slate-800/70">
        <svg viewBox="0 0 24 24" className="h-[95%] w-[95%]" aria-hidden>
          <path
            d="M12 2 4 5v6c0 5 3.4 9.4 8 11 4.6-1.6 8-6 8-11V5l-8-3Z"
            fill={europe ? "#1d4ed8" : "#b91c1c"}
            stroke="#0f172a"
            strokeWidth="1.5"
          />
        </svg>
      </div>
      <div className="min-w-0 flex-1 space-y-1.5">
        <div className="flex items-center gap-2">
          <h1 className="truncate text-2xl font-semibold leading-tight text-slate-50">{name}</h1>
          <span className="shrink-0 rounded-full border border-amber-700/60 bg-amber-900/30 px-2.5 py-0.5 text-xs font-medium text-amber-300">
            Legends only
          </span>
        </div>
        <p className="text-xs text-slate-400">
          {europe
            ? "The best European players who missed a franchise team — no NHL games required, so pre-NHL and non-NHL greats are reachable."
            : "The best North American players who missed a franchise team — no NHL games required, so WHA, NHA and other-league greats are reachable."}
        </p>
        <p className="text-xs text-slate-400">{count} on roster</p>
      </div>
    </div>
  );
}

function SpecialTeamData({
  name,
  teamKey,
  count,
}: {
  name: string;
  teamKey: string;
  count: number;
}) {
  const europe = teamKey === "european-elite";
  return (
    <div className="space-y-6">
      <Section title="Identity">
        <F label="Team Name" value={name} />
        <F label="Abbreviation" value={europe ? "EUR" : "WDS"} />
        <F label="Roster" value={count} />
      </Section>
      <Section title="Eligibility">
        <F label="Region" value={europe ? "Europe" : "North America"} />
        <F label="NHL Games Required" value="None" />
        <F
          label="Pool"
          value="Players who didn't make a franchise team"
        />
      </Section>
    </div>
  );
}
