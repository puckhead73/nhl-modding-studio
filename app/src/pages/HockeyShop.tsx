// Hockey Shop — per-equipment-slot browse + 3D preview (read-only first cut).
//
// The studio's port of the prior app's Equipment Studio. Pick a slot (skater or
// goalie gear), browse every equipment id the install ships for it, and preview the
// selected item in 3D with its texture files. Store (import/sync) and install-to-game
// arrive in later phases. This never edits the .db — gear is referenced by numeric
// ids on each player's xmja/fMVl record; to put an item on a player, set that field
// in the player's Equipment tab.

import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useAssetRootArgs } from "../lib/project";
import { EmptyState, Spinner } from "../components/primitives";
import { EquipmentItemViewer } from "../components/KitMeshViewer";
import { TextureThumbnail } from "../components/TextureReplace";
import type { EquipmentSlotDto } from "../bindings/EquipmentSlotDto";
import type { EquipmentSlotAssetDto } from "../bindings/EquipmentSlotAssetDto";

export function HockeyShopPage() {
  const { installDir, rootArgs } = useAssetRootArgs();

  const slotsQ = useQuery({
    queryKey: ["equipmentSlots"],
    queryFn: () => call("equipment_slots", {}),
    staleTime: Infinity,
  });
  const slots = slotsQ.data ?? [];

  const [slotKey, setSlotKey] = useState<string>("stick");
  const slot = slots.find((s) => s.key === slotKey) ?? slots[0];

  if (!installDir) {
    return (
      <EmptyState
        title="Hockey Shop needs an install tree"
        hint="Open a root-folder project (not a single database file) so the shop can scan the game's rendering folder for equipment."
      />
    );
  }
  if (slotsQ.isLoading) return <Spinner label="Loading equipment slots…" />;

  const skater = slots.filter((s) => s.category === "skater");
  const goalie = slots.filter((s) => s.category === "goalie");

  return (
    <div className="space-y-3">
      <header className="space-y-1">
        <h1 className="text-lg font-semibold text-slate-100">Hockey Shop</h1>
        <p className="text-xs text-slate-500">
          Browse the equipment the install ships for each slot and preview it in 3D. Editing a
          player's gear happens on their Equipment tab — the shop reads and (later) authors the render
          files, never the database.
        </p>
      </header>

      <div className="flex flex-wrap items-start gap-3">
        {/* Slot picker — skater + goalie category strips. */}
        <nav className="w-48 shrink-0 space-y-3">
          <SlotGroup title="Skater" slots={skater} active={slot?.key} onPick={setSlotKey} />
          <SlotGroup title="Goalie" slots={goalie} active={slot?.key} onPick={setSlotKey} />
        </nav>

        {slot ? <SlotShop key={slot.key} slot={slot} installDir={installDir} rootArgs={rootArgs} /> : null}
      </div>
    </div>
  );
}

function SlotGroup({
  title,
  slots,
  active,
  onPick,
}: {
  title: string;
  slots: EquipmentSlotDto[];
  active: string | undefined;
  onPick: (key: string) => void;
}) {
  return (
    <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-2">
      <div className="px-1 pb-1 text-[10px] font-semibold uppercase tracking-wide text-slate-500">
        {title}
      </div>
      <ul className="space-y-0.5">
        {slots.map((s) => {
          const on = s.key === active;
          return (
            <li key={s.key}>
              <button
                type="button"
                onClick={() => onPick(s.key)}
                title={s.field_ref}
                className={
                  "w-full rounded px-2 py-1 text-left text-sm transition-colors " +
                  (on
                    ? "bg-sky-900/50 font-medium text-sky-100"
                    : "text-slate-300 hover:bg-slate-800/60 hover:text-slate-100")
                }
              >
                {s.label}
              </button>
            </li>
          );
        })}
      </ul>
    </div>
  );
}

/** One slot's browse + preview: item list on the left, 3D preview + texture tiles
 *  on the right. Its own component so switching slots resets the selection. */
function SlotShop({
  slot,
  installDir,
  rootArgs,
}: {
  slot: EquipmentSlotDto;
  installDir: string;
  rootArgs: ReturnType<typeof useAssetRootArgs>["rootArgs"];
}) {
  const assetsQ = useQuery({
    queryKey: ["equipmentSlotAssets", rootArgs.install_dir, slot.key],
    queryFn: () => call("list_equipment_slot_assets", { roots: rootArgs, slot: slot.key }),
    staleTime: Infinity,
  });
  const assets = useMemo(() => assetsQ.data ?? [], [assetsQ.data]);

  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState<number | null>(null);

  // Keep a valid selection as the slot's item list loads / changes.
  useEffect(() => {
    if (assets.length === 0) setSelectedId(null);
    else if (selectedId == null || !assets.some((a) => a.id === selectedId)) {
      setSelectedId(assets[0].id);
    }
  }, [assets, selectedId]);

  const filtered = useMemo(() => {
    const q = query.trim();
    if (!q) return assets;
    return assets.filter((a) => String(a.id).includes(q));
  }, [assets, query]);

  const selected = assets.find((a) => a.id === selectedId) ?? null;

  return (
    <>
      {/* Item list. */}
      <section className="w-56 shrink-0 rounded-lg border border-slate-800 bg-slate-900/40 p-2">
        <div className="mb-2 flex items-baseline justify-between gap-2">
          <span className="text-xs font-semibold text-slate-200">{slot.label}</span>
          <span className="text-[10px] text-slate-500">
            {assets.length} {assets.length === 1 ? "item" : "items"}
          </span>
        </div>
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Filter by id…"
          className="mb-2 w-full rounded border border-slate-700 bg-slate-950 px-2 py-1 text-xs text-slate-100 focus:border-sky-600 focus:outline-none"
        />
        {assetsQ.isLoading ? (
          <Spinner label="Scanning…" />
        ) : assetsQ.isError ? (
          <p className="text-xs text-rose-400">{String(assetsQ.error)}</p>
        ) : filtered.length === 0 ? (
          <p className="px-1 py-4 text-center text-xs text-slate-500">
            {assets.length === 0
              ? `No ${slot.label.toLowerCase()} render files under this install.`
              : "No ids match that filter."}
          </p>
        ) : (
          <ul className="max-h-[62vh] space-y-0.5 overflow-y-auto pr-1">
            {filtered.map((a) => {
              const on = a.id === selectedId;
              return (
                <li key={a.id}>
                  <button
                    type="button"
                    onClick={() => setSelectedId(a.id)}
                    className={
                      "flex w-full items-baseline justify-between gap-2 rounded px-2 py-1 text-left text-xs transition-colors " +
                      (on
                        ? "bg-sky-900/50 text-sky-100"
                        : "text-slate-300 hover:bg-slate-800/60 hover:text-slate-100")
                    }
                  >
                    <span className="font-mono font-medium">#{a.id}</span>
                    <span className="text-[10px] text-slate-500">
                      {a.files.length} {a.files.length === 1 ? "file" : "files"}
                    </span>
                  </button>
                </li>
              );
            })}
          </ul>
        )}
      </section>

      {/* Preview + textures. */}
      <section className="min-w-[28rem] flex-1 basis-[36rem] space-y-3">
        <div className="h-[60vh] min-h-[30rem]">
          <EquipmentItemViewer roots={rootArgs} slot={slot.key} id={selectedId} />
        </div>
        {selected ? <ItemTextures installDir={installDir} asset={selected} /> : null}
      </section>
    </>
  );
}

/** The selected item's render files as decoded texture thumbnails (index 0 each). */
function ItemTextures({ installDir, asset }: { installDir: string; asset: EquipmentSlotAssetDto }) {
  return (
    <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-3">
      <h3 className="mb-2 text-xs font-semibold uppercase tracking-wide text-slate-400">
        Textures — id #{asset.id}
      </h3>
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
