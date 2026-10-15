// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import { afterEach, expect, it, vi } from "vitest";
const mock = vi.hoisted(() => ({ call: vi.fn(), update: vi.fn() }));
vi.mock("../lib/client", () => ({ call: mock.call }));
vi.mock("../lib/project", () => ({ useProject: () => ({ update: mock.update }) }));
import { StrategiesTab } from "./StrategiesTab";
afterEach(() => { cleanup(); vi.clearAllMocks(); });
it("displays a strategy selection before validation and restores it if refused", async () => {
  let reject!: (error: Error) => void;
  const field = { studio_table: "EeYo", studio_field: "xKvv", decoded: 0, raw_int: 0, display: "Normal", editable: true, enum_options: [{ raw: 0, label: "Normal" }, { raw: 1, label: "High" }] };
  mock.call.mockImplementation(async command => {
    if (command === "team_strategies") return { team_id: 14, record: 0, tactics: [{ label: "Defensive Pressure", field }], forward_biases: [], defense_biases: [], team_biases: [] };
    if (command === "lines_view") return { entries: [], slot_fields: [] };
    if (command === "list_players") return [];
    if (command === "set_field") return new Promise((_resolve, fail) => { reject = fail; });
    return {};
  });
  render(<MemoryRouter><QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><StrategiesTab projectId={1} teamId={14} /></QueryClientProvider></MemoryRouter>);
  const select = await screen.findByRole("combobox") as HTMLSelectElement;
  fireEvent.change(select, { target: { value: "1" } });
  expect(select.value).toBe("1");
  await waitFor(() => expect(mock.call).toHaveBeenCalledWith("set_field", { projectId: 1, table: "EeYo", record: 0, field: "xKvv", intValue: 1 }));
  reject(new Error("Strategy refused"));
  await waitFor(() => expect(select.value).toBe("0"));
});
