// Hockey Shop — the studio's port of the prior app's Equipment Studio. A category strip
// (Skater / Goalie) over a slot-tab strip, each slot a three-column view:
//
//   • Left   — the game's shipped equipment ids for the slot, searchable by id or model name,
//              each row resolving to its catalogued name (from defs/enums/*.toml) where one exists.
//   • Center — a 3D preview of the selected item + its decoded texture files.
//   • Right  — the Store: a disk-backed library of textures (sync from game / import PNG),
//              with entries already installed in the game greyed out, plus install-to-game.
//
// This never edits the .db — gear is referenced by numeric ids on each player's xmja/fMVl
// record; to put an item on a player, set that slot's field in the player's Equipment tab.

import { useEffect, useMemo, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { appDataDir, join } from "@tauri-apps/api/path";
import { call } from "../lib/client";
import { useAssetRootArgs } from "../lib/project";
import { EmptyState, Spinner } from "../components/primitives";
import { EquipmentItemViewer } from "../components/KitMeshViewer";
import { TextureThumbnail } from "../components/TextureReplace";
import type { EquipmentSlotDto } from "../bindings/EquipmentSlotDto";
import type { EquipmentSlotAssetDto } from "../bindings/EquipmentSlotAssetDto";
import type { HockeyShopItemDto } from "../bindings/HockeyShopItemDto";
import type { InstallStoreItemRequestDto } from "../bindings/InstallStoreItemRequestDto";

type Category = "skater" | "goalie";

const CATEGORIES: { key: Category; label: string }[] = [
  { key: "skater", label: "Skater Equipment" },
  { key: "goalie", label: "Goalie Equipment" },
];

export function HockeyShopPage() {
  const { installDir, rootArgs } = useAssetRootArgs();

  const slotsQ = useQuery({
    queryKey: ["equipmentSlots"],
    queryFn: () => call("equipment_slots", {}),
    staleTime: Infinity,
  });
  const slots = useMemo(() => slotsQ.data ?? [], [slotsQ.data]);

  const [slotKey, setSlotKey] = useState<string>("stick");
  const slot = slots.find((s) => s.key === slotKey) ?? slots[0];
  const activeCategory: Category = (slot?.category as Category) ?? "skater";

  // The store library lives in an app-managed folder (Tauri's app-data dir), kept separate from
  // the game install so operator imports survive re-extraction.
  const storeRootQ = useQuery({
    queryKey: ["hockeyShopStoreRoot"],
    queryFn: async () => join(await appDataDir(), "hockey_shop"),
    staleTime: Infinity,
  });
  const storeRoot = storeRootQ.data ?? null;

  if (!installDir) {
    return (
      <EmptyState
        title="Hockey Shop needs an install tree"
        hint="Open a root-folder project (not a single database file) so the shop can scan the game's rendering folder for equipment."
      />
    );
  }
  if (slotsQ.isLoading) return <Spinner label="Loading equipment slots…" />;

  const setCategory = (next: Category) => {
    if (next === activeCategory) return;
    const first = slots.find((s) => s.category === next);
    if (first) setSlotKey(first.key);
  };

  const categorySlots = slots.filter((s) => s.category === activeCategory);

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3">
      <header className="shrink-0 space-y-1">
        <h1 className="text-lg font-semibold text-slate-100">Hockey Shop</h1>
        <p className="text-xs text-slate-500">
          Browse the equipment the install ships for each slot and preview it in 3D. The store on the
          right is a library of textures you can build up and install into the game. Editing a
          player's gear happens on their Equipment tab — the shop reads and authors the render files,
          never the database.
        </p>
      </header>

      {/* Category strip. */}
      <div className="flex shrink-0 items-center gap-2">
        {CATEGORIES.map((c) => {
          const on = c.key === activeCategory;
          return (
            <button
              key={c.key}
              type="button"
              onClick={() => setCategory(c.key)}
              className={
                "rounded-md px-4 py-1.5 text-sm font-semibold transition-colors " +
                (on
                  ? "bg-sky-600 text-white"
                  : "bg-slate-800/60 text-slate-300 hover:bg-slate-800 hover:text-slate-100")
              }
            >
              {c.label}
            </button>
          );
        })}
      </div>

      {/* Slot tabs for the active category. */}
      <div className="flex shrink-0 items-center gap-1 overflow-x-auto border-b border-slate-800 px-1">
        {categorySlots.map((s) => {
          const on = s.key === slot?.key;
          return (
            <button
              key={s.key}
              type="button"
              onClick={() => setSlotKey(s.key)}
              title={s.field_ref}
              className={
                "shrink-0 rounded-t-md px-3 py-1.5 text-sm transition-colors " +
                (on
                  ? "border-b-2 border-sky-500 bg-slate-900/80 text-slate-100"
                  : "text-slate-400 hover:text-slate-200")
              }
            >
              {s.label}
            </button>
          );
        })}
      </div>

      {slot ? (
        <SlotShop
          key={slot.key}
          slot={slot}
          installDir={installDir}
          rootArgs={rootArgs}
          storeRoot={storeRoot}
        />
      ) : null}
    </div>
  );
}

/** One slot's three-column view: game list (left), 3D preview + textures (center), store (right). */
function SlotShop({
  slot,
  installDir,
  rootArgs,
  storeRoot,
}: {
  slot: EquipmentSlotDto;
  installDir: string;
  rootArgs: ReturnType<typeof useAssetRootArgs>["rootArgs"];
  storeRoot: string | null;
}) {
  const assetsQ = useQuery({
    queryKey: ["equipmentSlotAssets", rootArgs.install_dir, slot.key],
    queryFn: () => call("list_equipment_slot_assets", { roots: rootArgs, slot: slot.key }),
    staleTime: Infinity,
  });
  const assets = useMemo(() => assetsQ.data ?? [], [assetsQ.data]);

  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState<number | null>(null);
  // A selected store item drives the center viewport (its texture) instead of the game 3D
  // preview — whichever side was clicked last wins, mirroring the old app.
  const [storeSel, setStoreSel] = useState<HockeyShopItemDto | null>(null);

  // Keep a valid selection as the slot's item list loads / changes.
  useEffect(() => {
    if (assets.length === 0) setSelectedId(null);
    else if (selectedId == null || !assets.some((a) => a.id === selectedId)) {
      setSelectedId(assets[0].id);
    }
  }, [assets, selectedId]);

  // Sort by model name where known (uncatalogued ids fall to the bottom, id-ascending), matching
  // the old app's alphabetical-by-label ordering.
  const sorted = useMemo(() => {
    return [...assets].sort((a, b) => {
      if (a.label && b.label) return a.label.localeCompare(b.label);
      if (a.label) return -1;
      if (b.label) return 1;
      return a.id - b.id;
    });
  }, [assets]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return sorted;
    return sorted.filter(
      (a) => String(a.id).includes(q) || (a.label?.toLowerCase().includes(q) ?? false),
    );
  }, [sorted, query]);

  const selected = assets.find((a) => a.id === selectedId) ?? null;
  const gameIds = useMemo(() => new Set(assets.map((a) => a.id)), [assets]);

  return (
    <div className="flex min-h-0 flex-1 gap-3">
      {/* Left — game item list. */}
      <aside className="flex w-72 shrink-0 flex-col rounded-lg border border-slate-800 bg-slate-900/40">
        <header className="shrink-0 border-b border-slate-800 p-2.5">
          <div className="flex items-baseline justify-between gap-2">
            <span className="text-sm font-semibold text-slate-100">{slot.label}</span>
            <span className="text-[10px] text-slate-500">
              {filtered.length} / {assets.length}
            </span>
          </div>
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search by id or name…"
            className="mt-2 w-full rounded border border-slate-700 bg-slate-950 px-2 py-1 text-xs text-slate-100 placeholder-slate-600 focus:border-sky-600 focus:outline-none"
          />
        </header>
        <div className="min-h-0 flex-1 overflow-y-auto">
          {assetsQ.isLoading ? (
            <div className="p-3">
              <Spinner label="Scanning…" />
            </div>
          ) : assetsQ.isError ? (
            <p className="p-3 text-xs text-rose-400">{String(assetsQ.error)}</p>
          ) : filtered.length === 0 ? (
            <p className="px-3 py-6 text-center text-xs text-slate-500">
              {assets.length === 0
                ? `No ${slot.label.toLowerCase()} render files under this install.`
                : "No matches."}
            </p>
          ) : (
            <ul>
              {filtered.map((a) => {
                const on = a.id === selectedId;
                return (
                  <li key={a.id}>
                    <button
                      type="button"
                      onClick={() => {
                        setSelectedId(a.id);
                        setStoreSel(null);
                      }}
                      title={`${a.files.length} render file${a.files.length === 1 ? "" : "s"}`}
                      className={
                        "flex w-full items-baseline gap-2 border-b border-slate-900 px-3 py-1.5 text-left text-xs transition-colors " +
                        (on && !storeSel
                          ? "bg-sky-900/40 text-sky-50"
                          : "text-slate-200 hover:bg-slate-800/60")
                      }
                    >
                      <span className="w-8 shrink-0 font-mono text-[10px] text-slate-500">
                        {a.id}
                      </span>
                      <span
                        className={
                          "min-w-0 flex-1 truncate" + (a.label ? "" : " italic text-slate-400")
                        }
                      >
                        {a.label ?? `#${a.id}`}
                      </span>
                    </button>
                  </li>
                );
              })}
            </ul>
          )}
        </div>
      </aside>

      {/* Center — 3D preview + textures (game side), or the selected store item's texture. */}
      <section className="flex min-h-0 min-w-[26rem] flex-1 flex-col gap-3 overflow-y-auto pr-1">
        {storeSel && storeRoot ? (
          <StoreTexturePreview storeRoot={storeRoot} slot={slot.key} item={storeSel} />
        ) : (
          <>
            <div className="h-[58vh] min-h-[28rem] shrink-0">
              <EquipmentItemViewer roots={rootArgs} slot={slot.key} id={selectedId} />
            </div>
            {selected ? (
              <ItemTextures installDir={installDir} slot={slot} asset={selected} />
            ) : null}
          </>
        )}
      </section>

      {/* Right — the store. */}
      <aside className="w-72 shrink-0">
        <StorePanel
          slot={slot}
          rootArgs={rootArgs}
          storeRoot={storeRoot}
          gameIds={gameIds}
          selectedId={storeSel?.id ?? null}
          onSelect={setStoreSel}
        />
      </aside>
    </div>
  );
}

/** The slot's disk-backed store library: sync game items in, import a PNG, install, delete. Entries
 *  extracted from an id that's currently installed in the game grey out. Selecting a row previews
 *  its texture in the center viewport (no per-row thumbnails). */
function StorePanel({
  slot,
  rootArgs,
  storeRoot,
  gameIds,
  selectedId,
  onSelect,
}: {
  slot: EquipmentSlotDto;
  rootArgs: ReturnType<typeof useAssetRootArgs>["rootArgs"];
  storeRoot: string | null;
  gameIds: ReadonlySet<number>;
  selectedId: string | null;
  onSelect: (item: HockeyShopItemDto | null) => void;
}) {
  const qc = useQueryClient();
  const fileInput = useRef<HTMLInputElement>(null);
  const [search, setSearch] = useState("");
  // After a file pick we stage the image and open a modal to capture name + author (the old app's
  // flow) — imports are no longer silently credited to "Me".
  const [pendingImport, setPendingImport] = useState<{ name: string; imageBase64: string } | null>(
    null,
  );
  const invalidate = () =>
    qc.invalidateQueries({ queryKey: ["hockeyShopStore", storeRoot, slot.key] });

  const itemsQ = useQuery({
    queryKey: ["hockeyShopStore", storeRoot, slot.key],
    queryFn: () => call("hockey_shop_list", { storeRoot: storeRoot!, slot: slot.key }),
    enabled: !!storeRoot,
  });
  const items = itemsQ.data ?? [];

  const sync = useMutation({
    mutationFn: () =>
      call("hockey_shop_sync", { roots: rootArgs, storeRoot: storeRoot!, slot: slot.key }),
    onSuccess: invalidate,
  });
  const importItem = useMutation({
    mutationFn: (v: { name: string; author: string; imageBase64: string }) =>
      call("hockey_shop_import", { storeRoot: storeRoot!, slot: slot.key, ...v }),
    onSuccess: invalidate,
  });
  const remove = useMutation({
    mutationFn: (entryId: string) =>
      call("hockey_shop_delete", { storeRoot: storeRoot!, slot: slot.key, entryId }),
    onSuccess: (_r, entryId) => {
      invalidate();
      if (entryId === selectedId) onSelect(null); // its viewport preview is now gone
    },
  });

  const onPickFile = (file: File) => {
    const reader = new FileReader();
    reader.onload = () => {
      const url = String(reader.result); // data:image/png;base64,XXXX
      const b64 = url.slice(url.indexOf(",") + 1);
      const name = file.name.replace(/\.[^.]+$/, "").replace(/[_-]+/g, " ").trim();
      setPendingImport({ name, imageBase64: b64 });
    };
    reader.readAsDataURL(file);
  };

  const isInstalled = (it: HockeyShopItemDto) =>
    it.game_asset_id != null && gameIds.has(it.game_asset_id);

  const sorted = [...items].sort((a, b) => a.name.localeCompare(b.name));
  const q = search.trim().toLowerCase();
  const filtered = q
    ? sorted.filter(
        (e) => e.name.toLowerCase().includes(q) || e.author.toLowerCase().includes(q),
      )
    : sorted;

  const [installing, setInstalling] = useState<HockeyShopItemDto | null>(null);
  const busy = sync.isPending || importItem.isPending || remove.isPending;
  const err = sync.error ?? importItem.error ?? remove.error;

  return (
    <div className="flex h-full min-h-0 flex-col rounded-lg border border-slate-800 bg-slate-900/40">
      <header className="shrink-0 space-y-2 border-b border-slate-800 p-2.5">
        <div className="flex items-baseline justify-between">
          <h3 className="text-sm font-semibold text-slate-100">Store</h3>
          <span className="text-[10px] text-slate-500">
            {filtered.length} / {items.length}
          </span>
        </div>
        <div className="flex gap-1.5">
          <button
            type="button"
            onClick={() => sync.mutate()}
            disabled={!storeRoot || busy}
            title="Extract a texture for every game item in this slot into the store"
            className="flex-1 rounded border border-slate-700 px-2 py-1 text-xs text-slate-300 hover:border-sky-700 hover:text-sky-200 disabled:opacity-50"
          >
            {sync.isPending ? "Syncing…" : "Sync game"}
          </button>
          <button
            type="button"
            onClick={() => fileInput.current?.click()}
            disabled={!storeRoot || busy}
            title="Import a PNG texture into this slot's store"
            className="flex-1 rounded border border-emerald-700/60 bg-emerald-900/40 px-2 py-1 text-xs text-emerald-100 hover:bg-emerald-900/60 disabled:opacity-50"
          >
            Add PNG…
          </button>
          <input
            ref={fileInput}
            type="file"
            accept="image/png"
            className="hidden"
            onChange={(e) => {
              const f = e.target.files?.[0];
              if (f) onPickFile(f);
              e.target.value = "";
            }}
          />
        </div>
        <input
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="Search by name or author…"
          className="w-full rounded border border-slate-700 bg-slate-950 px-2 py-1 text-[11px] text-slate-100 placeholder-slate-600 focus:border-sky-600 focus:outline-none"
        />
        {err ? <p className="text-[10px] text-rose-400">{String(err)}</p> : null}
        {sync.data && sync.data.added > 0 ? (
          <p className="text-[10px] text-emerald-300">
            Extracted {sync.data.added} game item{sync.data.added === 1 ? "" : "s"} into the store.
          </p>
        ) : null}
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto p-2">
        {itemsQ.isLoading ? (
          <Spinner label="Loading store…" />
        ) : filtered.length === 0 ? (
          <p className="px-1 py-6 text-center text-xs text-slate-500">
            {items.length === 0
              ? "Empty. “Sync game” pulls this slot's shipped textures in, or add your own PNG."
              : "No matches."}
          </p>
        ) : (
          <ul className="space-y-1">
            {filtered.map((it) => (
              <StoreRow
                key={it.id}
                item={it}
                installed={isInstalled(it)}
                selected={it.id === selectedId}
                onSelect={() => onSelect(it)}
                onInstall={() => setInstalling(it)}
                onDelete={() => remove.mutate(it.id)}
                deleting={remove.isPending}
              />
            ))}
          </ul>
        )}
      </div>

      {pendingImport && storeRoot ? (
        <AddToStoreModal
          slotLabel={slot.label}
          pending={pendingImport}
          importing={importItem.isPending}
          error={importItem.error ? String(importItem.error) : null}
          onCancel={() => setPendingImport(null)}
          onConfirm={(name, author) =>
            importItem.mutate(
              { name, author, imageBase64: pendingImport.imageBase64 },
              {
                onSuccess: (entry) => {
                  setPendingImport(null);
                  onSelect(entry); // preview the freshly-imported texture
                },
              },
            )
          }
        />
      ) : null}

      {installing && storeRoot ? (
        <InstallModal
          slot={slot}
          rootArgs={rootArgs}
          storeRoot={storeRoot}
          item={installing}
          onClose={() => setInstalling(null)}
          onInstalled={() => {
            setInstalling(null);
            // The install wrote render files — refresh the game browser + preview.
            qc.invalidateQueries({ queryKey: ["equipmentSlotAssets"] });
            qc.invalidateQueries({ queryKey: ["equipmentItemPreview"] });
          }}
        />
      ) : null}
    </div>
  );
}

/** Author credit tone: EA Sports (extracted game textures) reads purple; community authors blue. */
function authorTone(author: string): string {
  return author.toUpperCase().includes("EA SPORTS") ? "text-purple-400" : "text-sky-400";
}

/** A single store entry: name + author on one line (no thumbnail — the texture previews in the
 *  center viewport when selected). Installed-in-game entries carry a badge; row highlights when
 *  selected. Install / remove sit on the right. */
function StoreRow({
  item,
  installed,
  selected,
  onSelect,
  onInstall,
  onDelete,
  deleting,
}: {
  item: HockeyShopItemDto;
  installed: boolean;
  selected: boolean;
  onSelect: () => void;
  onInstall: () => void;
  onDelete: () => void;
  deleting: boolean;
}) {
  return (
    <li
      className={
        "flex items-center gap-1 rounded border transition-colors " +
        (selected
          ? "border-sky-700 bg-sky-900/40"
          : installed
            ? "border-emerald-900/50 bg-emerald-950/30 hover:bg-emerald-900/30"
            : "border-slate-800 bg-slate-900/50 hover:bg-slate-800/60")
      }
    >
      <button
        type="button"
        onClick={onSelect}
        title={`By ${item.author}${installed ? " · installed in game" : ""}`}
        className="flex min-w-0 flex-1 items-baseline gap-2 px-2 py-1.5 text-left"
      >
        <span className="min-w-0 flex-1 truncate text-xs font-medium text-slate-100">
          {item.name}
        </span>
        {installed ? (
          <span className="shrink-0 rounded bg-emerald-900/60 px-1 text-[9px] uppercase tracking-wider text-emerald-200">
            installed
          </span>
        ) : null}
        <span className={"shrink-0 text-[10px] " + authorTone(item.author)}>{item.author}</span>
      </button>
      <button
        type="button"
        onClick={onInstall}
        title="Install this texture into the game at an equipment id"
        className="shrink-0 rounded border border-slate-700 px-1.5 py-0.5 text-[10px] text-slate-300 hover:border-sky-700 hover:text-sky-200"
      >
        Install…
      </button>
      <button
        type="button"
        onClick={onDelete}
        disabled={deleting}
        title="Remove from store"
        className="mr-1 shrink-0 rounded px-1 text-slate-500 hover:bg-slate-800 hover:text-rose-400 disabled:opacity-50"
      >
        ✕
      </button>
    </li>
  );
}

/** The selected store item's texture, shown in the center viewport (fetched on demand — not per
 *  list row). Mirrors the old app's "texture previews in the viewport on selection". */
function StoreTexturePreview({
  storeRoot,
  slot,
  item,
}: {
  storeRoot: string;
  slot: string;
  item: HockeyShopItemDto;
}) {
  const tex = useQuery({
    queryKey: ["hockeyShopTex", storeRoot, slot, item.id],
    queryFn: () => call("hockey_shop_load_texture", { storeRoot, slot, entryId: item.id }),
    staleTime: Infinity,
    retry: false,
  });
  return (
    <div className="flex min-h-0 flex-1 flex-col rounded-lg border border-slate-800 bg-slate-900/40">
      <header className="flex shrink-0 items-baseline justify-between gap-2 border-b border-slate-800 px-3 py-2">
        <div className="min-w-0">
          <p className="truncate text-sm font-semibold text-slate-100" title={item.name}>
            {item.name}
          </p>
          <p className="truncate text-[10px] text-slate-500">
            by <span className={authorTone(item.author)}>{item.author}</span> · {item.width}×
            {item.height}
          </p>
        </div>
      </header>
      <div className="flex min-h-0 flex-1 items-center justify-center overflow-hidden p-3">
        {tex.isLoading ? (
          <Spinner label="Loading texture…" />
        ) : tex.error ? (
          <p className="text-xs text-rose-400">{String(tex.error)}</p>
        ) : tex.data ? (
          <img
            src={tex.data}
            alt={item.name}
            className="max-h-full max-w-full rounded object-contain"
            style={{ imageRendering: "pixelated" }}
          />
        ) : null}
      </div>
    </div>
  );
}

/** Import modal: capture a display name (prefilled from the file) + author before ingesting a PNG
 *  into the store. */
function AddToStoreModal({
  slotLabel,
  pending,
  importing,
  error,
  onCancel,
  onConfirm,
}: {
  slotLabel: string;
  pending: { name: string; imageBase64: string };
  /** Whether the import mutation is running. */
  importing: boolean;
  error: string | null;
  onCancel: () => void;
  onConfirm: (name: string, author: string) => void;
}) {
  const [name, setName] = useState(pending.name);
  const [author, setAuthor] = useState("");
  const canConfirm = name.trim().length > 0 && author.trim().length > 0 && !importing;
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-6" onClick={onCancel}>
      <div
        className="w-full max-w-md rounded-lg border border-slate-700 bg-slate-900 shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <header className="border-b border-slate-800 px-4 py-2.5">
          <p className="text-sm font-medium text-slate-100">Add {slotLabel} texture to store</p>
          <p className="mt-0.5 text-[11px] text-slate-500">
            It's saved to the store only — installing it into the game is a separate step.
          </p>
        </header>
        <div className="space-y-3 p-4 text-xs">
          <label className="block space-y-1">
            <span className="text-[11px] text-slate-400">Name</span>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              autoFocus
              placeholder="e.g. CCM Ribcore 40K (custom)"
              className="w-full rounded border border-slate-700 bg-slate-950 px-2 py-1 text-slate-100 focus:border-sky-600 focus:outline-none"
            />
          </label>
          <label className="block space-y-1">
            <span className="text-[11px] text-slate-400">Author</span>
            <input
              type="text"
              value={author}
              onChange={(e) => setAuthor(e.target.value)}
              placeholder="Who created this texture?"
              className="w-full rounded border border-slate-700 bg-slate-950 px-2 py-1 text-slate-100 focus:border-sky-600 focus:outline-none"
            />
          </label>
          {error ? <p className="text-[11px] text-rose-400">{error}</p> : null}
        </div>
        <footer className="flex justify-end gap-2 border-t border-slate-800 px-4 py-2.5">
          <button
            type="button"
            onClick={onCancel}
            disabled={importing}
            className="rounded border border-slate-700 px-3 py-1 text-xs text-slate-300 hover:bg-slate-800 disabled:opacity-50"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={() => onConfirm(name.trim(), author.trim())}
            disabled={!canConfirm}
            className="rounded border border-emerald-700/60 bg-emerald-900/40 px-3 py-1 text-xs font-medium text-emerald-100 hover:bg-emerald-900/60 disabled:opacity-50"
          >
            {importing ? "Adding…" : "Add to store"}
          </button>
        </footer>
      </div>
    </div>
  );
}

/** Install a store item into the game at a chosen equipment id: pick the id, see a live plan
 *  (overwrite vs create, donor, warnings, blockers), then apply. The apply writes render files
 *  into the install (a .bak first) — it never edits the .db. */
function InstallModal({
  slot,
  rootArgs,
  storeRoot,
  item,
  onClose,
  onInstalled,
}: {
  slot: EquipmentSlotDto;
  rootArgs: ReturnType<typeof useAssetRootArgs>["rootArgs"];
  storeRoot: string;
  item: HockeyShopItemDto;
  onClose: () => void;
  onInstalled: () => void;
}) {
  const [targetId, setTargetId] = useState<number>(item.game_asset_id ?? 0);
  const request: InstallStoreItemRequestDto = {
    slot: slot.key,
    entry_id: item.id,
    target_id: targetId,
    write_backup: true,
  };

  const plan = useQuery({
    queryKey: ["hockeyShopInstallPlan", storeRoot, slot.key, item.id, targetId],
    queryFn: () => call("hockey_shop_install_plan", { roots: rootArgs, storeRoot, request }),
    enabled: Number.isFinite(targetId),
    retry: false,
  });
  const apply = useMutation({
    mutationFn: () => call("hockey_shop_install_apply", { roots: rootArgs, storeRoot, request }),
    onSuccess: onInstalled,
  });

  const p = plan.data;
  const blocked = (p?.blocking_errors.length ?? 1) > 0;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-6" onClick={onClose}>
      <div
        className="w-full max-w-lg rounded-lg border border-slate-700 bg-slate-900 shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <header className="flex items-center justify-between border-b border-slate-800 px-4 py-2">
          <p className="text-sm font-medium text-slate-100">
            Install “{item.name}” — {slot.label}
          </p>
          <button
            type="button"
            onClick={onClose}
            className="rounded border border-slate-700 px-2 py-0.5 text-xs text-slate-200 hover:bg-slate-800"
          >
            Close
          </button>
        </header>
        <div className="space-y-3 p-4">
          <label className="flex items-center gap-2 text-xs text-slate-300">
            <span className="shrink-0">Install at equipment id</span>
            <input
              type="number"
              min={0}
              max={slot.max_id}
              value={targetId}
              onChange={(e) => setTargetId(Number(e.target.value))}
              className="w-24 rounded border border-slate-700 bg-slate-950 px-2 py-1 font-mono text-slate-100 focus:border-sky-600 focus:outline-none"
            />
            <span className="text-slate-500">0–{slot.max_id}</span>
          </label>

          {plan.isLoading ? (
            <Spinner label="Planning…" />
          ) : plan.error ? (
            <p className="text-xs text-rose-400">{String(plan.error)}</p>
          ) : p ? (
            <div className="space-y-2 rounded-md border border-slate-800 bg-slate-950/50 p-3 text-xs">
              <p className="text-slate-300">
                Mode:{" "}
                <b className={p.mode === "overwrite" ? "text-amber-300" : "text-sky-300"}>
                  {p.mode}
                </b>
                {p.mode === "overwrite" && p.target_existing_label ? (
                  <span className="text-slate-500"> — replaces “{p.target_existing_label}”</span>
                ) : null}
                {p.mode === "create" && p.donor_id != null ? (
                  <span className="text-slate-500">
                    {" "}
                    — clones donor id {p.donor_id} ({p.cloned_files.length} files)
                  </span>
                ) : null}
              </p>
              {p.target_color_file ? (
                <p className="truncate text-slate-500" title={p.target_color_file}>
                  → {p.target_color_file} · tex {p.target_texture_index} · {p.target_width}×
                  {p.target_height}
                </p>
              ) : null}
              {p.warnings.map((w, i) => (
                <p key={i} className="text-amber-400/80">
                  ⚠ {w}
                </p>
              ))}
              {p.blocking_errors.map((e, i) => (
                <p key={i} className="text-rose-400">
                  ✕ {e}
                </p>
              ))}
            </div>
          ) : null}

          {apply.error ? <p className="text-xs text-rose-400">{String(apply.error)}</p> : null}

          <div className="flex justify-end gap-2">
            <button
              type="button"
              onClick={onClose}
              className="rounded border border-slate-700 px-3 py-1 text-xs text-slate-300 hover:bg-slate-800"
            >
              Cancel
            </button>
            <button
              type="button"
              onClick={() => apply.mutate()}
              disabled={blocked || apply.isPending || plan.isLoading}
              className="rounded border border-sky-700 bg-sky-900/50 px-3 py-1 text-xs font-medium text-sky-100 hover:bg-sky-900/80 disabled:opacity-50"
            >
              {apply.isPending ? "Installing…" : "Install to game"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

/** The selected item's render files as decoded texture thumbnails (index 0 each). */
function ItemTextures({
  installDir,
  slot,
  asset,
}: {
  installDir: string;
  slot: EquipmentSlotDto;
  asset: EquipmentSlotAssetDto;
}) {
  return (
    <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-3">
      <div className="mb-2 flex items-baseline justify-between gap-2">
        <h3 className="text-xs font-semibold uppercase tracking-wide text-slate-400">
          Textures — {asset.label ?? `${slot.label} #${asset.id}`}
        </h3>
        <span className="font-mono text-[10px] text-slate-500">#{asset.id}</span>
      </div>
      <div className="grid grid-cols-4 gap-3 sm:grid-cols-6">
        {asset.files.map((f) => {
          const name = f.relative_path.split("/").pop() ?? f.relative_path;
          return (
            <div key={f.relative_path} className="flex flex-col gap-1">
              <TextureThumbnail
                installDir={installDir}
                relPath={f.relative_path}
                textureIndex={0}
                maxEdge={256}
                className="aspect-square h-auto w-full rounded border border-slate-800"
              />
              <p className="truncate text-[10px] text-slate-400" title={f.relative_path}>
                {name}
              </p>
            </div>
          );
        })}
      </div>
    </div>
  );
}
