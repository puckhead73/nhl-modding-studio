// Equipment 4.5 G3 (receipt 51) — a PS4 goalie's stored (mask, helmet type) pair checked against the title's own vault
// catalogue (research 2026-09-28 handoff 02). Reads the working values, so a pending edit shows its own check — a
// compatibility note, never a refusal. Internal pack / file names stay behind disclosure; no brand names are shown.
import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import type { AssetRootsArgsDto } from "../bindings/AssetRootsArgsDto";
import { Card, Pill, Spinner } from "./primitives";

const TONE: Record<string, "ok" | "warn" | "neutral"> = {
  unique: "ok",
  not_a_catalogue_helmet_type: "warn",
  only_other_helmet_types: "warn",
  absent: "warn",
  ambiguous: "warn",
};
const LABEL: Record<string, string> = {
  unique: "in the catalogue",
  not_a_catalogue_helmet_type: "helmet type not in the catalogue",
  only_other_helmet_types: "other helmet types only",
  absent: "no catalogue record",
  ambiguous: "ambiguous",
};

function MaskPicture({ roots, archive, member, basis }: { roots: AssetRootsArgsDto; archive: string; member: string; basis: string | null }) {
  const q = useQuery({
    queryKey: ["ps4GearThumb", roots.install_dir, archive, member],
    queryFn: () => call("ps4_gear_thumbnail", { roots, archive, member }),
    staleTime: Infinity,
    retry: false,
  });
  if (q.isPending) return <Spinner label="Decoding…" />;
  if (q.isError) return <p className="text-rose-300">{String(q.error)}</p>;
  return (
    <figure className="flex flex-col gap-1">
      <img src={q.data.png_data_url} alt="Mask thumbnail" className="h-28 w-28 rounded border border-slate-800 bg-slate-950 object-contain" data-testid="mask-thumb" />
      <figcaption className="text-[11px] text-slate-500">
        {basis === "name_convention_candidate" ? "Picture by the list's naming rule (a candidate, not a list row)" : "Picture named by the title's Creation Zone mask list"}
      </figcaption>
    </figure>
  );
}

export function Ps4GoalieMaskPairCard({ projectId, playerId, roots, fieldsKey }: { projectId: number; playerId: number; roots: AssetRootsArgsDto; fieldsKey: string }) {
  const q = useQuery({
    queryKey: ["ps4GoalieMaskPair", projectId, playerId, roots.install_dir, fieldsKey],
    queryFn: () => call("ps4_goalie_mask_pair", { projectId, playerId, roots }),
    retry: false,
  });
  const [show, setShow] = useState(false);
  if (q.isPending) return <Spinner label="Checking the mask pair…" />;
  if (q.isError) return <p className="text-xs text-rose-300">{String(q.error)}</p>;
  const d = q.data;
  if (!d.applies) return null;
  return (
    <Card title="Mask pair (PS4)" subtitle="The stored mask and helmet type checked together against this title's own mask catalogue.">
      <div className="flex flex-col gap-1 text-xs" data-testid="ps4-mask-pair">
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-slate-200">{d.text}</span>
          {d.mask_name && (
            <span className="text-slate-100" title="The Creation Zone list row's name, in the title's own text" data-testid="mask-name">
              {d.mask_name}
            </span>
          )}
          {d.class && <Pill tone={TONE[d.class] ?? "neutral"}>{LABEL[d.class] ?? d.class}</Pill>}
          {d.thumbnail_member && (
            <button type="button" className="text-sky-300 underline" onClick={() => setShow(!show)} data-testid="mask-open">
              {show ? "hide picture" : "picture"}
            </button>
          )}
        </div>
        {show && d.thumbnail_archive && d.thumbnail_member && <MaskPicture roots={roots} archive={d.thumbnail_archive} member={d.thumbnail_member} basis={d.thumbnail_basis ?? null} />}
        {d.class && d.class !== "unique" && (
          <p className="text-amber-200/80">Reported, not refused: whether the game rejects this pair is not known.</p>
        )}
        {d.catalogue_problem && <p className="text-slate-400">{d.catalogue_problem}</p>}
        {(d.pack_debug_name || d.catalogue) && (
          <details>
            <summary className="cursor-pointer text-slate-500">Details</summary>
            {d.pack_debug_name && <p className="font-mono text-slate-400">{d.pack_debug_name}</p>}
            {d.colour_map && <p className="font-mono text-slate-400">{d.colour_map}</p>}
            {d.catalogue && <p className="text-slate-500">{d.catalogue}</p>}
          </details>
        )}
        {d.notes.map((n) => (
          <p key={n} className="text-[11px] text-slate-500">{n}</p>
        ))}
      </div>
    </Card>
  );
}
