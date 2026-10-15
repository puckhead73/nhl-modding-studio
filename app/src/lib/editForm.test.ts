import { describe, expect, it } from "vitest";
import { friendlySeed, friendlyToRaw } from "./editForm";
import type { FieldViewDto } from "../bindings/FieldViewDto";

describe("NHL07 player display units", () => {
  it("edits the contract in dollars and keeps the 25000-dollar storage unit", () => {
    const salary = { decoded: 180, edit_scale: 25000 } as FieldViewDto;
    expect(friendlySeed(salary)).toBe("4500000");
    expect(friendlyToRaw(salary, "5000000")).toBe(200);
    expect(friendlyToRaw(salary, "")).toBeNull();
  });
  it("maps the displayed NHL07 rating back to its stored value", () => {
    const speed = { decoded: 42, edit_offset: 50 } as FieldViewDto;
    expect(friendlySeed(speed)).toBe("92");
    expect(friendlyToRaw(speed, "93")).toBe(43);
  });
});
