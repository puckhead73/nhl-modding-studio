import type { ReactNode } from "react";
import { Link } from "react-router-dom";
import { POS_BADGE } from "../lib/position";
import type { HistoricalTeamBlockDto } from "../bindings/HistoricalTeamBlockDto";
import type { HistoricalTeamMemberDto } from "../bindings/HistoricalTeamMemberDto";

// Shared pieces of the historical team view — the crest tile, the colour swatch, and the two
// roster-driven panels (Depth Chart, Jersey Numbers & Captaincy). The Historical Database team page
// and the Legends builder both render these: Historical feeds them a club's real season roster, the
// Legends builder feeds them the roster it assembles from calculated OVR. Keeping one implementation
// means the Legends tabs light up as soon as that roster exists, with no second copy to keep in step.

// ---------------------------------------------------------------------------
// Identity helpers
// ---------------------------------------------------------------------------

/// The display title for a club. Franchise era names are already complete ("Minnesota North
/// Stars"); a standalone team's name is just the city, so its nickname is appended.
export function blockTitle(kind: string, block: HistoricalTeamBlockDto): string {
  if (kind === "franchise") return block.name;
  return block.nickname ? `${block.name} ${block.nickname}` : block.name;
}

export function seasonLabel(year: number): string {
  return `${year - 1}-${year}`;
}

export function memberName(m: HistoricalTeamMemberDto): string {
  const full = `${m.last_name}, ${m.first_name}`.replace(/^, |, $/, "");
  return full || `Person #${m.person_id}`;
}

/** Hockey age: the player's age during the season, from the birth year alone. */
export function hockeyAge(dob: string, season: number): string {
  const y = Number(dob.slice(0, 4));
  if (!Number.isFinite(y) || y <= 0) return "—";
  return String(season - y);
}

/// The crest window. The corpus ships no logo art, so the tile draws the club's own
/// primary/secondary colours into the same shield the EA banner falls back to — the tile geometry
/// matches `TeamLogo` so the historical and EA banners line up.
export function TeamCrest({
  name,
  primary,
  secondary,
  title,
}: {
  name: string;
  primary: string | null;
  secondary: string | null;
  /** Overrides the hover title (the Legends banner explains the pending logo database). */
  title?: string;
}) {
  const fill = primary ?? "#475569";
  const stroke = secondary ?? "#1e293b";
  return (
    <div
      className="grid h-28 w-28 shrink-0 place-items-center overflow-hidden rounded-md border border-slate-700 bg-slate-800/70"
      title={
        title ??
        (primary
          ? `${name} — club colours (the historical corpus ships no logo art)`
          : `${name} — no logo art or colours on file`)
      }
    >
      <svg viewBox="0 0 24 24" className="h-[95%] w-[95%]" aria-hidden>
        <path
          d="M12 2 4 5v6c0 5 3.4 9.4 8 11 4.6-1.6 8-6 8-11V5l-8-3Z"
          fill={fill}
          stroke={stroke}
          strokeWidth="1.5"
        />
      </svg>
    </div>
  );
}

export function Swatch({ color }: { color: string | null }) {
  if (!color) return null;
  return (
    <span className="flex items-center gap-1.5" title={color}>
      <span
        className="inline-block h-4 w-4 shrink-0 rounded border border-slate-600"
        style={{ backgroundColor: color }}
      />
      <span className="font-mono text-[11px] text-slate-300">{color}</span>
    </span>
  );
}

// ---------------------------------------------------------------------------
// Depth Chart — the EA layout: five position blocks (LW / C / RW on top, D / G
// centered below). The historical corpus has no roster-status or rights fields,
// so there's no "in the system" split — every member is on the chart, and the
// right-hand number is the FHM ability (a 0–20 scale, not an EA overall, so it
// deliberately isn't tier-coloured).
// ---------------------------------------------------------------------------

const POSITION_BUCKETS = ["LW", "C", "RW", "D", "G"] as const;
type PositionBucket = (typeof POSITION_BUCKETS)[number];

const POSITION_LABELS: Record<PositionBucket, string> = {
  LW: "Left Wing",
  C: "Center",
  RW: "Right Wing",
  D: "Defense",
  G: "Goalie",
};

/** FHM's derived primary position is already C/LW/RW/D/G; `is_goalie` is the authority for G. */
function bucketFor(m: HistoricalTeamMemberDto): PositionBucket | undefined {
  if (m.is_goalie) return "G";
  const t = (m.position ?? "").trim().toUpperCase();
  return (POSITION_BUCKETS as readonly string[]).includes(t) ? (t as PositionBucket) : undefined;
}

/** Depth order: ability descending, then name. */
function compareMembers(a: HistoricalTeamMemberDto, b: HistoricalTeamMemberDto): number {
  const aA = a.ability ?? -1;
  const bA = b.ability ?? -1;
  if (aA !== bA) return bA - aA;
  return memberName(a).localeCompare(memberName(b));
}

/** How a depth-chart row renders its right-hand number. */
export type RatingCell = { text: string; tone?: string; title?: string };

/** The historical default: FHM ability on its 0–20 scale, deliberately not tier-coloured. */
const fhmAbility = (m: HistoricalTeamMemberDto): RatingCell => ({
  text: m.ability != null ? m.ability.toFixed(1) : "—",
  title: "FHM ability (0–20), not an EA overall",
});

export function HistoricalDepthChart({
  roster,
  notice,
  empty,
  rating = fhmAbility,
}: {
  roster: HistoricalTeamMemberDto[];
  /** Override the right-hand number — the Legends builder shows its EA-scale Legends OVR here. */
  rating?: (m: HistoricalTeamMemberDto) => RatingCell;
  /** Rendered above the chart (the Legends builder explains its unfilled roster here). */
  notice?: ReactNode;
  /** Rendered *instead of* the chart when the roster is empty. Omit to render the empty chart
   *  structure itself — what the Legends builder wants before its roster is assembled. */
  empty?: ReactNode;
}) {
  if (roster.length === 0 && empty !== undefined) return <>{empty}</>;

  const chart: Record<PositionBucket, HistoricalTeamMemberDto[]> = {
    LW: [],
    C: [],
    RW: [],
    D: [],
    G: [],
  };
  let unplaced = 0;
  for (const m of roster) {
    const bucket = bucketFor(m);
    if (!bucket) {
      unplaced += 1;
      continue;
    }
    chart[bucket].push(m);
  }
  for (const b of POSITION_BUCKETS) chart[b].sort(compareMembers);

  return (
    <div className="space-y-4">
      {notice}
      <div className="grid grid-cols-1 gap-4 md:grid-cols-3">
        {(["LW", "C", "RW"] as PositionBucket[]).map((b) => (
          <div key={b}>
            <PositionBlock bucket={b} players={chart[b]} rating={rating} />
          </div>
        ))}
      </div>
      <div className="flex flex-col gap-4 md:flex-row md:justify-center">
        {(["D", "G"] as PositionBucket[]).map((b) => (
          <div key={b} className="md:w-1/3">
            <PositionBlock bucket={b} players={chart[b]} rating={rating} />
          </div>
        ))}
      </div>
      {unplaced > 0 && (
        <p className="text-[11px] text-slate-500">
          {unplaced} roster {unplaced === 1 ? "member has" : "members have"} no derived position in
          the corpus and {unplaced === 1 ? "isn't" : "aren't"} on the chart.
        </p>
      )}
    </div>
  );
}

function PositionBlock({
  bucket,
  players,
  rating,
}: {
  bucket: PositionBucket;
  players: HistoricalTeamMemberDto[];
  rating: (m: HistoricalTeamMemberDto) => RatingCell;
}) {
  return (
    <section className="flex h-full flex-col rounded-lg border border-slate-800 bg-slate-900/50">
      <header className="flex items-baseline justify-between border-b border-slate-800 px-3 py-2">
        <h3 className="flex items-center gap-2 text-sm font-semibold text-slate-100">
          <span
            className={
              "inline-flex items-center rounded px-1.5 py-0.5 text-xs font-semibold " +
              (POS_BADGE[bucket] ?? "bg-slate-800 text-slate-300")
            }
          >
            {bucket}
          </span>
          {POSITION_LABELS[bucket]}
        </h3>
        <span className="text-[11px] uppercase tracking-wide text-slate-500">
          {players.length} {players.length === 1 ? "player" : "players"}
        </span>
      </header>
      <div className="flex-1 px-3 py-2">
        {players.length === 0 ? (
          <div className="px-1 py-1 text-xs italic text-slate-600">No players</div>
        ) : (
          <ul className="space-y-0.5">
            {players.map((m) => (
              <DepthRow key={m.person_id} member={m} rating={rating} />
            ))}
          </ul>
        )}
      </div>
    </section>
  );
}

function DepthRow({
  member,
  rating,
}: {
  member: HistoricalTeamMemberDto;
  rating: (m: HistoricalTeamMemberDto) => RatingCell;
}) {
  const cell = rating(member);
  return (
    <li>
      <Link
        to={`/app/historical/players/${member.person_id}`}
        className="flex items-baseline gap-2 rounded px-1 py-0.5 text-xs text-slate-100 hover:bg-slate-800/60"
      >
        <span className="inline-flex w-7 shrink-0 justify-end font-mono text-[11px] text-slate-500">
          {member.jersey != null ? `#${member.jersey}` : "—"}
        </span>
        <span className="flex min-w-0 flex-1 flex-col">
          <span className="truncate">
            {memberName(member)}
            {member.captaincy && (
              <span
                className="ml-1.5 rounded bg-amber-800/40 px-1 text-[9px] font-bold text-amber-300"
                title={member.captaincy === "C" ? "Captain" : "Alternate captain"}
              >
                {member.captaincy}
              </span>
            )}
          </span>
          {member.nationality && (
            <span className="truncate text-[10px] text-slate-500">{member.nationality}</span>
          )}
        </span>
        <span
          className={
            "w-8 shrink-0 self-center text-right font-mono text-[11px] font-semibold tabular-nums " +
            (cell.tone ?? "text-slate-300")
          }
          title={cell.title}
        >
          {cell.text}
        </span>
      </Link>
    </li>
  );
}

// ---------------------------------------------------------------------------
// Jersey Numbers & Captaincy — the EA tab's three panels (roster + number slots,
// the 99-number grid, the captaincy picks), read-only: the corpus records what a
// club actually wore, it isn't an editable assignment surface. The corpus carries
// no retired-number list, so the grid marks worn vs. free.
// ---------------------------------------------------------------------------

const NUMBERS = Array.from({ length: 99 }, (_, i) => i + 1);
const GRID_COLS = 9;

export function HistoricalJerseyCaptaincy({
  roster,
  seasonYear,
  notice,
  empty,
}: {
  roster: HistoricalTeamMemberDto[];
  seasonYear: number;
  notice?: ReactNode;
  /** Rendered *instead of* the panels when the roster is empty. Omit to render the empty panels. */
  empty?: ReactNode;
}) {
  if (roster.length === 0 && empty !== undefined) return <>{empty}</>;

  const sorted = [...roster].sort((a, b) => memberName(a).localeCompare(memberName(b)));
  const worn = new Map<number, HistoricalTeamMemberDto>();
  for (const m of sorted) if (m.jersey != null && m.jersey > 0) worn.set(m.jersey, m);
  const captain = sorted.find((m) => m.captaincy === "C") ?? null;
  const alternates = sorted.filter((m) => m.captaincy === "A");
  const unnumbered = sorted.filter((m) => m.jersey == null || m.jersey <= 0).length;

  return (
    <div className="space-y-3">
      {notice}
      <div className="flex flex-wrap items-start gap-4">
        <section className="flex w-72 shrink-0 flex-col rounded-lg border border-slate-800 bg-slate-900/40">
          <header className="flex items-baseline justify-between border-b border-slate-800 px-3 py-2">
            <span className="text-[11px] font-semibold uppercase tracking-wide text-slate-200">
              Roster
            </span>
            <span className="text-[11px] text-slate-500">{sorted.length} players</span>
          </header>
          {sorted.length === 0 ? (
            <p className="px-3 py-3 text-xs italic text-slate-600">No players yet</p>
          ) : (
            <ul className="max-h-[70vh] divide-y divide-slate-800/60 overflow-y-auto">
              {sorted.map((m) => (
                <li key={m.person_id} className="flex items-center gap-2 px-3 py-1.5 text-xs">
                  <div
                    className={
                      "flex h-8 w-10 shrink-0 items-center justify-center rounded-md border font-mono text-xs " +
                      (m.jersey != null && m.jersey > 0
                        ? "border-slate-700 bg-slate-800/60 text-slate-100"
                        : "border-dashed border-slate-700 bg-slate-950/40 text-slate-600")
                    }
                  >
                    {m.jersey != null && m.jersey > 0 ? `#${m.jersey}` : "—"}
                  </div>
                  <Link
                    to={`/app/historical/players/${m.person_id}`}
                    className="min-w-0 flex-1 truncate text-slate-100 hover:text-sky-300"
                  >
                    {memberName(m)}
                  </Link>
                  {m.captaincy && (
                    <span
                      className={
                        "shrink-0 rounded px-1 font-mono text-[9px] font-bold " +
                        (m.captaincy === "C"
                          ? "bg-amber-500/20 text-amber-300"
                          : "bg-sky-500/20 text-sky-300")
                      }
                    >
                      {m.captaincy}
                    </span>
                  )}
                  <span className="shrink-0 font-mono text-[10px] text-slate-500">
                    {m.position ?? (m.is_goalie ? "G" : "")}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </section>

        <div className="flex flex-col gap-4">
          <section className="rounded-lg border border-slate-800 bg-slate-900/40">
            <header className="border-b border-slate-800 px-3 py-2">
              <div className="flex items-baseline justify-between gap-4">
                <span className="text-[11px] font-semibold uppercase tracking-wide text-slate-200">
                  Numbers
                </span>
                <span className="text-[11px] text-slate-500">{worn.size} worn</span>
              </div>
              <p className="mt-0.5 text-[10px] text-slate-500">
                The numbers this club iced in {seasonLabel(seasonYear)} — hover a lit number for its
                wearer. The corpus records no retired-number list.
              </p>
            </header>
            <div
              className="grid w-fit gap-1.5 p-3"
              style={{ gridTemplateColumns: `repeat(${GRID_COLS}, auto)` }}
            >
              {NUMBERS.map((n) => {
                const holder = worn.get(n);
                return (
                  <span
                    key={n}
                    className={
                      "flex h-7 w-8 select-none items-center justify-center rounded-md border font-mono text-xs tabular-nums " +
                      (holder
                        ? "border-sky-700/70 bg-sky-900/30 text-sky-100"
                        : "border-slate-800 bg-slate-950/40 text-slate-600")
                    }
                    title={holder ? `#${n} — ${memberName(holder)}` : `#${n} — not worn`}
                  >
                    {n}
                  </span>
                );
              })}
            </div>
          </section>

          <section className="w-72 shrink-0 rounded-lg border border-slate-800 bg-slate-900/40">
            <header className="border-b border-slate-800 px-3 py-2 text-[11px] font-semibold uppercase tracking-wide text-slate-200">
              Captaincy
            </header>
            <div className="space-y-3 px-3 py-3">
              <CaptaincySlot label="Captain" badge="C" member={captain} />
              <CaptaincySlot label="Alternate 1" badge="A" member={alternates[0] ?? null} />
              <CaptaincySlot label="Alternate 2" badge="A" member={alternates[1] ?? null} />
              {alternates.length > 2 && (
                <p className="text-[10px] text-slate-500">
                  {alternates.length - 2} further alternate
                  {alternates.length - 2 === 1 ? "" : "s"} on record this season.
                </p>
              )}
            </div>
          </section>
        </div>
      </div>
      {unnumbered > 0 && (
        <p className="text-[11px] text-slate-500">
          {unnumbered} roster {unnumbered === 1 ? "member has" : "members have"} no number on record.
        </p>
      )}
    </div>
  );
}

function CaptaincySlot({
  label,
  badge,
  member,
}: {
  label: string;
  badge: string;
  member: HistoricalTeamMemberDto | null;
}) {
  return (
    <div className="flex flex-col gap-1 text-xs">
      <span className="flex items-center gap-2 text-[10px] uppercase tracking-wide text-slate-500">
        {label}
        <span
          className={
            "rounded px-1 font-mono text-[9px] font-bold " +
            (badge === "C" ? "bg-amber-500/20 text-amber-300" : "bg-sky-500/20 text-sky-300")
          }
        >
          {badge}
        </span>
      </span>
      {member ? (
        <Link
          to={`/app/historical/players/${member.person_id}`}
          className="truncate font-mono text-xs text-slate-100 hover:text-sky-300"
        >
          {memberName(member)}
          {member.jersey != null && member.jersey > 0 ? `  #${member.jersey}` : ""}
        </Link>
      ) : (
        <span className="font-mono text-xs text-slate-600">— none on record —</span>
      )}
    </div>
  );
}
