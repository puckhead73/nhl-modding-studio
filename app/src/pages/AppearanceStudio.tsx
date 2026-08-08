// Appearance Studio — browse a built appearance catalog (catalog.json).
//
// An initial view over the Phase-6 appearance catalog: point at a folder the
// "Build appearance catalog" export wrote, and browse every head, hairstyle and
// facial-hair mesh it captured — players, generics, referees, coaches, trainers.
// Select one to preview its geometry in 3D (decoded from the source container,
// filtered to just that asset's submeshes) alongside its exported textures.
//
// This is the read/browse foundation the mix-and-match editor (give any player
// any head + hair + facial-hair) is built on next.

import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { call } from "../lib/client";
import {
  Button,
  Card,
  EmptyState,
  ErrorBanner,
  PageScroll,
  Pill,
  Spinner,
  Stat,
} from "../components/primitives";
import type { Tone } from "../components/primitives";
import { HeadMeshViewer } from "../components/HeadMeshViewer";
import type { CatalogEntryDto } from "../bindings/CatalogEntryDto";
import type { CatalogManifestDto } from "../bindings/CatalogManifestDto";

const LAST_DIR_KEY = "appearance.catalogDir";

type ClassFilter = "all" | "head" | "hair" | "facialhair";
type OriginFilter = "all" | "player" | "generic" | "coach" | "referee" | "trainer";

const CLASS_LABEL: Record<string, string> = {
  head: "Head",
  hair: "Hair",
  facialhair: "Facial hair",
};

const ORIGIN_TONE: Record<string, Tone> = {
  player: "info",
};

export function AppearanceStudioPage() {
  const [dir, setDir] = useState<string | null>(() => localStorage.getItem(LAST_DIR_KEY));

  const manifest = useQuery({
    queryKey: ["catalogManifest", dir],
    queryFn: () => call("read_catalog_manifest", { dir: dir! }),
    enabled: !!dir,
    retry: false,
  });

  const pick = async () => {
    const chosen = await openDialog({
      multiple: false,
      directory: true,
      title: "Choose a built appearance-catalog folder (contains catalog.json)",
    });
    if (typeof chosen === "string") {
      localStorage.setItem(LAST_DIR_KEY, chosen);
      setDir(chosen);
    }
  };

  return (
    <PageScroll>
      <div className="space-y-3 p-3">
        <Card
          title="Appearance Studio"
          subtitle="Browse a built appearance catalog — every head, hairstyle and facial-hair mesh, players and staff alike."
        >
          <div className="flex items-center gap-3 text-sm">
            <Button onClick={pick}>{dir ? "Choose another folder…" : "Choose catalog folder…"}</Button>
            {dir ? (
              <span className="truncate font-mono text-xs text-slate-500" title={dir}>
                {dir}
              </span>
            ) : (
              <span className="text-xs text-slate-500">
                Build one first under Assets → Export → “Build appearance catalog”.
              </span>
            )}
          </div>
          {manifest.data ? (
            <div className="mt-3 grid grid-cols-4 gap-2 border-t border-slate-800 pt-3">
              <Stat label="Heads" value={manifest.data.head_count} />
              <Stat label="Hair" value={manifest.data.hair_count} />
              <Stat label="Facial hair" value={manifest.data.facial_hair_count} />
              <Stat label="Game" value={manifest.data.game} />
            </div>
          ) : null}
        </Card>

        {!dir ? (
          <EmptyState
            title="No catalog loaded"
            hint="Choose a folder containing a catalog.json to browse its heads, hair and facial hair."
          />
        ) : manifest.isLoading ? (
          <Spinner label="Reading catalog…" />
        ) : manifest.error ? (
          <ErrorBanner
            title="Couldn't read this catalog"
            message={`${String(manifest.error)} — pick a folder that holds a catalog.json.`}
          />
        ) : manifest.data ? (
          <Browser dir={dir} manifest={manifest.data} />
        ) : null}
      </div>
    </PageScroll>
  );
}

function Browser({ dir, manifest }: { dir: string; manifest: CatalogManifestDto }) {
  const [search, setSearch] = useState("");
  const [classF, setClassF] = useState<ClassFilter>("all");
  const [originF, setOriginF] = useState<OriginFilter>("all");
  const [selected, setSelected] = useState<CatalogEntryDto | null>(null);

  // The origins actually present (so the filter only offers real buckets).
  const origins = useMemo(() => {
    const seen = new Set<string>();
    for (const e of manifest.entries) seen.add(e.origin);
    return ["all", ...[...seen].sort()] as OriginFilter[];
  }, [manifest.entries]);

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    return manifest.entries.filter(
      (e) =>
        (classF === "all" || e.class === classF) &&
        (originF === "all" || e.origin === originF) &&
        (q === "" || e.name.toLowerCase().includes(q)),
    );
  }, [manifest.entries, search, classF, originF]);

  // Keep the selection valid as filters change; default to the first match.
  useEffect(() => {
    if (filtered.length === 0) {
      setSelected(null);
    } else if (!selected || !filtered.some((e) => e.name === selected.name)) {
      setSelected(filtered[0]);
    }
  }, [filtered, selected]);

  return (
    <div className="grid grid-cols-1 gap-3 lg:grid-cols-[22rem_1fr]">
      {/* Left: filters + list */}
      <Card title="Catalog" subtitle={`${filtered.length} of ${manifest.entries.length} assets`}>
        <div className="space-y-2">
          <input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search by name…"
            className="w-full rounded-md border border-slate-700 bg-slate-900 px-2 py-1 text-sm text-slate-200 placeholder:text-slate-600"
          />
          <FilterRow
            label="Type"
            value={classF}
            onChange={(v) => setClassF(v as ClassFilter)}
            options={[
              ["all", "All"],
              ["head", "Head"],
              ["hair", "Hair"],
              ["facialhair", "Facial hair"],
            ]}
          />
          <FilterRow
            label="Source"
            value={originF}
            onChange={(v) => setOriginF(v as OriginFilter)}
            options={origins.map((o) => [o, o === "all" ? "All" : titleCase(o)])}
          />
        </div>

        <ul className="mt-2 max-h-[30rem] space-y-0.5 overflow-y-auto border-t border-slate-800 pt-2">
          {filtered.map((e) => (
            <li key={e.name}>
              <button
                onClick={() => setSelected(e)}
                className={`flex w-full items-center gap-2 rounded px-2 py-1 text-left text-xs hover:bg-slate-800/60 ${
                  selected?.name === e.name ? "bg-slate-800" : ""
                }`}
              >
                <span
                  className="inline-block h-2 w-2 shrink-0 rounded-full"
                  style={{ background: classColor(e.class) }}
                />
                <span className="truncate font-mono text-slate-300">{e.name}</span>
                {e.origin === "player" ? null : (
                  <span className="ml-auto shrink-0 text-[10px] text-slate-500">
                    {titleCase(e.origin)}
                  </span>
                )}
              </button>
            </li>
          ))}
          {filtered.length === 0 ? (
            <li className="px-2 py-4 text-center text-xs text-slate-500">No assets match.</li>
          ) : null}
        </ul>
      </Card>

      {/* Right: detail */}
      {selected ? (
        <Detail key={selected.name} dir={dir} entry={selected} />
      ) : (
        <EmptyState title="Nothing selected" hint="Pick an asset on the left to preview it." />
      )}
    </div>
  );
}

function Detail({ dir, entry }: { dir: string; entry: CatalogEntryDto }) {
  return (
    <Card
      title={entry.name}
      subtitle={`${CLASS_LABEL[entry.class] ?? entry.class} · ${titleCase(entry.origin)} · ${entry.game}`}
    >
      <div className="space-y-3">
        <div className="flex flex-wrap items-center gap-1.5">
          <Pill tone={ORIGIN_TONE[entry.origin] ?? "neutral"}>{titleCase(entry.origin)}</Pill>
          <Pill>{CLASS_LABEL[entry.class] ?? entry.class}</Pill>
          <Pill>{entry.submeshes.length} submeshes</Pill>
          <Pill>{entry.textures.length} textures</Pill>
        </div>

        {/* Geometry preview: decoded from the source container, filtered to this
            asset's own submeshes and framed to them. */}
        <HeadMeshViewer
          catalogEntry={{
            source: entry.source,
            submeshes: entry.submeshes,
            textureSources: entry.texture_sources,
          }}
        />

        <TextureStrip dir={dir} textures={entry.textures} />

        <details className="text-xs text-slate-400">
          <summary className="cursor-pointer select-none text-slate-500">Details</summary>
          <div className="mt-1 space-y-1">
            <Row label="Mesh file" value={entry.mesh_file} />
            <Row label="Submeshes" value={entry.submeshes.join(", ") || "—"} />
            <Row label="Source" value={entry.source} />
          </div>
        </details>
      </div>
    </Card>
  );
}

// Exported catalog textures shown as thumbnails — the PNGs live in the catalog
// folder, so join each catalog-relative path onto the folder and read it back.
function TextureStrip({ dir, textures }: { dir: string; textures: string[] }) {
  if (textures.length === 0) {
    return (
      <div className="rounded-md border border-slate-800 bg-slate-900/50 p-2 text-center text-[10px] text-slate-500">
        No textures were exported for this asset.
      </div>
    );
  }
  return (
    <div className="rounded-md border border-slate-800 bg-slate-900/50 p-2">
      <div className="mb-1.5 text-[10px] font-semibold uppercase tracking-wide text-slate-500">
        Textures — {textures.length} {textures.length === 1 ? "map" : "maps"}
      </div>
      <div className="flex flex-wrap gap-3">
        {textures.map((t) => (
          <Thumb key={t} dir={dir} rel={t} />
        ))}
      </div>
    </div>
  );
}

function Thumb({ dir, rel }: { dir: string; rel: string }) {
  const path = `${dir}/${rel}`;
  const img = useQuery({
    queryKey: ["catalogPng", path],
    queryFn: () => call("read_png_data_url", { path }),
    retry: false,
  });
  const name = rel.split("/").pop() ?? rel;
  return (
    <figure className="flex flex-col items-center gap-1">
      <div
        className="grid h-24 w-24 place-items-center overflow-hidden rounded border border-slate-700"
        style={{
          backgroundImage:
            "linear-gradient(45deg, #334155 25%, transparent 25%), linear-gradient(-45deg, #334155 25%, transparent 25%), linear-gradient(45deg, transparent 75%, #334155 75%), linear-gradient(-45deg, transparent 75%, #334155 75%)",
          backgroundSize: "12px 12px",
          backgroundPosition: "0 0, 0 6px, 6px -6px, -6px 0",
          backgroundColor: "#1e293b",
        }}
      >
        {img.data ? (
          <img src={img.data} alt={name} className="max-h-24 max-w-24" style={{ imageRendering: "pixelated" }} />
        ) : (
          <span className="text-[9px] text-slate-600">{img.error ? "—" : "…"}</span>
        )}
      </div>
      <figcaption className="max-w-24 truncate text-center font-mono text-[10px] text-slate-400" title={name}>
        {name}
      </figcaption>
    </figure>
  );
}

function FilterRow({
  label,
  value,
  onChange,
  options,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  options: [string, string][];
}) {
  return (
    <div className="flex items-center gap-1.5">
      <span className="w-12 shrink-0 text-[10px] font-semibold uppercase tracking-wide text-slate-500">
        {label}
      </span>
      <div className="flex flex-wrap gap-1">
        {options.map(([key, lbl]) => (
          <button
            key={key}
            onClick={() => onChange(key)}
            className={`rounded border px-1.5 py-0.5 text-[11px] ${
              value === key
                ? "border-sky-600 bg-sky-950/60 text-sky-300"
                : "border-slate-700 bg-slate-900 text-slate-400 hover:text-slate-200"
            }`}
          >
            {lbl}
          </button>
        ))}
      </div>
    </div>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex gap-2">
      <span className="w-20 shrink-0 text-slate-500">{label}</span>
      <span className="break-all font-mono text-slate-400">{value}</span>
    </div>
  );
}

function classColor(cls: string): string {
  return cls === "hair" ? "#a3e635" : cls === "facialhair" ? "#fbbf24" : "#38bdf8";
}

function titleCase(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}
