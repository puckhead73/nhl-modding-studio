import { Link } from "react-router-dom";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { usePlayerAssets } from "../pages/PlayerAssets";
import { POS_BADGE, POS_FULL } from "../lib/position";
import type { PlayerViewDto } from "../bindings/PlayerViewDto";

/// The Player detail banner — portrait window, name + main-team pill, then the bio stack:
/// position badge (full name, list-style tones) + player type, DOB, birthplace, and
/// handedness/weight/height. All values come pre-resolved from the backend `header` (enum labels,
/// formatted height/weight/date, resolved team) per the human-readable-surfaces rule — the
/// frontend renders them verbatim. Fields the EA bio doesn't carry stay `null` and drop out.
export function PlayerHeader({ player }: { player: PlayerViewDto }) {
  const h = player.header;
  const fullName = h.full_name || `Player #${player.player_id}`;
  const isGoalie = player.kind === "goalie";

  const physical = [
    h.handedness_label && `${isGoalie ? "Catches" : "Shoots"} ${h.handedness_label}`,
    h.weight,
    h.height,
  ].filter(Boolean);

  const number = h.jersey_number != null ? String(h.jersey_number) : "";

  return (
    <div className="flex items-start gap-4 rounded-lg border border-slate-800 bg-gradient-to-r from-slate-900/80 to-slate-900/30 p-4">
      <Portrait name={fullName} bioRecord={player.bio_record} />

      <div className="min-w-0 flex-1 space-y-1.5">
        <div className="flex items-center gap-2">
          <h1 className="truncate text-2xl font-semibold leading-tight text-slate-50">{fullName}</h1>
          {h.primary_team_id != null ? (
            <Link
              to={`/app/database/teams/${h.primary_team_id}`}
              className="shrink-0 rounded-full border border-emerald-700/60 bg-emerald-900/40 px-2.5 py-0.5 text-xs font-medium text-emerald-300 hover:bg-emerald-900/60"
            >
              {h.primary_team_name || `Team ${h.primary_team_id}`}
            </Link>
          ) : (
            <span className="shrink-0 rounded-full border border-slate-700 bg-slate-800/60 px-2.5 py-0.5 text-xs font-medium text-slate-300">
              Free Agent
            </span>
          )}
        </div>

        {(h.position_label || h.style_label) && (
          <div className="flex flex-wrap items-center gap-2">
            {h.position_label && (
              <span
                className={
                  "inline-flex items-center rounded px-1.5 py-0.5 text-xs font-semibold " +
                  (POS_BADGE[h.position_label] ?? "bg-slate-800 text-slate-300")
                }
              >
                {POS_FULL[h.position_label] ?? h.position_label}
              </span>
            )}
            {h.style_label && (
              <span className="text-sm font-medium text-sky-300">{h.style_label}</span>
            )}
          </div>
        )}

        {h.birthdate && <p className="text-xs text-slate-400">DOB: {h.birthdate}</p>}
        {h.birthplace && <p className="text-xs text-slate-400">{h.birthplace}</p>}
        {physical.length > 0 && (
          <p className="text-xs text-slate-400">{physical.join(" · ")}</p>
        )}
      </div>

      {/* Right rail: jersey badge top, Google-search chips stacked 2×2 in the bottom-right
          corner — each searches "PLAYER NAME <term>" in the default browser. */}
      <div className="flex shrink-0 flex-col items-end justify-between gap-3 self-stretch">
        {number ? <JerseyBadge number={number} /> : <span />}
        <div className="flex items-end gap-1">
          <div className="flex flex-col gap-1">
            <SearchChip name={fullName} label="Portrait" images />
            {/* "playing hockey" surfaces action shots — the best view of the gear they wear. */}
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

/// The player portrait window. Finds this player's portrait file from the (cached, app-wide) asset
/// scan and decodes it **bottom-framed** (`portrait_header_preview`): cropped to the content and down
/// to its lowest solid row, so the solid bottom of the portrait pins to the tile bottom, the content
/// stands 95% of the tile tall, and any width wider than the tile is clipped (never shrunk). Falls
/// back to a neutral silhouette while loading or when the player has no portrait.
function Portrait({ name, bioRecord }: { name: string; bioRecord: number }) {
  const { assets } = usePlayerAssets();
  const portraitPath =
    assets.data?.find((a) => a.bio_record === bioRecord)?.portrait_path ?? null;
  const preview = useQuery({
    queryKey: ["portraitHeaderPreview", portraitPath],
    queryFn: () => call("portrait_header_preview", { path: portraitPath! }),
    enabled: !!portraitPath,
    staleTime: Infinity,
  });

  return (
    <div
      className="relative grid h-28 w-28 shrink-0 place-items-center overflow-hidden rounded-md border border-slate-700 bg-slate-800/70"
      title={preview.data ? `${name} portrait` : `${name} — no portrait art found`}
    >
      {preview.data ? (
        // Pin the (solid) bottom to the tile, stand 95% tall, center horizontally; the tile's
        // overflow-hidden clips any width that spills past the sides (not shrunk to fit).
        <img
          src={preview.data}
          alt={`${name} portrait`}
          className="absolute bottom-0 left-1/2 h-[95%] w-auto max-w-none -translate-x-1/2"
        />
      ) : (
        <svg viewBox="0 0 24 24" className="h-14 w-14 text-slate-600" fill="currentColor" aria-hidden>
          <path d="M12 12a5 5 0 1 0 0-10 5 5 0 0 0 0 10Zm0 2c-4.42 0-8 2.24-8 5v1h16v-1c0-2.76-3.58-5-8-5Z" />
        </svg>
      )}
    </div>
  );
}

function JerseyBadge({ number }: { number: string }) {
  return (
    <div className="grid h-14 w-14 shrink-0 place-items-center rounded-md border-2 border-amber-500/70 bg-slate-900/60 text-2xl font-bold tabular-nums text-slate-100">
      {number}
    </div>
  );
}

/** A Google-search chip: opens `"<name> <term>"` (term defaults to the button label) in the
 *  system default browser — as an image search when `images` is set. A plain `target="_blank"`
 *  anchor is a no-op inside the Tauri webview, so this goes through the opener plugin instead. */
function SearchChip({
  name,
  label,
  term,
  images,
}: {
  name: string;
  label: string;
  term?: string;
  images?: boolean;
}) {
  const query = `${name} ${term ?? label}`;
  const url = `https://www.google.com/search?q=${encodeURIComponent(query)}${images ? "&tbm=isch" : ""}`;
  return (
    <button
      type="button"
      title={`Google ${images ? "Images" : ""} "${query}"`}
      onClick={() => void openUrl(url)}
      className="rounded border border-slate-700 px-1.5 py-px text-center text-[10px] font-medium leading-4 text-slate-400 transition-colors hover:bg-slate-800 hover:text-slate-100"
    >
      {label}
    </button>
  );
}
