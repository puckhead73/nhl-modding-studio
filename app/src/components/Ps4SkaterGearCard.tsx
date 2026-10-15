// Equipment 4.1–4.4 (receipt 52) — a PS4 skater's gear numbers checked against the title's own Creation Zone lists
// (research 2026-09-28 handoff 02b), each with its thumbnail on demand. Reads the working values (pending edits are
// checked too); anything outside the lists is reported, never refused. List keys stay behind disclosure.
import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import type { AssetRootsArgsDto } from "../bindings/AssetRootsArgsDto";
import type { Ps4SkaterGearItemDto } from "../bindings/Ps4SkaterGearItemDto";
import { Card, Pill, Spinner } from "./primitives";

const TONE: Record<string, "ok" | "warn" | "neutral"> = { row: "ok", enum_only_title: "ok", catalogue_only: "warn", outside: "warn", not_in_catalogue: "warn" };
const LABEL: Record<string, string> = {
  row: "Creation Zone item",
  enum_only_title: "in the catalogue",
  catalogue_only: "catalogue only",
  outside: "not in the catalogue",
  not_in_catalogue: "not in the catalogue",
};

function Thumb({ roots, item }: { roots: AssetRootsArgsDto; item: Ps4SkaterGearItemDto }) {
  const q = useQuery({
    queryKey: ["ps4GearThumb", roots.install_dir, item.thumbnail_archive, item.thumbnail_member],
    queryFn: () => call("ps4_gear_thumbnail", { roots, archive: item.thumbnail_archive!, member: item.thumbnail_member! }),
    staleTime: Infinity,
    retry: false,
  });
  if (q.isPending) return <Spinner label="Decoding…" />;
  if (q.isError) return <p className="text-rose-300">{String(q.error)}</p>;
  return <img src={q.data.png_data_url} alt={`${item.label} ${item.value} thumbnail`} className="h-24 w-24 rounded border border-slate-800 bg-slate-950 object-contain" data-testid={`gear-thumb-${item.family}`} />;
}

export function Ps4SkaterGearCard({ projectId, playerId, roots, fieldsKey }: { projectId: number; playerId: number; roots: AssetRootsArgsDto; fieldsKey: string }) {
  const q = useQuery({
    queryKey: ["ps4SkaterGear", projectId, playerId, roots.install_dir, fieldsKey],
    queryFn: () => call("ps4_skater_gear", { projectId, playerId, roots }),
    retry: false,
  });
  const [open, setOpen] = useState<string | null>(null);
  if (q.isPending) return <Spinner label="Checking the gear…" />;
  if (q.isError) return <p className="text-xs text-rose-300">{String(q.error)}</p>;
  const d = q.data;
  if (!d.applies) return null;
  return (
    <Card title="Gear catalogue (PS4)" subtitle="Each stored number checked against this title's own Creation Zone lists.">
      <div className="flex flex-col gap-1 text-xs" data-testid="ps4-skater-gear">
        {d.problem && <p className="text-slate-400">{d.problem}</p>}
        {d.items.map((it) => (
          <div key={it.family} className="flex flex-wrap items-center gap-2">
            <span className="w-20 text-slate-400">{it.label}</span>
            <span className="text-slate-200">{it.text}</span>
            {it.class && <Pill tone={TONE[it.class] ?? "neutral"}>{LABEL[it.class] ?? it.class}</Pill>}
            {it.name ? (
              <span className="text-slate-100" title={`The title's own text for the list key ${it.key ?? ""}`} data-testid={`gear-name-${it.family}`}>
                {it.name}
              </span>
            ) : (
              it.key && (
                <span className="font-mono text-[10px] text-slate-500" title="The list's name key (a dictionary key, not display text); no text for it in this title's dictionary">
                  {it.key}
                </span>
              )
            )}
            {it.thumbnail_member && (
              <button type="button" className="text-sky-300 underline" onClick={() => setOpen(open === it.family ? null : it.family)} data-testid={`gear-open-${it.family}`}>
                {open === it.family ? "hide picture" : "picture"}
              </button>
            )}
            {open === it.family && it.thumbnail_member && (
              <div className="basis-full">
                <Thumb roots={roots} item={it} />
              </div>
            )}
          </div>
        ))}
        {d.catalogue && <p className="text-[11px] text-slate-500">{d.catalogue}</p>}
        {d.notes.map((n) => (
          <p key={n} className="text-[11px] text-slate-500">
            {n}
          </p>
        ))}
      </div>
    </Card>
  );
}
