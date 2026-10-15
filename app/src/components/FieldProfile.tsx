import { TeamArtIdSelect } from "./TeamArtIdSelect";
import type { ReactNode } from "react";
import { ArenaSelect, EditableField, ReadonlyField, TeamSelect } from "./EditableField";
import { ColorField } from "./ColorField";
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
  teamId,
  extras,
}: {
  projectId: number;
  profile: EntityFieldProfileDto;
  teams: TeamOption[];
  arena?: TeamViewDto["arena"];
  /** The team the profile belongs to — the arena cell's assignment preview is keyed by it. */
  teamId?: number;
  extras?: Record<string, ReactNode>;
}) {
  return (
    <div className="space-y-6">
      {profile.sections.map((s) => (
        <Section key={s.title} title={s.title}>
          {s.cells.map((c) => (
            <ProfileCellView key={cellKey(projectId, c)} projectId={projectId} cell={c} teams={teams} arena={arena} teamId={teamId} />
          ))}
          {extras?.[s.title]}
        </Section>
      ))}
    </div>
  );
}

/// R1 F2 (20 Sep 2026): a cell's React identity is its edit target — project, record and the
/// field(s) it writes — not its label, so a control never keeps a draft or an in-flight edit across
/// a change of team, project or field set that happens to render under the same label.
export function cellKey(projectId: number, c: FieldCellDto): string {
  const fields = c.control === "color" ? c.color_fields : c.field ? [c.field] : [];
  const table = fields[0]?.studio_table ?? "";
  return `${c.label}|${projectId}/${table}/${c.record}/${fields.map((f) => f.studio_field).join("+")}`;
}

/// One profile cell: dispatch on the control hint, then flag it when the loaded title's value
/// semantics aren't yet confirmed (raw value shown with an "unverified" note).
function ProfileCellView({
  projectId,
  cell,
  teams,
  arena,
  teamId,
}: {
  projectId: number;
  cell: FieldCellDto;
  teams: TeamOption[];
  arena?: TeamViewDto["arena"];
  teamId?: number;
}) {
  if (teamId != null && cell.field?.name === "artid") {
    return <TeamArtIdSelect key={`${projectId}/${teamId}`} projectId={projectId} teamId={teamId} />;
  }
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
      control =
        cell.field && teamId != null ? (
          <ArenaSelect projectId={projectId} teamId={teamId} record={cell.record} field={cell.field} arena={arena ?? null} />
        ) : cell.field ? (
          <ReadonlyField label={cell.label}>{arena ? `${arena.name}${arena.city ? ` — ${arena.city}` : ""}` : String(cell.field.display)}</ReadonlyField>
        ) : null;
      break;
    case "derived":
      control = <ReadonlyField label={cell.label}>{cell.derived_display ?? ""}</ReadonlyField>;
      break;
    case "color":
      control = (
        <ColorField projectId={projectId} record={cell.record} components={cell.color_fields} label={cell.label} />
      );
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
