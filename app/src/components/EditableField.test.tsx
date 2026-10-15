// @vitest-environment jsdom
//
// P3.2-E (25 Sep 2026) — an inline-string field knows its byte width, so a name that would not
// fit is refused **before** the write, in words, and nothing is sent; a fitting value commits as
// before; the check counts UTF-8 bytes, not characters.
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const calls: Array<{ cmd: string; args: any }> = [];
vi.mock("../lib/client", () => ({
  call: (cmd: string, args: any) => {
    calls.push({ cmd, args });
    if (cmd === "set_field") return Promise.resolve({ edits: 1, repairs: [] });
    if (cmd === "project_summary") return Promise.resolve({});
    return Promise.reject(new Error(`unmocked ${cmd}`));
  },
}));
vi.mock("../lib/project", () => ({
  useProject: () => ({ update: () => {} }),
}));

import { EditableField } from "./EditableField";

const lastName: any = {
  studio_table: "ubPc",
  studio_field: "QbMR",
  name: "lastname",
  kind: "string",
  display: "Kukkonen",
  decoded: null,
  enum_label: null,
  enum_options: [],
  edit_offset: null,
  edit_height: false,
  edit_scale: null,
  raw_int: null,
  text: "Kukkonen",
  range_low: null,
  range_high: null,
  editable: true,
  is_string: true,
  text_max_bytes: 12,
};

function mount(field: any = lastName) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <MemoryRouter>
      <QueryClientProvider client={qc}>
        <EditableField projectId={1} record={0} field={field} label="Last name" />
      </QueryClientProvider>
    </MemoryRouter>,
  );
}

beforeEach(() => {
  calls.length = 0;
});
afterEach(cleanup);

describe("EditableField inline strings", () => {
  it("refuses a value over the byte width before the write and commits one that fits", async () => {
    mount();
    fireEvent.click(screen.getByText("Kukkonen"));
    const input = screen.getByTestId("text-number-input") as HTMLInputElement;
    // 13 ASCII bytes into a 12-byte field: refused, nothing sent.
    fireEvent.change(input, { target: { value: "Sweeptestname" } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(screen.getByTestId("text-too-long").textContent).toMatch(/13 bytes — this field holds at most 12/);
    expect(calls.filter((c) => c.cmd === "set_field")).toHaveLength(0);
    // The shell closed on Enter and shows the unchanged value; reopen and type a fitting value:
    // 10 characters but 12 UTF-8 bytes — fits exactly and commits.
    fireEvent.click(screen.getByText("Kukkonen"));
    const again = screen.getByTestId("text-number-input") as HTMLInputElement;
    fireEvent.change(again, { target: { value: "Litoměřice" } });
    fireEvent.keyDown(again, { key: "Enter" });
    await waitFor(() => expect(calls.filter((c) => c.cmd === "set_field")).toHaveLength(1));
    expect(calls.find((c) => c.cmd === "set_field")!.args).toEqual({ projectId: 1, table: "ubPc", record: 0, field: "QbMR", textValue: "Litoměřice" });
    expect(screen.queryByTestId("text-too-long")).toBeNull();
  });

  it("counts UTF-8 bytes, not characters", () => {
    mount();
    fireEvent.click(screen.getByText("Kukkonen"));
    const input = screen.getByTestId("text-number-input") as HTMLInputElement;
    // 11 characters, 13 bytes (two accented letters).
    fireEvent.change(input, { target: { value: "Litoměřicee" } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(screen.getByTestId("text-too-long").textContent).toMatch(/13 bytes/);
    expect(calls.filter((c) => c.cmd === "set_field")).toHaveLength(0);
  });
});

// Receipt 28 — a labelled dropdown names only the codes established for the title (e.g. national-team
// nationality codes); any other code in the declared range (0..255 on NHL 20 / 21) is still reachable
// through the stored-code box, and a code past the range is neither written nor clamped.
const nationality: any = {
  ...lastName,
  studio_field: "Lelh",
  name: "intlcountry",
  kind: "int",
  display: "Czech Republic",
  decoded: 82,
  enum_label: "Czech Republic",
  enum_options: [
    [82, "Czech Republic"],
    [85, "Germany"],
  ].map(([raw, label]) => ({ raw, label })),
  text: null,
  range_low: 0,
  range_high: 255,
  is_string: false,
  text_max_bytes: null,
};

describe("Enum fields retain their names in edit mode", () => {
  it.each([
    ["QvxI", "Birth State", 88, "Slovakia", 86, "Poland"],
    ["GRkp", "Handedness", 0, "Left", 1, "Right"],
    ["vjla", "Position", 0, "C", 1, "LW"],
    ["Lelh", "Nationality", 82, "Czech Republic", 85, "Germany"],
  ])("opens %s as named choices and commits the selected ID", async (id, label, raw, name, nextRaw, nextName) => {
    mount({ ...nationality, studio_field: id, name: label, decoded: raw, display: name,
      enum_options: [{ raw, label: name }, { raw: nextRaw, label: nextName }] });
    fireEvent.click(screen.getByRole("button", { name: String(name) }));
    const select = screen.getByRole("combobox") as HTMLSelectElement;
    expect(select.selectedOptions[0].textContent).toBe(name);
    expect(Array.from(select.options, (o) => o.textContent)).toEqual([name, nextName]);
    expect(screen.queryByRole("spinbutton")).toBeNull();
    expect(screen.queryByRole("textbox")).toBeNull();
    expect(document.activeElement).toBe(select);
    expect(calls.filter((c) => c.cmd === "set_field")).toHaveLength(0);
    fireEvent.change(select, { target: { value: String(nextRaw) } });
    await waitFor(() => expect(calls.filter((c) => c.cmd === "set_field")).toHaveLength(1));
    expect(calls.find((c) => c.cmd === "set_field")!.args).toEqual({ projectId: 1, table: "ubPc", record: 0, field: id, intValue: nextRaw });
  });

  it("preserves an unknown current code without guessing a label or writing on open", () => {
    mount({ ...nationality, decoded: 200, display: "200", enum_label: null });
    fireEvent.click(screen.getByRole("button", { name: "200" }));
    const select = screen.getByRole("combobox") as HTMLSelectElement;
    expect(select.selectedOptions[0].textContent).toBe("Unknown value (200)");
    expect(screen.queryByRole("spinbutton")).toBeNull();
    fireEvent.keyDown(select, { key: "Escape" });
    expect(screen.queryByRole("combobox")).toBeNull();
    expect(calls.filter((c) => c.cmd === "set_field")).toHaveLength(0);
  });
});
describe("NHL07 friendly player inputs", () => {
  it.each([
    ["kKhd", "contractdollars", "$4,500,000", 180, 25000, null, "5000000", 200],
    ["qPWY", "p_speed", "92", 42, null, 50, "93", 43],
    ["qFnd", "year", "1974", 15, null, 1959, "1975", 16],
  ])("stages %s in database units", async (id, name, display, raw, scale, offset, input, expected) => {
    mount({ ...lastName, studio_field: id, name, display, decoded: raw, edit_scale: scale,
      edit_offset: offset, is_string: false, kind: "integer", text: null, text_max_bytes: null,
      range_low: 0, range_high: 511 });
    fireEvent.click(screen.getByText(String(display)));
    const field = screen.getByTestId("text-number-input");
    fireEvent.change(field, { target: { value: input } });
    fireEvent.keyDown(field, { key: "Enter" });
    await waitFor(() => expect(calls.filter((c) => c.cmd === "set_field")).toHaveLength(1));
    expect(calls.find((c) => c.cmd === "set_field")!.args.intValue).toBe(expected);
  });
});
