import { POS_BADGE, POS_FULL } from "../lib/position";
import { JerseyBadge, SearchChip } from "./PlayerHeader";
import type { HistoricalPersonDto } from "../bindings/HistoricalPersonDto";

/// The Historical Player banner — the same shape as the EA `PlayerHeader`: art window, name + team
/// pill, then the bio stack (position badge, career span, DOB, birthplace, shoots/weight/height) with
/// the jersey badge + web-search chips on the right rail. Only the data source differs: everything
/// here is the FHM person record, and the art window is always the silhouette (the historical corpus
/// ships no portrait art). The team pill and jersey come from the player's most recent played season.
export function HistoricalPlayerHeader({ p }: { p: HistoricalPersonDto }) {
  const fullName = `${p.first_name} ${p.last_name}`.trim() || `Person #${p.person_id}`;
  // `seasons` is newest-first, so the head of the list is the latest played season.
  const latest = p.seasons[0] ?? null;
  const teamLabel = latest?.team_name || latest?.team_abbr || null;

  const physical = [
    p.hand && `${p.is_goalie ? "Catches" : "Shoots"} ${p.hand}`,
    p.weight ? `${p.weight} lb` : null,
    heightStr(p.height),
  ].filter(Boolean);

  const birthplace = [p.birthcity, p.birthstate].filter(Boolean).join(", ");
  const span = `${p.first_year ?? "?"}–${p.active ? "Present" : p.last_year ?? "?"}`;
  const number = latest?.jersey != null ? String(latest.jersey) : "";

  return (
    <div className="flex items-start gap-4 rounded-lg border border-slate-800 bg-gradient-to-r from-slate-900/80 to-slate-900/30 p-4">
      <PortraitPlaceholder name={fullName} />

      <div className="min-w-0 flex-1 space-y-1.5">
        <div className="flex items-center gap-2">
          <h1 className="truncate text-2xl font-semibold leading-tight text-slate-50">{fullName}</h1>
          {teamLabel ? (
            <span className="shrink-0 rounded-full border border-emerald-700/60 bg-emerald-900/40 px-2.5 py-0.5 text-xs font-medium text-emerald-300">
              {teamLabel}
            </span>
          ) : (
            <span className="shrink-0 rounded-full border border-slate-700 bg-slate-800/60 px-2.5 py-0.5 text-xs font-medium text-slate-300">
              No club on file
            </span>
          )}
          {p.retired && (
            <span className="shrink-0 rounded-full border border-slate-700 bg-slate-800/60 px-2.5 py-0.5 text-xs font-medium text-slate-400">
              Retired
            </span>
          )}
        </div>

        {(p.position || p.nick_name) && (
          <div className="flex flex-wrap items-center gap-2">
            {p.position && (
              <span
                className={
                  "inline-flex items-center rounded px-1.5 py-0.5 text-xs font-semibold " +
                  (POS_BADGE[p.position] ?? "bg-slate-800 text-slate-300")
                }
              >
                {POS_FULL[p.position] ?? p.position}
              </span>
            )}
            {/* The EA banner puts the archetype here; the corpus has no archetype, so the
                nickname takes the slot (it's the closest "how they were known" value). */}
            {p.nick_name && <span className="text-sm font-medium text-sky-300">“{p.nick_name}”</span>}
          </div>
        )}

        {p.dob && p.dob !== "?" && <p className="text-xs text-slate-400">DOB: {p.dob}</p>}
        {birthplace && (
          <p className="text-xs text-slate-400">
            {birthplace}
            {p.nationality ? ` · ${p.nationality}` : ""}
          </p>
        )}
        {physical.length > 0 && <p className="text-xs text-slate-400">{physical.join(" · ")}</p>}
        <p className="text-xs text-slate-400">
          {span} · {p.seasons.length} {p.seasons.length === 1 ? "season" : "seasons"}
        </p>
      </div>

      {/* Right rail: jersey badge top, Google-search chips stacked in the bottom-right corner —
          the same set as the EA banner (they're just as useful for sourcing a legend's likeness). */}
      <div className="flex shrink-0 flex-col items-end justify-between gap-3 self-stretch">
        {number ? <JerseyBadge number={number} /> : <span />}
        <div className="flex items-end gap-1">
          <div className="flex flex-col gap-1">
            <SearchChip name={fullName} label="Portrait" images />
            <SearchChip name={fullName} label="Equipment" term="playing hockey" images />
          </div>
          <div className="flex flex-col gap-1">
            <SearchChip name={fullName} label="HockeyDB" />
            <SearchChip name={fullName} label="Hockey Reference" />
            <SearchChip name={fullName} label="Elite Prospects" />
          </div>
        </div>
      </div>
    </div>
  );
}

/// The art window. The historical corpus carries no portrait art, so this is always the neutral
/// silhouette — the same tile geometry as the EA banner's `Portrait` so the two pages line up.
function PortraitPlaceholder({ name }: { name: string }) {
  return (
    <div
      className="grid h-28 w-28 shrink-0 place-items-center overflow-hidden rounded-md border border-slate-700 bg-slate-800/70"
      title={`${name} — the historical corpus ships no portrait art`}
    >
      <svg viewBox="0 0 24 24" className="h-14 w-14 text-slate-600" fill="currentColor" aria-hidden>
        <path d="M12 12a5 5 0 1 0 0-10 5 5 0 0 0 0 10Zm0 2c-4.42 0-8 2.24-8 5v1h16v-1c0-2.76-3.58-5-8-5Z" />
      </svg>
    </div>
  );
}

export function heightStr(inches: number | null): string {
  if (inches == null || inches <= 0) return "";
  return `${Math.floor(inches / 12)}'${inches % 12}"`;
}
