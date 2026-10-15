// @vitest-environment jsdom
//
// Research R122 (27 Sep 2026): the Lines roster rail's style dropdown takes its options from the
// domain's per-title vocabulary for the roster cell — the Legacy ten styles from NHL 11 on, the NHL 07
// seven styles on NHL 08 / 09, none on NHL 07 / 10 — and falls back to the field's declared range as
// plain numbers; goalie rows always show numbers (their roster-cell meaning is unresolved).
import { describe, expect, it } from "vitest";
import { archetypeChoices } from "./LinesTab";
import type { FieldViewDto } from "../bindings/FieldViewDto";

const cell = (over: Partial<FieldViewDto>): FieldViewDto =>
  ({
    studio_table: "eGlu",
    studio_field: "QgFs",
    name: "Player Style",
    kind: "Int",
    display: "4",
    decoded: 4,
    enum_label: null,
    enum_options: [],
    edit_offset: 0,
    edit_height: false,
    edit_scale: null,
    raw_int: 4,
    text: null,
    range_low: 0,
    range_high: 15,
    editable: true,
    is_string: false,
    text_max_bytes: null,
    vocabulary_note: null,
    ...over,
  }) as unknown as FieldViewDto;

describe("archetypeChoices (roster style rail)", () => {
  it("uses the domain's labels when the title has them", () => {
    const opts = archetypeChoices(cell({ enum_options: [{ raw: 3, label: "Grinder" }, { raw: 1, label: "Defensive" }], range_high: 7 }), false);
    expect(opts).toEqual([
      { raw: 1, label: "Defensive" },
      { raw: 3, label: "Grinder" },
    ]);
  });

  it("offers the field's own range as numbers when the title has no vocabulary (NHL 07 / 10)", () => {
    const opts = archetypeChoices(cell({ enum_options: [], range_low: 0, range_high: 15 }), false);
    expect(opts.map((o) => o.label)).toEqual(Array.from({ length: 16 }, (_, i) => String(i)));
    expect(opts.some((o) => /Defenseman|Forward/.test(o.label))).toBe(false);
  });

  it("never borrows skater labels for a goalie row", () => {
    const opts = archetypeChoices(cell({ enum_options: [{ raw: 1, label: "Defensive Defenseman" }], range_low: 0, range_high: 3 }), true);
    expect(opts).toEqual([0, 1, 2, 3].map((r) => ({ raw: r, label: String(r) })));
  });

  it("is empty without a cell", () => {
    expect(archetypeChoices(null, false)).toEqual([]);
  });
});
