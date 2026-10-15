import { describe, expect, it } from "vitest";
import { outfitNamedGroup, outfitStyleVariant } from "./UniformsTab";

describe("native uniform grouping", () => {
  it("groups PS4 map families without inventing a numeric registration", () => {
    for (const prefix of ["jersey", "font", "pant", "sock"]) {
      for (const map of ["cm", "nm", "sm"]) {
        expect(outfitNamedGroup(`${prefix}_nineties_anaheim_away_${map}.gnf`)).toBe("nineties_anaheim_away");
      }
    }
    expect(outfitNamedGroup("jersey_nineties_anaheim_home_cm.GNF")).toBe("nineties_anaheim_home");
    expect(outfitNamedGroup("goaliesticktape_6_cm.gnf")).toBeNull();
    expect(outfitNamedGroup("jersey_0_10_1_cm.rx2")).toBeNull();
  });
  it("keeps PS3 bundles and loose maps in the same kit as their Xbox counterparts", () => {
    for (const extension of ["rx2", "rpsgl", "RPSGL"]) {
      for (const name of ["texlib_0_10_1", "jersey_0_10_1_cm", "jersey_0_10_1_nm"]) {
        expect(outfitStyleVariant(`${name}.${extension}`)).toEqual({ style: 0, variant: 1 });
      }
    }
    expect(outfitStyleVariant("texlib_0_10.rpsgl")).toBeNull();
  });
});
