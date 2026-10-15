// @vitest-environment jsdom
//
// About & licences (receipt 46): packaged candidate (status, blockers, notices, licence-file reader), a
// development build (no archive files: says so), missing files named, and a withheld Studio licence explained.
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const calls: Array<{ cmd: string; args: any }> = [];
let about: any = null;
vi.mock("../lib/client", () => ({
  call: (cmd: string, args: any) => {
    calls.push({ cmd, args });
    if (cmd === "about_licence_file") return Promise.resolve(`text of ${args.path}`);
    return Promise.resolve(about);
  },
}));

import { SettingsAboutPage } from "./About";

function mount() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter>
        <SettingsAboutPage />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

const packaged = {
  version: "0.0.0",
  studio_licence_spdx: "MIT",
  studio_licence_text: null,
  packaged: true,
  root: "E:/x/NHL Modding Studio rc",
  candidate: "rc-20260928-r46",
  release_ready: false,
  release_blockers: ["Studio's MIT licence file: the copyright holder has not been recorded."],
  notices: "NHL Modding Studio — third-party notices\n1. FFmpeg",
  licence_files: [{ path: "licenses/libspeex/COPYING" }, { path: "licenses/rust/zip-2.0.0/LICENSE" }],
  problems: [],
};

describe("About & licences", () => {
  beforeEach(() => {
    calls.length = 0;
  });
  afterEach(cleanup);

  it("shows the internal-build status with its blockers, the notices and a readable licence file", async () => {
    about = packaged;
    mount();
    expect(await screen.findByText("Internal review build — not a release")).toBeTruthy();
    expect(screen.getByText(/copyright holder has not been recorded/)).toBeTruthy();
    expect(screen.getByLabelText("Third-party notices").textContent).toContain("1. FFmpeg");
    expect(screen.getByText(/withheld rather than filled with a placeholder/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "licenses/libspeex/COPYING" }));
    await waitFor(() => expect(screen.getByLabelText("Text of licenses/libspeex/COPYING").textContent).toBe("text of licenses/libspeex/COPYING"));
    expect(calls.some((c) => c.cmd === "about_licence_file" && c.args.path === "licenses/libspeex/COPYING")).toBe(true);
  });

  it("says a development build has no packaged notices, without claiming a release status", async () => {
    about = { ...packaged, packaged: false, candidate: null, release_blockers: [], notices: null, licence_files: [], problems: ["This build is not running from an extracted release archive (X is not beside the executable), so the packaged licence and notices are not available here."] };
    mount();
    expect(await screen.findByText("Licence files are not available in this build")).toBeTruthy();
    expect(screen.queryByText("Internal review build — not a release")).toBeNull();
    expect(screen.getByText("The notices file is not available in this build.")).toBeTruthy();
  });

  it("names missing packaged files and shows a shipped Studio licence", async () => {
    about = { ...packaged, release_ready: true, release_blockers: [], studio_licence_text: "MIT License\n\nCopyright (c) 2026 Holder", problems: ["The notices name licenses/libspeex/COPYING, which is missing from E:/x."] };
    mount();
    expect(await screen.findByText("Files this build should include are missing")).toBeTruthy();
    expect(screen.getByText(/which is missing from/)).toBeTruthy();
    expect(screen.getByLabelText("Text of Studio's licence").textContent).toContain("MIT License");
    expect(screen.getByText(/every release check in its build record passed/)).toBeTruthy();
  });
});
