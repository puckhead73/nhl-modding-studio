import { describe, expect, it } from "vitest";
import { CONSOLE_ICON, CONSOLE_LABEL, GAMES, gameById, gamesForPlatform, img } from "./catalog";

describe("game catalog", () => {
  it("lists NHL 22–24 as PS4-only titles marked in development, after NHL 21", () => {
    for (const id of ["nhl22", "nhl23", "nhl24"]) {
      const g = gameById(id);
      expect(g?.platforms).toEqual(["ps4"]);
      expect(g?.inDevelopment).toBe(true);
    }
    const ps4 = gamesForPlatform("ps4").map((g) => g.id);
    expect(ps4.slice(-4)).toEqual(["nhl21", "nhl22", "nhl23", "nhl24"]);
  });

  it("lists NHL 25–27 as PS5-only titles marked in development", () => {
    for (const id of ["nhl25", "nhl26", "nhl27"]) {
      const g = gameById(id);
      expect(g?.platforms).toEqual(["ps5"]);
      expect(g?.inDevelopment).toBe(true);
    }
    expect(gamesForPlatform("ps5").map((g) => g.id)).toEqual(["nhl25", "nhl26", "nhl27"]);
    expect(CONSOLE_LABEL.ps5).toBe("PS5");
    expect(CONSOLE_ICON.ps5).toBe("ps5.png");
  });

  it("has bundled art for every title and console icon", () => {
    for (const g of GAMES) expect(img(g.file), g.file).not.toBe("");
    for (const file of Object.values(CONSOLE_ICON)) expect(img(file!), file).not.toBe("");
  });

  it("keeps game ids unique and established titles unmarked", () => {
    expect(new Set(GAMES.map((g) => g.id)).size).toBe(GAMES.length);
    expect(GAMES.filter((g) => g.inDevelopment).map((g) => g.id)).toEqual(["nhl22", "nhl23", "nhl24", "nhl25", "nhl26", "nhl27"]);
  });
});
