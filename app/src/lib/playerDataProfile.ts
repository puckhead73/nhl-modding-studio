import type { PlayerViewDto } from "../bindings/PlayerViewDto";
import type { EntityFieldProfileDto } from "../bindings/EntityFieldProfileDto";

/** Player Data and Attributes edit the same ratings-companion archetype field. */
export function playerDataProfile(player: PlayerViewDto): EntityFieldProfileDto {
  const style = player.attributes.style?.field;
  if (!style || player.ratings_record == null) return player.profile;
  return {
    ...player.profile,
    sections: player.profile.sections.map(section => ({
      ...section,
      cells: section.cells.map(cell => cell.label === "Archetype" ? {
        ...cell,
        control: "field" as const,
        field: style,
        record: player.ratings_record!,
        derived_display: null,
      } : cell),
    })),
  };
}
