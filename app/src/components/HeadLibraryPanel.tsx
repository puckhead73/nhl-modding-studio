import { useEffect, useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import * as THREE from "three";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import { LoadedViewer } from "./HeadMeshViewer";
import { ensurePreviewNormals } from "../lib/headPreviewGeometry";
import { foldHeadName, headPart as role, previewColorMaps } from "../lib/headLibraryMatching";
import type { HeadPreviewDto } from "../bindings/HeadPreviewDto";
import type { PreviewSubmeshDto } from "../bindings/PreviewSubmeshDto";
import type { PreviewTextureDto } from "../bindings/PreviewTextureDto";

type Asset = { id: string; class: string; stem: string; mesh_file: string; game: string; platform: string; origin: string; lod: number; sources: string };
type Appearance = { id: number; identity: string | null; game: string; platform: string; source: string; original?: { layer?: string; patch_version?: string }; textures: { name: string; file: string }[] };
function gameYear(game: string) {
  if (game.toLowerCase().includes("legacy")) return 2016;
  const digits = Number(game.replace(/\D/g, ""));
  return digits < 100 ? 2000 + digits : digits;
}
function sourceLabel(asset: Asset) {
  const game = asset.game.toLowerCase().includes("legacy") ? "NHL Legacy" : asset.game.toUpperCase();
  const platform: Record<string, string> = { xbox360: "XBOX 360", ps3: "PS3", ps4: "PS4", ps5: "PS5" };
  return `${game} - ${platform[asset.platform.toLowerCase()] ?? asset.platform.toUpperCase()}`;
}
const control = "rounded border border-slate-700 bg-slate-950 px-2 py-1 text-sm";


async function preview(asset: Asset, appearance: Appearance): Promise<HeadPreviewDto> {
  const data = await invoke<{ base64: string }>("head_library_file", { path: asset.mesh_file });
  const bytes = Uint8Array.from(atob(data.base64), c => c.charCodeAt(0));
  const gltf = await new GLTFLoader().parseAsync(bytes.buffer, "");
  // Keep native UV convention: the shared viewer uses flipY=false.
  const maps = previewColorMaps(appearance.textures, appearance.game);
  const textures = await Promise.all(maps.map(async t => ({
    name: t.name, ...await invoke<Omit<PreviewTextureDto, "name">>("head_library_file", { path: t.file }),
  })));
  const submeshes: PreviewSubmeshDto[] = [];
  const box = new THREE.Box3();
  gltf.scene.updateMatrixWorld(true);
  gltf.scene.traverse(object => {
    if (!(object instanceof THREE.Mesh)) return;
    const geometry = object.geometry.clone().applyMatrix4(object.matrixWorld);
    ensurePreviewNormals(geometry, ["head", "eye"].includes(role(object.name)));
    const p = geometry.getAttribute("position"), n = geometry.getAttribute("normal"), uv = geometry.getAttribute("uv");
    const positions: [number, number, number][] = [], normals: [number, number, number][] = [], uv1: [number, number][] = [];
    for (let i = 0; i < p.count; i++) {
      positions.push([p.getX(i), p.getY(i), p.getZ(i)]);
      normals.push([n.getX(i), n.getY(i), n.getZ(i)]);
      uv1.push(uv ? [uv.getX(i), uv.getY(i)] : [0, 0]);
      box.expandByPoint(new THREE.Vector3(...positions[i]));
    }
    const triangles: [number, number, number][] = [];
    const ix = geometry.index;
    for (let i = 0; i < (ix?.count ?? p.count); i += 3)
      triangles.push(ix ? [ix.getX(i), ix.getX(i + 1), ix.getX(i + 2)] : [i, i + 1, i + 2]);
    const kind = role(object.name);
    const material = textures.findIndex(t => role(t.name) === kind);
    submeshes.push({ name: object.name, positions, normals, uv1, triangles,
      material_index: material >= 0 ? material : null, transparent: ["hair", "facial", "eyelash"].includes(kind),
      render_order: kind === "head" ? 0 : 1 });
    geometry.dispose();
    object.geometry.dispose();
    const materials = Array.isArray(object.material) ? object.material : [object.material];
    materials.forEach(m => m.dispose());
  });
  if (box.isEmpty()) throw new Error("Library mesh contains no geometry");
  const center = box.getCenter(new THREE.Vector3()).toArray() as [number, number, number];
  return { submeshes, textures, center, span: box.getSize(new THREE.Vector3()).length(),
    source_path: asset.mesh_file, lenient_rig: false };
}

export function HeadLibraryPanel({ name = "", personKey = "" }: { name?: string; personKey?: string }) {
  const [query, setQuery] = useState(name);
  const [kind, setKind] = useState("head");
  const [selected, setSelected] = useState<Asset | null>(null);
  const [appearanceIndex, setAppearanceIndex] = useState(0);
  const [companions, setCompanions] = useState<Record<string, Asset>>({});
  const [savedAppearance, setSavedAppearance] = useState<number | null>(null);
  const [selectionQuery, setSelectionQuery] = useState("");
  const [selectionNotice, setSelectionNotice] = useState("");
  const [activeQuery, setActiveQuery] = useState(name);
  useEffect(() => {
    setQuery(name); setActiveQuery(name); setKind("head");
    setSelected(null); setCompanions({}); setAppearanceIndex(0);
    setSavedAppearance(null); setSelectionQuery(""); setSelectionNotice("");
    if (!personKey) return;
    try {
      const saved = JSON.parse(localStorage.getItem(`head-library-selection:${personKey}`) ?? "null");
      if (saved?.version === 1 && typeof saved.asset?.id === "string") {
        setSelected(saved.asset); setCompanions(saved.companions ?? {});
        setSavedAppearance(saved.appearance);
        setSelectionNotice("Restored saved library appearance.");
      }
    } catch { setSelectionNotice("Saved appearance could not be read. Choose a new selection."); }
  }, [personKey, name]);
  useEffect(() => { const timer = setTimeout(() => { setActiveQuery(query); }, 250); return () => clearTimeout(timer); }, [query]);
  const status = useQuery({ queryKey: ["headLibraryStatus-patches"], queryFn: () => invoke<{ available: boolean; assets: number; root: string }>("head_library_status") });
  const results = useQuery({ queryKey: ["headLibraryAll-patches", activeQuery, kind], enabled: !!status.data?.available,
    queryFn: async ({ signal }) => {
      const matches: Asset[] = [];
      // Read the API's bounded batches into one ordered, scrollable selector.
      for (let offset = 0; ; offset += 100) {
        signal.throwIfAborted();
        const batch = await invoke<Asset[]>("head_library_search", { query: foldHeadName(activeQuery), class: kind, offset });
        matches.push(...batch);
        if (batch.length < 100) break;
      }
      return matches.sort((a, b) => gameYear(b.game) - gameYear(a.game) || a.platform.localeCompare(b.platform) || a.lod - b.lod || a.id.localeCompare(b.id));
    } });
  const details = useQuery({ queryKey: ["headLibraryDetail-patches", selected?.id], enabled: !!selected,
    queryFn: () => invoke<Appearance[]>("head_library_detail", { id: selected!.id }) });
  useEffect(() => {
    const fresh = results.data?.find(a => a.id === selected?.id);
    if (fresh && selected && fresh.mesh_file !== selected.mesh_file) setSelected(fresh);
  }, [results.data, selected]);
  useEffect(() => {
    if (selected && details.data?.length === 0 && !details.isFetching) {
      setSelected(null); setCompanions({}); setSavedAppearance(null);
      setSelectionNotice("This mesh was replaced during a library update. Showing an available match.");
    }
  }, [details.data, details.isFetching, selected]);
  // Opening a player should show a model immediately, without a hidden selection prerequisite.
  useEffect(() => {
    if (selected || !results.data?.length || results.isFetching) return;
    const terms = foldHeadName(activeQuery).split(/\s+/).filter(Boolean);
    const first = results.data.find(a => a.lod === 0 && terms.every(t => foldHeadName(a.stem).includes(t)))
      ?? results.data.find(a => a.lod === 0) ?? results.data[0];
    setSelected(first); setAppearanceIndex(0); setSelectionQuery(activeQuery);
  }, [results.data, results.isFetching, selected, activeQuery]);
  useEffect(() => {
    if (savedAppearance === null || !details.data) return;
    const index = details.data.findIndex(a => a.id === savedAppearance);
    setAppearanceIndex(Math.max(0, index)); setSavedAppearance(null);
    if (index < 0) setSelectionNotice("Saved texture set is unavailable; showing the first available source.");
  }, [details.data, savedAppearance]);
  useEffect(() => {
    if (!details.data || !selectionQuery || savedAppearance !== null) return;
    const terms = foldHeadName(selectionQuery).split(/\s+/).filter(Boolean);
    const index = details.data.findIndex(a => a.identity && terms.every(t => foldHeadName(a.identity!).includes(t)));
    if (index >= 0) setAppearanceIndex(index);
  }, [details.data, selectionQuery, savedAppearance]);
  const appearance = details.data?.[appearanceIndex];
  const rendered = useQuery({ queryKey: ["headLibraryPreview-v5", selected?.id, selected?.mesh_file, appearance?.id, Object.values(companions).map(a => a.id)], enabled: !!selected && !!appearance,
    queryFn: async () => {
      const result = await preview(selected!, appearance!);
      for (const companion of Object.values(companions)) {
        const variants = await invoke<Appearance[]>("head_library_detail", { id: companion.id });
        if (!variants.length) throw new Error(`No source appearance for ${companion.stem}`);
        const extra = await preview(companion, variants[0]);
        const shift = result.textures.length;
        result.textures.push(...extra.textures);
        result.submeshes.push(...extra.submeshes.map(s => ({ ...s, material_index: s.material_index === null ? null : s.material_index + shift })));
      }
      return result;
    } });
  const exporting = useMutation({ mutationFn: async () => {
    const parent = await open({ directory: true, multiple: false, title: "Choose export parent folder" });
    if (typeof parent !== "string" || !selected || !appearance) return null;
    return invoke<{ destination: string }>("head_library_export", { id: selected.id, appearance: appearance.id,
      companions: Object.values(companions).map(a => a.id),
      destination: `${parent}/${selected.stem}-${Date.now()}` });
  } });
  return <section className="space-y-3 rounded border border-slate-800 p-3">
    <div className="flex items-center gap-3"><h3 className="font-semibold">Headmesh library</h3>
      <span className="text-xs text-slate-400">{status.data?.assets.toLocaleString() ?? "…"} unique assets</span></div>
    <div className="flex gap-2">
      <input className={`${control} flex-1`} aria-label="Search headmesh library" placeholder="Search player name or slug" value={query} onChange={e => setQuery(e.target.value)} />
      <select aria-label="Asset type" className={control} value={kind} onChange={e => { setKind(e.target.value); }}>
        <option value="head">Heads</option><option value="hair">Hair</option><option value="facialhair">Facial hair</option><option value="">All</option>
      </select>
    </div>
    {status.data && !status.data.available && <p>Place the compiled library at {status.data.root}.</p>}
    <div className="grid gap-3 lg:grid-cols-[18rem_1fr]">
      <div className="max-h-[40rem] overflow-auto space-y-1">
        {results.isFetching && <p role="status">Searching headmesh library…</p>}
        {!results.isFetching && results.data?.length === 0 && <p>No matching headmeshes. Try a surname or another spelling.</p>}
        {results.data?.map(a => <div key={a.id} className={`rounded border p-2 ${selected?.id === a.id ? "border-sky-500" : "border-slate-800"}`}>
          <button className="w-full text-left text-xs" aria-pressed={selected?.id === a.id} onClick={() => { setSelected(a); setAppearanceIndex(0); setCompanions({}); setSavedAppearance(null); setSelectionQuery(activeQuery); setSelectionNotice(""); }}>
            {sourceLabel(a)}
          </button>
          {selected?.class === "head" && a.class !== "head" && <button className="block text-xs text-sky-400" onClick={() => setCompanions(current => ({ ...current, [a.class]: a }))}>Preview with selected head</button>}
        </div>)}

      </div>
      <div className="space-y-2">
        {!selected && <div className="flex min-h-[360px] items-center justify-center rounded border border-slate-700 bg-slate-950 p-6 text-slate-400">
          {results.isFetching ? "Finding a headmesh to preview…" : "Select a headmesh from the library to open the 3D preview."}
        </div>}
        {details.data && <select className={control} aria-label="Source texture set" value={appearanceIndex} onChange={e => setAppearanceIndex(Number(e.target.value))}>
          {details.data.map((a, i) => <option key={a.id} value={i}>{a.game} / {a.platform}{a.original?.layer === "Patch" ? ` / Patch${a.original.patch_version ? ` ${a.original.patch_version}` : ""}` : ""} — {a.identity ?? "unresolved identity"}</option>)}
        </select>}
        {rendered.isFetching && <p>Loading mesh and textures…</p>}
        {rendered.data && <LoadedViewer data={rendered.data} />}
        {Object.entries(companions).map(([key, asset]) => <button key={key} className={control} onClick={() => setCompanions(current => {
          const next = { ...current }; delete next[key]; return next;
        })}>Remove {asset.class === "hair" ? "hair" : "facial hair"}</button>)}
        {selected && appearance && <div className="flex gap-2">
          <button className={control} disabled={exporting.isPending} onClick={() => exporting.mutate()}>Export GLB + DDS package</button>
          {personKey && <button className={control} onClick={() => {
            try {
              localStorage.setItem(`head-library-selection:${personKey}`, JSON.stringify({ version: 1, asset: selected, appearance: appearance.id, companions }));
              setSelectionNotice("Library appearance saved on this computer. Game files have not been changed.");
            } catch { setSelectionNotice("Could not save the selection on this computer."); }
          }}>Remember appearance</button>}
        </div>}
        {exporting.data && <p className="text-xs">Exported to {exporting.data.destination}</p>}
        {selectionNotice && <p role="status" className="text-xs">{selectionNotice}</p>}
        <p className="text-xs text-slate-400">Toggle individual submeshes to compare hair-card states. Preview texture bindings follow map names; native rig and shader conversion is a separate destination-specific operation.</p>
      </div>
    </div>
    {[status.error, results.error, details.error, rendered.error, exporting.error].filter(Boolean).map((e,i) => <p key={i} className="text-sm text-rose-400">{String(e)}</p>)}
  </section>;
}
