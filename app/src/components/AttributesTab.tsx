import { useMutation, useQueryClient } from "@tanstack/react-query";
import { EditableField, ReadonlyField } from "./EditableField";
import { FieldRow } from "./FieldRow";
import { EmptyState } from "./primitives";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { ratingToneClass } from "../lib/ratingColor";
import type { PlayerViewDto } from "../bindings/PlayerViewDto";
import type { AttributeViewDto } from "../bindings/AttributeViewDto";
import type { TraitOptionDto } from "../bindings/TraitOptionDto";

/// The Attributes (Ratings) tab. Everything is resolved in studio-domain (`player.attributes`):
/// operator labels, curated order, the potential/accuracy/active vocabularies, and the trait
/// dictionary. Each cell edits through the ordinary `set_field` + friendly-input path (ratings type on
/// the 36..99 scale; the +36 offset is applied backend-side). Ratings are read-only when the player
/// has no companion record.
export function AttributesTab({ projectId, player }: { projectId: number; player: PlayerViewDto }) {
  const { project } = useProject();
  const caps = project!.capabilities;
  const a = player.attributes;
  const record = player.ratings_record;
  const editable = record !== null;

  if (!a.kind) {
    return (
      <EmptyState
        title="No ratings attached"
        hint="Neither a skater (dSvy) nor goalie (mHuy) companion row could be joined for this player — usually a broken cluster or a synthetic fixture."
      />
    );
  }

  const rec = record ?? player.bio_record;
  const kindLabel = a.kind === "goalie" ? "Goalie" : "Skater";
  // How many of this game's rating fields carry an operator label. Ratings resolve by metadata
  // name across every era (NHL 10–13's p_/g_ names included), so a shortfall here means the loaded
  // schema has ratings our vocabulary can't name yet — hedge visibly instead of looking
  // sparse-but-fine.
  const ratingsLabelled =
    a.kind === "goalie" ? caps.goalie_ratings_labelled : caps.skater_ratings_labelled;
  const ratingsTotal =
    a.kind === "goalie" ? caps.goalie_ratings_total : caps.skater_ratings_total;
  // The storage scale is per-era (PS3/X360: raw+36 → 36..99; PS4: stored 0..99 directly). The
  // backend stamps the offset on each cell; read it off the first rating for the caption.
  const plus36 = a.flat.some((c) => c.is_rating && c.field?.edit_offset === 36);

  const cell = (attr: AttributeViewDto | null | undefined) =>
    attr ? (
      <AttrCell key={attr.label} projectId={projectId} record={rec} attr={attr} editable={editable} />
    ) : null;

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-baseline gap-x-3 gap-y-1">
        <h3 className="text-sm font-semibold text-slate-100">{kindLabel} Ratings</h3>
        <span className="text-[11px] uppercase tracking-wider text-slate-500">
          from {player.ratings_table} · {plus36 ? "raw + 36 → 36..99" : "stored 0..99"}
        </span>
      </div>

      {!editable && (
        <p className="text-xs text-amber-300">
          No companion record for this player — ratings are read-only.
        </p>
      )}

      {ratingsLabelled < ratingsTotal && (
        <p className="text-xs text-amber-300">
          This game's ratings layout isn't fully mapped yet — {ratingsLabelled} of its {ratingsTotal}{" "}
          {kindLabel.toLowerCase()} ratings have names in our vocabulary. The rest appear under
          their raw field names below.
        </p>
      )}

      {/* Headline: Style + Potential Range/Accuracy (EA fields growthtier/growthletter — the in-game
          scouting potential). Overall is formula-derived (added later); tEok/c_potential is NHLViewNG's
          separate numeric potential, dormant in rosters, so it stays in the raw list only. */}
      <div className="rounded-lg border border-slate-800 bg-slate-900/40 p-3">
        <div className="grid grid-cols-2 gap-x-6 gap-y-3 sm:grid-cols-3">
          {cell(a.style)}
          {cell(a.potential_range)}
          {cell(a.potential_accuracy)}
        </div>
        {(a.potential_range || a.potential_accuracy) && (
          <p className="mt-2 text-[11px] leading-snug text-slate-500">
            Potential is EA's growth system (fields <span className="text-slate-400">growthtier</span> /
            <span className="text-slate-400"> growthletter</span>): Range = star ceiling (1★–5★),
            Accuracy = scouting certainty. Operator-anchored (Crosby = 5★) and direction-confirmed on
            anchor data — treat exact stars as provisional.
          </p>
        )}
      </div>

      {/* Curated flat attribute list. */}
      <section>
        <SectionHeading>Attributes</SectionHeading>
        <div className="grid grid-cols-2 items-start gap-x-8 gap-y-3 sm:grid-cols-3 lg:grid-cols-4">
          {a.flat.map((attr, i) => (
            <AttrCell key={`${attr.label}-${i}`} projectId={projectId} record={rec} attr={attr} editable={editable} />
          ))}
        </div>
      </section>

      <TraitsSection
        projectId={projectId}
        table={player.ratings_table}
        record={record}
        options={a.trait_options}
      />

      <details className="rounded-md border border-slate-800 bg-slate-900/40 px-3 py-2">
        <summary className="cursor-pointer text-xs font-semibold text-slate-300">
          All rating fields ({player.ratings.length}) — full editor
        </summary>
        <div className="mt-2 sm:grid sm:grid-cols-2 sm:gap-x-6">
          {player.ratings.map((f) => (
            <FieldRow
              key={f.studio_field}
              projectId={projectId}
              record={rec}
              field={{ ...f, editable: f.editable && editable }}
            />
          ))}
        </div>
      </details>
    </div>
  );
}

/** One attribute cell: label over an inline editor, tier-coloured when it's a 6-bit rating. An
 *  unmapped placeholder (no source field yet) renders read-only with an em dash. */
function AttrCell({
  projectId,
  record,
  attr,
  editable,
}: {
  projectId: number;
  record: number;
  attr: AttributeViewDto;
  editable: boolean;
}) {
  if (!attr.field) {
    return <ReadonlyField label={attr.label} />;
  }
  const tone = attr.is_rating ? ratingToneClass(Number(attr.field.display)) : undefined;
  const field = editable ? attr.field : { ...attr.field, editable: false };
  return (
    <EditableField projectId={projectId} record={record} field={field} label={attr.label} tone={tone} />
  );
}

function SectionHeading({ children }: { children: React.ReactNode }) {
  return (
    <h3 className="mb-3 border-b border-slate-800 pb-1 text-[11px] font-bold uppercase tracking-wider text-slate-400">
      {children}
    </h3>
  );
}

/** The trait menu: every dictionary trait as a checkbox — check to activate (write into a free slot),
 *  uncheck to deactivate (clear its slot). Read-only when the player has no ratings record. Weaker
 *  labels stay visibly hedged (amber/italic) and the provenance rides in the tooltip. */
function TraitsSection({
  projectId,
  table,
  record,
  options,
}: {
  projectId: number;
  table: string;
  record: number | null;
  options: TraitOptionDto[];
}) {
  const qc = useQueryClient();
  const { update } = useProject();
  const mut = useMutation({
    mutationFn: (v: { value: number; active: boolean }) =>
      call("set_player_trait", { projectId, table, record: record ?? 0, value: v.value, active: v.active }),
    onSuccess: async () => {
      await qc.invalidateQueries();
      update(await call("project_summary", { projectId }));
    },
  });

  const editable = record !== null;
  const activeCount = options.filter((o) => o.active).length;
  const pendingValue = mut.isPending ? mut.variables?.value : undefined;

  return (
    <section>
      <div className="mb-2 flex items-center gap-3">
        <span className="text-xs font-bold uppercase tracking-wider text-slate-300">Traits</span>
        <span className="h-px flex-1 bg-slate-800" />
        <span className="text-[10px] uppercase tracking-wider text-slate-500">
          {activeCount} active · {options.length} available on {table}
        </span>
      </div>
      {!editable && (
        <p className="mb-2 text-xs text-amber-300">No companion record — traits are read-only.</p>
      )}
      {mut.isError && <p className="mb-2 text-xs text-rose-400">{String(mut.error)}</p>}
      <div className="grid grid-cols-1 gap-x-6 gap-y-1 sm:grid-cols-2 lg:grid-cols-3">
        {options.map((o) => (
          <TraitToggle
            key={o.value}
            option={o}
            disabled={!editable || mut.isPending}
            busy={pendingValue === o.value}
            onToggle={(active) => mut.mutate({ value: o.value, active })}
          />
        ))}
      </div>
    </section>
  );
}

const CONFIDENCE_TEXT: Record<string, string> = {
  confirmed: "text-slate-200",
  strong: "text-emerald-200",
  weak: "text-amber-200 italic",
};

function TraitToggle({
  option,
  disabled,
  busy,
  onToggle,
}: {
  option: TraitOptionDto;
  disabled: boolean;
  busy: boolean;
  onToggle: (active: boolean) => void;
}) {
  const text = CONFIDENCE_TEXT[option.confidence] ?? "text-slate-300";
  const tooltip = [`trait ${option.value} · confidence: ${option.confidence}`, option.note]
    .filter(Boolean)
    .join("\n");
  return (
    <label
      className={`flex items-center gap-2 rounded px-1.5 py-1 text-sm ${
        disabled ? "opacity-60" : "cursor-pointer hover:bg-slate-800/40"
      }`}
      title={tooltip}
    >
      <input
        type="checkbox"
        className="h-4 w-4 shrink-0 rounded border-slate-600 bg-slate-950 accent-sky-600"
        checked={option.active}
        disabled={disabled}
        onChange={(e) => onToggle(e.target.checked)}
      />
      <span className={`truncate ${option.active ? text : "text-slate-400"}`}>{option.label}</span>
      {busy && <span className="ml-auto shrink-0 text-xs text-slate-500">…</span>}
    </label>
  );
}
