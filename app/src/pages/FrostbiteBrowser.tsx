import { useState } from "react";
import { Link } from "react-router-dom";
import { useMutation, useQuery } from "@tanstack/react-query";
import { open, save } from "@tauri-apps/plugin-dialog";
import { inspectFrostbite, listFrostbite, frostbiteAssets, exportFrostbite, type FrostbiteInstall } from "../lib/frostbite";
import { listFrostbiteChunks, exportFrostbiteChunk } from "../lib/frostbite";

const control = "rounded border border-slate-700 bg-slate-900 px-3 py-2 disabled:opacity-40";
export function FrostbiteBrowser() {
  const [path, setPath] = useState("");
  const [install, setInstall] = useState<FrostbiteInstall | null>(null);
  const [layer, setLayer] = useState("Data");
  const [superbundle, setSuperbundle] = useState("");
  const [search, setSearch] = useState("");
  const [draft, setDraft] = useState("");
  const [offset, setOffset] = useState(0);
  const [bundle, setBundle] = useState<number | null>(null);
  const [showChunks, setShowChunks] = useState(false);
  const selection = { root: install?.root ?? "", layer, superbundle };
  function resetSelection() { setOffset(0); setBundle(null); }
  const scan = useMutation({ mutationFn: inspectFrostbite, onSuccess: result => {
    setInstall(result); setLayer("Data"); setSuperbundle(result.layers[0]?.superbundles[0] ?? ""); resetSelection();
  } });
  const listing = useQuery({ queryKey: ["frostbite-bundles", selection, search, offset], queryFn: () => listFrostbite(selection, search, offset), enabled: !!install && !!superbundle && !showChunks });
  const chunks = useQuery({ queryKey: ["frostbite-chunks", selection, search, offset], queryFn: () => listFrostbiteChunks(selection, search, offset), enabled: !!install && !!superbundle && showChunks });
  const assets = useQuery({ queryKey: ["frostbite-assets", selection, bundle], queryFn: () => frostbiteAssets(selection, bundle!), enabled: !!install && bundle !== null });
  const exporting = useMutation({ mutationFn: async ({ asset, decoded }: { asset: number; decoded: boolean }) => {
    if (!showChunks && bundle === null) return null;
    const destination = await save({ title: decoded ? "Export decoded asset" : "Export raw CAS span", defaultPath: `asset-${asset}.${decoded ? "bin" : "casblocks"}` });
    if (!destination) return null;
    return showChunks ? exportFrostbiteChunk(selection, asset, decoded, destination) : exportFrostbite(selection, bundle!, asset, decoded, destination);
  } });
  const error = scan.error ?? listing.error ?? chunks.error ?? assets.error ?? exporting.error;
  return <main className="min-h-screen bg-slate-950 p-6 text-slate-100">
    <div className="mx-auto max-w-7xl space-y-4">
      <Link to="/studio" className="text-sky-400">← Project picker</Link>
      <h1 className="text-2xl font-semibold">Frostbite assets · NHL 22–24 PS4</h1>
      <p className="text-sm text-slate-400">Open an extracted game folder containing Data/layout.toc. Browse assets without a roster database. This workspace is read-only; PS5 support is still in development.</p>
      <form className="flex gap-2" onSubmit={e => { e.preventDefault(); setInstall(null); scan.mutate(path); }}>
        <input aria-label="Frostbite game folder" className={`${control} flex-1`} value={path} onChange={e => setPath(e.target.value)} placeholder="Extracted PS4 game folder" />
        <button type="button" className={control} onClick={async () => { const p = await open({ directory: true, multiple: false }); if (typeof p === "string") setPath(p); }}>Browse</button>
        <button className={control} disabled={!path || scan.isPending}>{scan.isPending ? "Inspecting…" : "Open assets"}</button>
      </form>
      {error && <p role="alert" className="rounded border border-red-800 p-3 text-red-300">{String(error)}</p>}
      {install && <>
        {install.warnings.map(w => <p key={w} className="text-sm text-amber-300">{w}</p>)}
        <div className="flex flex-wrap items-center gap-3">
          <select aria-label="Content layer" className={control} value={layer} onChange={e => { setLayer(e.target.value); setSuperbundle(install.layers.find(l => l.name === e.target.value)?.superbundles[0] ?? ""); resetSelection(); }}>
            {install.layers.map(l => <option key={l.name}>{l.name}</option>)}
          </select>
          <select aria-label="Superbundle" className={control} value={superbundle} onChange={e => { setSuperbundle(e.target.value); resetSelection(); }}>
            {install.layers.find(l => l.name === layer)?.superbundles.map(s => <option key={s}>{s}</option>)}
          </select>
          <span className="text-sm text-slate-400">Initfs: {install.layers.find(l => l.name === layer)?.initfs}</span>
        </div>
        <form className="flex gap-2" onSubmit={e => { e.preventDefault(); setSearch(draft); resetSelection(); }}>
          <select aria-label="Entry kind" className={control} value={showChunks ? "chunks" : "bundles"} onChange={e => { setShowChunks(e.target.value === "chunks"); resetSelection(); setDraft(""); setSearch(""); }}><option value="bundles">Bundles</option><option value="chunks">TOC chunks</option></select>
          <input aria-label="Search container entries" className={`${control} flex-1`} value={draft} onChange={e => setDraft(e.target.value)} placeholder={showChunks ? "Search chunk GUIDs" : "Search decoded bundle names"} />
          <button className={control}>Search</button>
        </form>
        {(listing.isFetching || chunks.isFetching) && <p role="status">Reading container index…</p>}
        {!showChunks && listing.data && <>
          <p className="text-sm text-slate-400">{listing.data.matched.toLocaleString()} matching bundles / {listing.data.total.toLocaleString()} total · {listing.data.toc_chunks.toLocaleString()} TOC chunks</p>
          <div className="grid gap-4 lg:grid-cols-2">
            <div className="max-h-[65vh] overflow-auto rounded border border-slate-800">
              {listing.data.items.map(b => <button key={b.ordinal} onClick={() => setBundle(b.ordinal)} className={`block w-full break-all border-b border-slate-800 p-2 text-left text-xs hover:bg-slate-800 ${bundle === b.ordinal ? "bg-slate-800" : ""}`}>{b.name}<span className="ml-2 text-slate-500">{b.spans - 1} assets</span></button>)}
              {listing.data.matched === 0 && <p className="p-4">No matching bundles.</p>}
            </div>
            <div className="max-h-[65vh] space-y-2 overflow-auto">
              {assets.isFetching && <p role="status">Reading bundle manifest…</p>}
              {bundle === null && <p className="text-slate-400">Select a bundle to inspect its assets.</p>}
              {assets.data?.assets.map((a, i) => <article key={i} className="rounded border border-slate-800 p-3 text-xs">
                <p className="break-all font-medium">{a.name}</p>
                <p className="my-2 text-slate-400">{a.kind} · {a.size.toLocaleString()} logical bytes{a.resource_type !== null && ` · type 0x${a.resource_type.toString(16)}`}</p>
                <p className="break-all text-slate-500">SHA-1 {a.sha1}</p>
                <p className="text-slate-500">CAS 0x{assets.data!.spans[i].cas.toString(16)} · offset {assets.data!.spans[i].offset} · {assets.data!.spans[i].size} stored bytes</p>
                <div className="mt-2 flex gap-2">
                  <button className={control} disabled={exporting.isPending} onClick={() => exporting.mutate({ asset: i, decoded: false })}>Export raw</button>
                  <button className={control} disabled={exporting.isPending} onClick={() => exporting.mutate({ asset: i, decoded: true })}>Decode stored blocks</button>
                </div>
              </article>)}
              {assets.data?.assets.length === 0 && <p>Empty bundle.</p>}
            </div>
          </div>
          <div className="flex gap-3">
            <button className={control} disabled={offset === 0} onClick={() => { setOffset(Math.max(0, offset - 100)); setBundle(null); }}>Previous</button>
            <button className={control} disabled={offset + 100 >= listing.data.matched} onClick={() => { setOffset(offset + 100); setBundle(null); }}>Next</button>
          </div>
        </>}
        {showChunks && chunks.data && <>
          <p>{chunks.data.matched.toLocaleString()} matching TOC chunks</p>
          <div className="max-h-[65vh] overflow-auto space-y-2">{chunks.data.items.map(c => <article key={c.ordinal} className="rounded border border-slate-800 p-3 text-xs">
            <p>{c.guid}</p><p className="text-slate-400">CAS 0x{c.span.cas.toString(16)} · offset {c.span.offset} · {c.span.size.toLocaleString()} stored bytes</p>
            <div className="mt-2 flex gap-2"><button className={control} disabled={exporting.isPending} onClick={() => exporting.mutate({asset:c.ordinal,decoded:false})}>Export raw</button><button className={control} disabled={exporting.isPending} onClick={() => exporting.mutate({asset:c.ordinal,decoded:true})}>Decode stored blocks</button></div>
          </article>)}</div>
          <div className="flex gap-3"><button className={control} disabled={offset === 0} onClick={() => setOffset(Math.max(0,offset-100))}>Previous</button><button className={control} disabled={offset+100 >= chunks.data.matched} onClick={() => setOffset(offset+100)}>Next</button></div>
        </>}
        {exporting.data != null && <p role="status">Exported {exporting.data.toLocaleString()} bytes.</p>}
        <p className="text-xs text-slate-500">Most PS4 payloads use codec 0x11. Raw export preserves their CAS framing; decoded export currently accepts stored blocks only.</p>
      </>}
    </div>
  </main>;
}
