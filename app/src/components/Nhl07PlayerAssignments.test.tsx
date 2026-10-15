// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { AssignmentRow } from "./Nhl07PlayerAssignments";
import type { PlayerAssignment } from "../lib/client";
afterEach(cleanup);
const item: PlayerAssignment = { key: "stick", label: "Stick", section: "Equipment", current: "4", availability: "Not verified in game source", options: [
  { key: "4", label: "Pavol Demitra (option 4)", users: 10 },
  { key: "2", label: "Jaromir Jagr (option 2)", users: 15 },
] };
it("echoes the named selection immediately and rolls back a refused write", async () => {
  let reject!: (reason: string) => void;
  const write = vi.fn(() => new Promise<void>((_, no) => { reject = no; }));
  render(<AssignmentRow item={item} onChange={write} />);
  const select = screen.getByRole("combobox", { name: "Stick" }) as HTMLSelectElement;
  fireEvent.change(select, { target: { value: "2" } });
  expect(select.value).toBe("2"); expect(select.selectedOptions[0].textContent).toContain("Jaromir Jagr");
  expect(write).toHaveBeenCalledWith("2");
  await act(async () => reject("Selection is unavailable"));
  expect(select.value).toBe("4"); expect(select.disabled).toBe(false);
  expect(screen.getByRole("alert").textContent).toContain("Selection is unavailable");
});
it("preserves an imported custom value without advertising it as a stock option", () => {
  render(<AssignmentRow item={{ ...item, current: "99", options: [...item.options, { key: "99", label: "Current custom assignment (99)", users: 0 }] }} onChange={vi.fn()} />);
  const select = screen.getByRole("combobox") as HTMLSelectElement;
  expect(select.value).toBe("99"); expect(select.selectedOptions[0].disabled).toBe(true);
  expect(select.options[0].disabled).toBe(false);
});
it("searches a large catalogue while retaining the current selection", () => {
  const options = [...item.options, ...Array.from({ length: 40 }, (_, i) => ({ key: String(100+i), label: `Player ${i}`, users: 1 }))];
  render(<AssignmentRow item={{ ...item, options }} onChange={vi.fn()} />);
  fireEvent.change(screen.getByRole("textbox", { name: "Search Stick" }), { target: { value: "Jagr" } });
  const select = screen.getByRole("combobox") as HTMLSelectElement;
  expect(select.options.length).toBe(2); expect(select.value).toBe("4");
  expect(select.options[1].textContent).toContain("Jagr");
});
