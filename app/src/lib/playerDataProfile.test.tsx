// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
const mock = vi.hoisted(() => ({ call: vi.fn() }));
vi.mock("./client", () => ({ call: mock.call }));
vi.mock("./project", () => ({ useProject: () => ({ update: () => {} }) }));
import { playerDataProfile } from "./playerDataProfile";
import { ProfileSections } from "../components/FieldProfile";
import type { PlayerViewDto } from "../bindings/PlayerViewDto";

function player(table = "dSvy", record: number | null = 12, editable = true): PlayerViewDto {
  return {
    bio_record: 281, ratings_record: record,
    attributes: { style: { field: { studio_table: table, studio_field: "mQoE", name: "p_style", editable,
      display: "Playmaker", decoded: 2, enum_options: [{ raw: 2, label: "Playmaker" }, { raw: 4, label: "Sniper" }],
      is_string: false, enum_label: "Playmaker", edit_offset: null, edit_scale: null, edit_height: false } } },
    profile: { sections: [{ title: "Roster Data", cells: [{ label: "Archetype", control: "derived", field: null,
      record: 0, derived_display: "Playmaker", color_fields: [], unverified: null }] }] },
  } as unknown as PlayerViewDto;
}
function mount(p: PlayerViewDto) {
  render(<QueryClientProvider client={new QueryClient()}>
    <ProfileSections projectId={1} profile={playerDataProfile(p)} teams={[]} />
  </QueryClientProvider>);
}
beforeEach(() => { mock.call.mockReset(); mock.call.mockResolvedValue({ project_id: 1 }); });
afterEach(cleanup);

it.each([["dSvy", 12], ["mHuy", 0]] as const)("edits archetype in the supplied %s companion record", async (table, record) => {
  const p = player(table, record);
  mount(p);
  fireEvent.click(screen.getByRole("button", { name: "Playmaker" }));
  const select = screen.getByRole("combobox", { name: "Archetype" }) as HTMLSelectElement;
  expect(select.selectedOptions[0].textContent).toBe("Playmaker");
  fireEvent.change(select, { target: { value: "4" } });
  expect(screen.getByRole("button", { name: "Sniper" })).toBeTruthy();
  await waitFor(() => expect(mock.call).toHaveBeenCalledWith("set_field", {
    projectId: 1, table, record, field: "mQoE", intValue: 4,
  }));
  expect(p.profile.sections[0].cells[0].control).toBe("derived"); // Cached API data is untouched.
});

it("does not invent an editable target when the ratings record is missing", () => {
  const p = player("dSvy", null);
  expect(playerDataProfile(p)).toBe(p.profile);
  mount(p);
  expect(screen.queryByRole("button", { name: "Playmaker" })).toBeNull();
});

it("preserves the actual field's read-only protection", () => {
  mount(player("dSvy", 12, false));
  expect(screen.getByText("Playmaker")).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Playmaker" })).toBeNull();
});
