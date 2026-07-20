import type { ReactNode } from "react";
import { ArenaSelect, EditableField, ReadonlyField, TeamSelect } from "./EditableField";
import type { EntityFieldProfileDto } from "../bindings/EntityFieldProfileDto";
import type { FieldCellDto } from "../bindings/FieldCellDto";
import type { TeamViewDto } from "../bindings/TeamViewDto";

type TeamOption = { team_id: number; name: string; abbrev: string };

/** A titled block whose cells flow in a 4-column grid (the reference Player/Team Data layout). */
export function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section>
      <h3 className="mb-3 border-b border-slate-800 pb-1 text-[11px] font-bold uppercase tracking-wider text-slate-400">
        {title}
      </h3>
      <div className="grid grid-cols-2 items-start gap-x-8 gap-y-3 sm:grid-cols-3 lg:grid-cols-4">
        {children}
      </div>
    </section>
  );
}

/// Render a whole server-driven field profile: each section becomes a titled grid, each cell an
/// inline-editable control chosen from the domain's control hint. The domain only emits cells the
/// loaded schema actually carries, so there are no "not in this game" placeholders — a title shows
/// exactly its own fields. `extras` lets a page inject an extra node into a named section's grid
/// (e.g. the player's "Additional Teams" chips inside Roster Data).
export function ProfileSections({
  projectId,
  profile,
  teams,
  arena,
  extras,
}: {
  projectId: number;
  profile: EntityFieldProfileDto;
  teams: TeamOption[];
  arena?: TeamViewDto["arena"];
  extras?: Record<string, ReactNode>;
}) {
  return (
    <div className="space-y-6">
      {profile.sections.map((s) => (
        <Section key={s.title} title={s.title}>
          {s.cells.map((c) => (
            <ProfileCellView key={c.label} projectId={projectId} cell={c} teams={teams} arena={arena} />
          ))}
          {extras?.[s.title]}
        </Section>
      ))}
    </div>
  );
}

/// One profile cell: dispatch on the control hint, then flag it when the loaded title's value
/// semantics aren't yet confirmed (raw value shown with an "unverified" note).
function ProfileCellView({
  projectId,
  cell,
  teams,
  arena,
}: {
  projectId: number;
  cell: FieldCellDto;
  teams: TeamOption[];
  arena?: TeamViewDto["arena"];
}) {
  let control: ReactNode;
  switch (cell.control) {
    case "team":
      control = (
        <TeamSelect
          projectId={projectId}
          record={cell.record}
          field={cell.field ?? undefined}
          teams={teams}
          label={cell.label}
          noneLabel={cell.none_label ?? undefined}
          offset={cell.team_offset ?? 1}
        />
      );
      break;
    case "arena":
      control = cell.field ? (
        <ArenaSelect projectId={projectId} record={cell.record} field={cell.field} arena={arena ?? null} />
      ) : null;
      break;
    case "derived":
      control = <ReadonlyField label={cell.label}>{cell.derived_display ?? ""}</ReadonlyField>;
      break;
    default: // "field"
      control = cell.field ? (
        <EditableField projectId={projectId} record={cell.record} field={cell.field} label={cell.label} />
      ) : null;
  }

  if (!cell.unverified) return <>{control}</>;
  return (
    <div className="min-w-0">
      {control}
      <div className="mt-0.5 text-[10px] text-amber-500/80" title={cell.unverified}>
        ⚠ unverified
      </div>
    </div>
  );
}
