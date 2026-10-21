import { describe, expect, it } from "vitest";
import { formatBytes, isActivePhase } from "./portraitInstall";

describe("portrait install helpers", () => {
  it("names the running phases and nothing else", () => {
    expect(isActivePhase("downloading")).toBe(true);
    expect(isActivePhase("extracting")).toBe(true);
    for (const p of ["idle", "done", "failed", "cancelled", undefined]) expect(isActivePhase(p)).toBe(false);
  });
  it("formats sizes at a readable precision", () => {
    expect(formatBytes(31_737_672_102)).toBe("29.6 GiB");
    expect(formatBytes(2 * 1024 * 1024 * 1024)).toBe("2.00 GiB");
    expect(formatBytes(68_516_660)).toBe("65 MiB");
    expect(formatBytes(1536)).toBe("2 KiB");
    expect(formatBytes(12)).toBe("12 B");
  });
});
