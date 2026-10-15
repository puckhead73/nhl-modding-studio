// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { InstallationRecovery } from "./InstallationRecovery";
import type { InstallRecordDto } from "../bindings/InstallRecordDto";
const call = vi.hoisted(() => vi.fn());
vi.mock("../lib/client", () => ({ call }));
beforeEach(() => { HTMLDialogElement.prototype.showModal = function () { this.setAttribute("open", ""); }; });
afterEach(() => { cleanup(); call.mockReset(); });
const record = { record_dir: "records/2", package_name: "Old mod" } as InstallRecordDto;
function mount() {
  const recovered = vi.fn(async () => {});
  const close = vi.fn();
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><InstallationRecovery records={[record]} onClose={close} onRecovered={recovered} /></QueryClientProvider>);
  return { recovered, close };
}
it("requires confirmation, sends the reviewed identity, and reports preserved copies", async () => {
  call.mockImplementation(async cmd => cmd === "package_recovery_review" ? { record_dir: "records/2", review_id: "fingerprint", conflicts: ["jersey.rpsgl"], destinations: ["jersey.rpsgl"] } : { state: "restored", detail: "Preserved in records/2/recovery-preserved-1" });
  const { recovered } = mount();
  await screen.findByText(/Files different from both/);
  const apply = screen.getByRole("button", { name: "Preserve files and restore originals" }) as HTMLButtonElement;
  expect(apply.disabled).toBe(true);
  expect(call).toHaveBeenCalledTimes(1);
  fireEvent.click(screen.getByRole("checkbox"));
  fireEvent.click(apply);
  await waitFor(() => expect(recovered).toHaveBeenCalledWith("Preserved in records/2/recovery-preserved-1"));
  expect(call).toHaveBeenCalledWith("package_recovery_apply", { recordDir: "records/2", reviewId: "fingerprint" });
});
it("does not permit recovery without verified backups", async () => {
  call.mockRejectedValue(new Error("Backup is not the recorded prior file"));
  mount();
  await screen.findByRole("alert");
  expect((screen.getByRole("checkbox") as HTMLInputElement).disabled).toBe(true);
  expect((screen.getByRole("button", { name: "Preserve files and restore originals" }) as HTMLButtonElement).disabled).toBe(true);
  expect(call).toHaveBeenCalledTimes(1);
});
it("requires a new review and confirmation after a stale-review refusal", async () => {
  let revision = 0;
  call.mockImplementation(async cmd => {
    if (cmd === "package_recovery_review") return { record_dir: "records/2", review_id: String(++revision), conflicts: [], destinations: [] };
    throw new Error("Files changed since review");
  });
  mount();
  await screen.findByText(/Files different from both/);
  fireEvent.click(screen.getByRole("checkbox"));
  fireEvent.click(screen.getByRole("button", { name: "Preserve files and restore originals" }));
  await screen.findByRole("alert");
  expect((screen.getByRole("checkbox") as HTMLInputElement).checked).toBe(false);
  fireEvent.click(screen.getByRole("button", { name: "Review again" }));
  await waitFor(() => expect(revision).toBe(2));
  expect((screen.getByRole("button", { name: "Preserve files and restore originals" }) as HTMLButtonElement).disabled).toBe(true);
});
