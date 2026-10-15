// R1 F2 (20 Sep 2026): a profile cell's React key is its edit target (project, table, record and
// the field(s) it writes), so two teams' cells under the same label never share a mounted control
// — the ColorField/EditableField drafts and in-flight edits belong to one target. Pure key check.
import { describe, expect, it } from "vitest";
import type { FieldCellDto } from "../bindings/FieldCellDto";
import type { FieldViewDto } from "../bindings/FieldViewDto";
import { cellKey } from "./FieldProfile";

function fv(field: string): FieldViewDto {
  return { studio_table: "kOtt", studio_field: field } as unknown as FieldViewDto;
}

function cell(over: Partial<FieldCellDto>): FieldCellDto {
  return {
    label: "Primary",
    control: "field",
    field: null,
    record: 3,
    team_offset: null,
    none_label: null,
    derived_display: null,
    color_fields: [],
    unverified: null,
    ...over,
  };
}

describe("cellKey", () => {
  it("binds a cell to project, table, record and its fields, not only its label", () => {
    const colour = cell({ control: "color", color_fields: [fv("feprimarycolor")] });
    expect(cellKey(7, colour)).toBe("Primary|7/kOtt/3/feprimarycolor");
    expect(cellKey(7, cell({ ...colour, record: 8 }))).not.toBe(cellKey(7, colour));
    expect(cellKey(9, colour)).not.toBe(cellKey(7, colour));
    expect(cellKey(7, cell({ control: "color", color_fields: [fv("fesecondarycolor")] }))).not.toBe(cellKey(7, colour));
    const triple = cell({ control: "color", color_fields: [fv("c_r"), fv("c_g"), fv("c_b")] });
    expect(cellKey(7, triple)).toBe("Primary|7/kOtt/3/c_r+c_g+c_b");
    // Plain and derived cells: their single field or none.
    expect(cellKey(7, cell({ field: fv("name") }))).toBe("Primary|7/kOtt/3/name");
    expect(cellKey(7, cell({ control: "derived", label: "Overall" }))).toBe("Overall|7//3/");
    // Two labels on one record stay distinct.
    expect(cellKey(7, cell({ label: "Secondary", field: fv("name") }))).not.toBe(cellKey(7, cell({ field: fv("name") })));
  });
});
