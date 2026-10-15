// @vitest-environment jsdom
import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { SettingsProvider, useSettings } from "./settings";
afterEach(() => { cleanup(); localStorage.clear(); });
it("defaults existing installations to ten minutes and persists a changed interval", () => {
  localStorage.setItem("nhl-modding-studio.settings.v1", JSON.stringify({ projectsRoot: "E:/Projects" }));
  const first = renderHook(() => useSettings(), { wrapper: SettingsProvider });
  expect(first.result.current.settings.autosaveMinutes).toBe(10);
  act(() => first.result.current.setAutosaveMinutes(3));
  first.unmount();
  const next = renderHook(() => useSettings(), { wrapper: SettingsProvider });
  expect(next.result.current.settings.autosaveMinutes).toBe(3);
  expect(next.result.current.settings.projectsRoot).toBe("E:/Projects");
  act(() => next.result.current.setAutosaveMinutes(0));
  expect(next.result.current.settings.autosaveMinutes).toBe(10);
});
