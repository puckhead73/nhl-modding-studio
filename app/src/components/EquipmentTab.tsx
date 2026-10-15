import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { EmptyState, Spinner } from "./primitives";
import {
  compareEquipment,
  equipmentTypeLabel,
  eraLabel,
  readableForeground,
  rgbToHex,
  zoneName,
  EQUIPMENT_ZONE_FIELDS,
  ERA_LABELS,
} from "../lib/teamPresentation";
import type { TeamEquipmentDto } from "../bindings/TeamEquipmentDto";

const EQUIPMENT_TYPE_GLOVES = 5;
const EQUIPMENT_TYPE_HELMET = 8;

type Category = "other" | "gloves" | "helmet";
const CATEGORY_OPTIONS: ReadonlyArray<{ value: Category; label: string }> = [
  { value: "other", label: "Equipment" },
  { value: "gloves", label: "Gloves" },
  { value: "helmet", label: "Helmet" },
];

function matchesCategory(rec: TeamEquipmentDto, c: Category): boolean {
  switch (c) {
    case "gloves":
      return rec.equipment_type_raw === EQUIPMENT_TYPE_GLOVES;
    case "helmet":
      return rec.equipment_type_raw === EQUIPMENT_TYPE_HELMET;
    case "other":
      return (
        rec.equipment_type_raw !== EQUIPMENT_TYPE_GLOVES &&
        rec.equipment_type_raw !== EQUIPMENT_TYPE_HELMET
      );
  }
}

/// The Equipment tab — the team's stock-equipment rows (`hHbv`) in the reference layout: a
/// category-filtered record list (Gloves and Helmet split out — they're the anchored types) and
/// the detail pane with Equipment Info and this schema's color zones. Zones use split channels
/// or a packed RGB word; the UI shows RGB with a live swatch and edits each
/// channel through `set_field`. The reference app's 3D preview needs the bundled glove/helmet
/// meshes — that arrives with the Production Asset tools.
export function EquipmentTab({ projectId, teamId }: { projectId: number; teamId: number }) {
  const q = useQuery({
    queryKey: ["equipment", projectId, teamId],
    queryFn: () => call("list_team_equipment", { projectId, teamId }),
  });

  const [category, setCategory] = useState<Category>("other");
  const [selected, setSelected] = useState<number | null>(null);

  const sorted = useMemo(() => [...(q.data ?? [])].sort(compareEquipment), [q.data]);
  const visible = useMemo(
    () => sorted.filter((r) => matchesCategory(r, category)),
    [sorted, category],
  );

  useEffect(() => {
    if (visible.length === 0) {
      setSelected(null);
      return;
    }
    if (selected === null || !visible.some((r) => r.record === selected)) {
      setSelected(visible[0].record);
    }
  }, [visible, selected]);

  if (q.isLoading) return <Spinner label="Loading equipment…" />;
  if (q.isError) return <p className="text-rose-400">{String(q.error)}</p>;
  if (sorted.length === 0) {
    return <EmptyState title="No stock equipment is registered for this team" />;
  }

  const current = selected !== null ? visible.find((r) => r.record === selected) : undefined;

  return (
    <div className="flex min-h-[28rem] flex-wrap gap-3">
      <aside className="flex w-[20rem] shrink-0 flex-col rounded-lg border border-slate-800 bg-slate-900/40">
        <header className="border-b border-slate-800 p-3">
          <div className="flex items-center justify-between gap-2">
            <select
              value={category}
              onChange={(e) => setCategory(e.target.value as Category)}
              aria-label="Equipment category"
              className="min-w-0 flex-1 rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-xs font-semibold text-slate-100"
            >
              {CATEGORY_OPTIONS.map((o) => (
                <option key={o.value} value={o.value}>
                  {o.label}
                </option>
              ))}
            </select>
            <span className="text-[11px] text-slate-500">{visible.length}</span>
          </div>
        </header>
        <ul className="max-h-[60vh] overflow-y-auto">
          {visible.map((r) => {
            const isSel = r.record === selected;
            return (
              <li key={r.record}>
                <button
                  type="button"
                  onClick={() => setSelected(r.record)}
                  className={
                    "flex w-full flex-col items-start gap-0.5 border-b border-slate-900 px-3 py-1.5 text-left text-[11px] transition-colors " +
                    (isSel ? "bg-sky-900/40 text-sky-50" : "text-slate-200 hover:bg-slate-800/60")
                  }
                >
                  <div className="flex w-full items-baseline gap-2">
                    <span className="font-mono text-[10px] text-slate-500">#{r.record}</span>
                    <span className="font-semibold">{equipmentTypeLabel(r.equipment_type_raw)}</span>
                    <span className="ml-auto text-[10px] text-slate-500">v{r.variant_raw}</span>
                  </div>
                  <div className="flex w-full items-baseline gap-2 text-[10px] text-slate-400">
                    <span>Brand {r.brand_raw}</span>
                    <span>·</span>
                    <span>{eraLabel(r.era_raw)}</span>
                  </div>
                </button>
              </li>
            );
          })}
        </ul>
      </aside>
      <section className="min-w-0 flex-1">
        {current ? (
          <EquipmentDetail projectId={projectId} record={current} />
        ) : (
          <EmptyState title="Select an equipment record on the left" />
        )}
      </section>
    </div>
  );
}

export function EquipmentDetail({ projectId, record }: { projectId: number; record: TeamEquipmentDto }) {
  const qc = useQueryClient();
  const { update } = useProject();
  const mut = useMutation({
    mutationFn: (v: { field: string; value: number }) =>
      call("set_field", {
        projectId,
        table: "hHbv",
        record: record.record,
        field: v.field,
        intValue: v.value,
      }),
    onSuccess: async () => {
      await qc.invalidateQueries();
      update(await call("project_summary", { projectId }));
    },
  });

  return (
    <div className="space-y-4">
      <section className="rounded-lg border border-slate-800 bg-slate-900/40">
        <header className="border-b border-slate-800 px-4 py-2 text-[11px] font-semibold uppercase tracking-wider text-slate-300">
          Equipment Info
        </header>
        <dl className="grid grid-cols-2 gap-x-6 gap-y-3 px-4 py-3 sm:grid-cols-3">
          <Info label="Index" value={String(record.equipment_id)} hint="hHbv.vfEq — identity key, read-only." />
          <Info
            label="Type"
            value={`${equipmentTypeLabel(record.equipment_type_raw)} (${record.equipment_type_raw})`}
            hint="hHbv.jJpM — identity, read-only. 5 = Gloves, 8 = Helmet are the anchored types."
          />
          <NumberCell
            label="Brand"
            value={record.brand_raw}
            disabled={mut.isPending}
            onCommit={(v) => mut.mutate({ field: "TKWL", value: v })}
          />
          <div>
            <div className="text-[11px] font-semibold uppercase tracking-wide text-sky-300/80">
              Era
            </div>
            <select
              value={record.era_raw}
              disabled={mut.isPending}
              onChange={(e) => mut.mutate({ field: "VhbR", value: Number(e.target.value) })}
              className="mt-0.5 w-full rounded border border-slate-700 bg-slate-950 px-1.5 py-0.5 font-mono text-xs text-slate-100 focus:border-sky-600 focus:outline-none"
            >
              {Object.entries(ERA_LABELS).map(([raw, l]) => (
                <option key={raw} value={raw}>
                  {l}
                </option>
              ))}
              {!(record.era_raw in ERA_LABELS) && (
                <option value={record.era_raw}>{`raw ${record.era_raw}`}</option>
              )}
            </select>
          </div>
          <NumberCell
            label="Variant"
            value={record.variant_raw}
            disabled={mut.isPending}
            onCommit={(v) => mut.mutate({ field: "FccT", value: v })}
          />
        </dl>
      </section>

      <section className="rounded-lg border border-slate-800 bg-slate-900/40">
        <header className="flex items-baseline justify-between border-b border-slate-800 px-4 py-2">
          <h3 className="text-[11px] font-semibold uppercase tracking-wider text-slate-300">
            Zone Colors
          </h3>
          <span
            className="text-[10px] text-slate-500"
            title="Colors use this game's channel fields or packed RGB field. Field ids are on each input's tooltip."
          >
            Displayed RGB
          </span>
        </header>
        <div className="grid grid-cols-1 gap-3 px-3 py-3 md:grid-cols-2 xl:grid-cols-3">
          {record.zones.map((zone, idx) => {
            const name = zone.packed_field ? `Zone ${idx + 1}` : zoneName(record.equipment_type_raw, idx);
            const hex = rgbToHex(zone.red, zone.green, zone.blue);
            const fg = readableForeground(zone.red, zone.green, zone.blue);
            const [bField, gField, rField] = zone.packed_field
              ? [zone.packed_field, zone.packed_field, zone.packed_field]
              : EQUIPMENT_ZONE_FIELDS[idx];
            const commit = (field: string, channel: "red" | "green" | "blue", value: number) => {
              const rgb = { ...zone, [channel]: value };
              mut.mutate({ field, value: zone.packed_field ? (rgb.red << 16) | (rgb.green << 8) | rgb.blue : value });
            };
            return (
              <div
                key={idx}
                className="space-y-2 rounded-md border border-slate-800 bg-slate-950/40 p-2"
              >
                <div className="flex items-center justify-between gap-2">
                  <div>
                    <div className="text-[11px] font-semibold uppercase tracking-wide text-sky-300/80">
                      Zone {idx + 1}
                    </div>
                    <div className="text-[10px] text-slate-400">{name}</div>
                  </div>
                  <span
                    className="inline-flex h-7 w-16 items-center justify-center rounded border border-slate-700 font-mono text-[10px]"
                    style={{ backgroundColor: hex, color: fg }}
                    title={`${name} · ${hex}`}
                  >
                    {hex.slice(1)}
                  </span>
                </div>
                <div className="grid grid-cols-3 gap-2">
                  <ChannelCell
                    label="R"
                    field={rField}
                    value={zone.red}
                    disabled={mut.isPending}
                    onCommit={(v) => commit(rField, "red", v)}
                  />
                  <ChannelCell
                    label="G"
                    field={gField}
                    value={zone.green}
                    disabled={mut.isPending}
                    onCommit={(v) => commit(gField, "green", v)}
                  />
                  <ChannelCell
                    label="B"
                    field={bField}
                    value={zone.blue}
                    disabled={mut.isPending}
                    onCommit={(v) => commit(bField, "blue", v)}
                  />
                </div>
              </div>
            );
          })}
        </div>
      </section>

      {mut.error != null && <p className="text-xs text-rose-400">{String(mut.error)}</p>}
      <p className="text-[11px] text-slate-500">
        The reference app's 3D glove/helmet preview and the zone bulk actions (copy/swap between
        variants) arrive with the Production Asset tools phase.
      </p>
    </div>
  );
}

function Info({ label, value, hint }: { label: string; value: string; hint?: string }) {
  return (
    <div title={hint}>
      <div className="text-[11px] font-semibold uppercase tracking-wide text-sky-300/80">
        {label}
      </div>
      <div className="mt-0.5 font-mono text-xs text-slate-100">{value}</div>
    </div>
  );
}

function NumberCell({
  label,
  value,
  disabled,
  onCommit,
}: {
  label: string;
  value: number;
  disabled: boolean;
  onCommit: (v: number) => void;
}) {
  const [draft, setDraft] = useState(String(value));
  useEffect(() => setDraft(String(value)), [value]);
  const commit = () => {
    const n = Number(draft);
    if (Number.isFinite(n) && n !== value) onCommit(n);
    else setDraft(String(value));
  };
  return (
    <div>
      <div className="text-[11px] font-semibold uppercase tracking-wide text-sky-300/80">
        {label}
      </div>
      <input
        type="text"
        inputMode="numeric"
        value={draft}
        disabled={disabled}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Enter") commit();
          if (e.key === "Escape") setDraft(String(value));
        }}
        className="mt-0.5 w-full rounded border border-slate-700 bg-slate-950 px-1.5 py-0.5 font-mono text-xs text-slate-100 focus:border-sky-600 focus:outline-none"
      />
    </div>
  );
}

/** One 0..255 colour channel; the raw field id rides on the tooltip for power users. */
function ChannelCell({
  label,
  field,
  value,
  disabled,
  onCommit,
}: {
  label: string;
  field: string;
  value: number;
  disabled: boolean;
  onCommit: (v: number) => void;
}) {
  const [draft, setDraft] = useState(String(value));
  useEffect(() => setDraft(String(value)), [value]);
  const commit = () => {
    const n = Number(draft);
    if (Number.isFinite(n) && n >= 0 && n <= 255 && n !== value) onCommit(n);
    else setDraft(String(value));
  };
  return (
    <label className="flex items-center gap-1" title={`hHbv.${field}`}>
      <span className="w-3 text-[10px] font-semibold text-slate-400">{label}</span>
      <input
        type="text"
        inputMode="numeric"
        value={draft}
        disabled={disabled}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Enter") commit();
          if (e.key === "Escape") setDraft(String(value));
        }}
        className="w-full rounded border border-slate-700 bg-slate-950 px-1.5 py-0.5 font-mono text-xs text-slate-100 focus:border-sky-600 focus:outline-none"
      />
    </label>
  );
}
