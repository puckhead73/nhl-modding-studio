// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, expect, it, vi } from "vitest";
import { EquipmentDetail } from "./EquipmentTab";

const mocks = vi.hoisted(() => ({ call: vi.fn(async () => ({})), update: vi.fn() }));
vi.mock("../lib/client", () => ({ call: mocks.call }));
vi.mock("../lib/project", () => ({ useProject: () => ({ update: mocks.update }) }));
afterEach(() => { cleanup(); mocks.call.mockClear(); });

it.each([true, false])("writes the correct color field and preserves other packed channels (packed=%s)", async (packed) => {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const zone = { red: 17, green: 43, blue: 91, ...(packed ? { packed_field: "Sgir" } : {}) };
  render(<QueryClientProvider client={client}><EquipmentDetail projectId={1} record={{ record: 7, equipment_id: 4, equipment_type_raw: 5, brand_raw: 0, era_raw: 0, variant_raw: 0, zones: packed ? Array.from({ length: 15 }, () => zone) : [zone] }} /></QueryClientProvider>);
  const input = screen.getAllByLabelText("R")[0];
  fireEvent.change(input, { target: { value: "34" } });
  fireEvent.blur(input);
  await waitFor(() => expect(mocks.call).toHaveBeenCalledWith("set_field", { projectId: 1, table: "hHbv", record: 7, field: packed ? "Sgir" : "VaOV", intValue: packed ? 0x222b5b : 34 }));
  if (packed) expect(screen.getAllByText("Zone 15").length).toBeGreaterThan(0);
  client.clear();
});
