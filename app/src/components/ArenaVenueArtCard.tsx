// Presentation 5.2 (receipt 49) — the read-only "Venue art" gallery on the Arena page.
//
// Lists the arena's gen7 venue packages (research-measured titles only): the board-ad / bench-ad / jumbotron /
// power-ring / glass materials by name with the raster their parameter block names, and every raster with its size.
// Xbox 360 rasters preview on demand; the preview names the package's SHA-256 from the listing, so a changed file is
// refused instead of shown. A material's name is not proof the game draws that raster.
//
// Receipt 67 (C37): on PS3 a board-ad material's raster (DXT1 / DXT3) offers "Replace…" — a same-size image becomes a pending
// change (archive member, plus the loose copy on NHL 14 / 15 / Legacy) that the next copied-target save writes; nothing is
// written from this card.
//
// Result ownership: keyed on project + arena + install root + platform; a preview is keyed on the package hash.

import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useAssetRootArgs } from "../lib/project";
import type { VenuePackageDto } from "../bindings/VenuePackageDto";
import type { VenueSlotDto } from "../bindings/VenueSlotDto";
import { Card, Spinner } from "./primitives";
import { useBoardReplace } from "./BoardReplace";

function slotText(pkg: VenuePackageDto, s: VenueSlotDto | undefined): string {
  if (!s) return "—";
  if (s.raster != null) {
    const r = pkg.rasters[s.raster];
    return r ? `raster ${s.raster} · ${r.width} × ${r.height} ${r.format}` : `raster ${s.raster}`;
  }
  if (s.raw != null) return `stored ${s.raw} (no raster)`;
  return s.unreadable ? "not readable" : "—";
}

function Preview({ projectId, arenaId, pkg, ordinal }: { projectId: number; arenaId: number; pkg: VenuePackageDto; ordinal: number }) {
  const { rootArgs } = useAssetRootArgs();
  const q = useQuery({
    queryKey: ["venueRaster", projectId, arenaId, pkg.file, pkg.sha256, ordinal],
    queryFn: () => call("arena_venue_raster", { projectId, roots: rootArgs, arenaId, file: pkg.file, sha256: pkg.sha256, ordinal, maxEdge: 512 }),
    staleTime: Infinity,
    retry: false,
  });
  if (q.isPending) return <Spinner label="Decoding…" />;
  if (q.isError) return <p className="text-xs text-rose-300" data-testid="venue-preview-error">{String(q.error)}</p>;
  const r = pkg.rasters[ordinal];
  return (
    <figure className="flex flex-col items-start gap-1">
      <img src={q.data.png_data_url} alt={`Raster ${ordinal} of ${pkg.file} (${q.data.width} × ${q.data.height})`} className="max-h-80 max-w-full rounded border border-slate-800 bg-slate-950" data-testid="venue-preview" />
      <figcaption className="text-[11px] text-slate-400">
        raster {ordinal} · {q.data.width} × {q.data.height} {r?.format}
        {r && r.used_by.length > 0 ? ` · named by ${r.used_by.join(", ")}` : " · named by no presentation material"}
      </figcaption>
    </figure>
  );
}

export function ArenaVenueArtCard({ projectId, arenaId }: { projectId: number; arenaId: number }) {
  const { installDir, rootArgs } = useAssetRootArgs();
  const q = useQuery({
    queryKey: ["venueArt", projectId, arenaId, installDir ?? "", rootArgs.platform ?? "", rootArgs.rendering_override ?? ""],
    queryFn: () => call("arena_venue_art", { projectId, roots: rootArgs, arenaId }),
    enabled: Number.isFinite(arenaId),
    staleTime: Infinity,
    retry: false,
  });
  const [open, setOpen] = useState<{ file: string; ordinal: number } | null>(null);
  // Receipt 67 (C37): PS3 board rasters can be replaced (same size, DXT1 / DXT3, every level; a pending change).
  const board = useBoardReplace(projectId, installDir ?? null);
  const [showAll, setShowAll] = useState(false);

  if (q.isPending) return <Card title="Venue art"><Spinner label="Reading the venue packages…" /></Card>;
  if (q.isError) return <Card title="Venue art"><p className="text-xs text-rose-300">{String(q.error)}</p></Card>;
  const v = q.data;
  if (!v) return null;
  return (
    <Card
      title="Venue art"
      subtitle="Materials are named by their parameter block's raster (parsed data); a name does not prove what the game draws. PS3 board-ad rasters can be replaced with a same-size image as a pending change; everything else here is read-only."
    >
      <div className="flex flex-col gap-3 text-xs" data-testid="venue-art">
        {!v.measured && <p className="text-slate-400" data-testid="venue-unmeasured">{v.notes.join(" ")}</p>}
        {v.measured && v.notes.length > 0 && (
          <ul className="list-disc pl-5 text-slate-400">
            {v.notes.map((n) => (
              <li key={n}>{n}</li>
            ))}
          </ul>
        )}
        {v.packages.map((p) => (
          <section key={p.file} className="rounded border border-slate-800 p-2" aria-label={p.file}>
            <header className="mb-1 flex flex-wrap items-baseline gap-2">
              <span className="font-mono text-slate-200">{p.file}</span>
              <span className="text-slate-500">
                {(Number(p.bytes) / 1048576).toFixed(1)} MB · {p.rasters.length} rasters · {p.materials.length} presentation materials · {p.other_materials} others
              </span>
              <span className="font-mono text-[10px] text-slate-600" title={p.sha256}>
                {p.sha256.slice(0, 12)}…
              </span>
            </header>
            {p.notes.length > 0 && <p className="text-amber-300/80">{p.notes.join(" ")}</p>}
            <table className="w-full border-collapse">
              <thead>
                <tr className="text-left text-slate-500">
                  <th className="pr-2 font-normal">Material</th>
                  <th className="pr-2 font-normal">Kind (by name)</th>
                  <th className="pr-2 font-normal">Slot 0</th>
                  <th className="pr-2 font-normal">Slot 1</th>
                  <th className="pr-2 font-normal" />
                </tr>
              </thead>
              <tbody>
                {p.materials.map((m, i) => {
                  const s0 = m.slots[0];
                  const r = s0?.raster != null ? p.rasters[s0.raster] : undefined;
                  return (
                    <tr key={`${i}-${m.name}`} className="border-t border-slate-800/60">
                      <td className="pr-2 font-mono text-slate-200" title={m.shader ?? undefined}>{m.name}</td>
                      <td className="pr-2 text-slate-400">{m.class}</td>
                      <td className="pr-2">
                        {r && r.previewable ? (
                          <button
                            type="button"
                            className="text-sky-300 underline hover:text-sky-200 focus:outline-none focus:ring-1 focus:ring-sky-600"
                            onClick={() => setOpen({ file: p.file, ordinal: s0!.raster! })}
                            data-testid={`venue-open-${m.name}`}
                          >
                            {slotText(p, s0)}
                          </button>
                        ) : (
                          <span className="text-slate-300" title={r?.note ?? s0?.unreadable ?? undefined}>{slotText(p, s0)}</span>
                        )}
                      </td>
                      <td className="pr-2 text-slate-400" title={m.slots[1]?.unreadable ?? undefined}>{slotText(p, m.slots[1])}</td>
                      <td className="pr-2">
                        {p.platform === "ps3" && m.class === "Board ad" && r && (r.format === "DXT1" || r.format === "DXT3") ? (
                          <button
                            type="button"
                            className="text-amber-300 underline hover:text-amber-200 focus:outline-none focus:ring-1 focus:ring-amber-600"
                            onClick={() => void board.pick(p.file.replace(/^stadium\//, ""), r.descriptor, m.name)}
                            data-testid={`venue-replace-${m.name}`}
                          >
                            Replace…
                          </button>
                        ) : null}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
            {open && open.file === p.file && (
              <div className="mt-2">
                <Preview projectId={projectId} arenaId={arenaId} pkg={p} ordinal={open.ordinal} />
              </div>
            )}
            <details className="mt-2" open={showAll} onToggle={(e) => setShowAll((e.target as HTMLDetailsElement).open)}>
              <summary className="cursor-pointer text-slate-400">Every raster in this package ({p.rasters.length})</summary>
              <ul className="mt-1 grid gap-x-4 sm:grid-cols-2">
                {p.rasters.map((r) => (
                  <li key={r.ordinal}>
                    {r.previewable ? (
                      <button type="button" className="text-sky-300 underline" onClick={() => setOpen({ file: p.file, ordinal: r.ordinal })}>
                        raster {r.ordinal} · {r.width} × {r.height} {r.format}
                      </button>
                    ) : (
                      <span className="text-slate-400" title={r.note ?? undefined}>
                        raster {r.ordinal} · {r.width} × {r.height} {r.format} (no preview)
                      </span>
                    )}
                    {r.used_by.length > 0 && <span className="text-slate-500"> — {r.used_by.join(", ")}</span>}
                  </li>
                ))}
              </ul>
            </details>
          </section>
        ))}
        {board.result}
        {board.dialog}
        <p className="text-[11px] text-slate-500">Scope: {v.scope}.</p>
      </div>
    </Card>
  );
}
