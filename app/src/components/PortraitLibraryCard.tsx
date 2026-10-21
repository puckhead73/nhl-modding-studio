// Settings → General: the supplementary portrait database — where it lives, whether it is
// installed, download-and-install in the background, or install from an archive on disk.
import { useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { Button, Card } from "./primitives";
import { PortraitInstallBanner } from "./PortraitInstallBanner";
import { formatBytes, isActivePhase, usePortraitInstallActions, usePortraitInstallPlan, usePortraitInstallStatus } from "../lib/portraitInstall";

export function PortraitLibraryCard() {
  const plan = usePortraitInstallPlan();
  const status = usePortraitInstallStatus();
  const { download, installArchive } = usePortraitInstallActions();
  const [actionError, setActionError] = useState<string | null>(null);
  const p = plan.data;
  const active = isActivePhase(status.data?.phase);

  async function pickArchive() {
    setActionError(null);
    const picked = await openDialog({
      title: "Choose the portrait database archive",
      multiple: false,
      directory: false,
      filters: [{ name: "7z archive", extensions: ["7z"] }],
    });
    if (typeof picked !== "string" || !picked) return;
    installArchive.mutate(picked, { onError: (e) => setActionError(String(e)) });
  }

  const freeShort = p?.free_bytes != null && p.free_bytes < p.expected_bytes + p.expected_bytes / 8;

  return (
    <Card
      title="Portrait database"
      subtitle="Player portraits for the historical database and the roster builder — about 110,000 images, downloaded once."
      className="mt-4"
    >
      <div className="space-y-3 text-xs text-slate-300">
        {plan.isError && <p className="text-rose-300">{String(plan.error)}</p>}
        {p && (
          <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1">
            <dt className="text-slate-500">Status</dt>
            <dd>
              {p.installed ? (
                <span className="text-emerald-300">Installed</span>
              ) : p.downloaded_archive ? (
                <span className="text-amber-300">Downloaded, not installed</span>
              ) : p.partial_bytes > 0 ? (
                <span className="text-amber-300">Download paused at {formatBytes(p.partial_bytes)} — resumes when started</span>
              ) : (
                <span className="text-slate-400">Not installed</span>
              )}
            </dd>
            <dt className="text-slate-500">Location</dt>
            <dd className="truncate font-mono text-[11px] text-slate-400" title={p.install_root}>{p.install_root}</dd>
            <dt className="text-slate-500">Download</dt>
            <dd>
              {formatBytes(p.expected_bytes)} archive
              {p.free_bytes != null && (
                <span className={freeShort ? "text-amber-300" : "text-slate-500"}>
                  {" "}· {formatBytes(p.free_bytes)} free on that drive{freeShort ? " — likely not enough for the download plus the extracted files" : ""}
                </span>
              )}
            </dd>
          </dl>
        )}

        <div className="rounded border border-slate-800">
          <PortraitInstallBanner compact />
        </div>

        <div className="flex flex-wrap gap-2">
          <Button
            onClick={() => {
              setActionError(null);
              download.mutate(undefined, { onError: (e) => setActionError(String(e)) });
            }}
            disabled={active || download.isPending}
          >
            {p?.downloaded_archive ? "Install the downloaded archive" : p && p.partial_bytes > 0 ? "Resume download and install" : "Download and install"}
          </Button>
          <Button variant="secondary" onClick={() => void pickArchive()} disabled={active || installArchive.isPending}>
            Install from archive…
          </Button>
          {p && (
            <a
              href={p.source_url}
              target="_blank"
              rel="noreferrer"
              className="self-center text-[11px] text-sky-300 underline-offset-2 hover:underline"
            >
              Open the download page
            </a>
          )}
        </div>
        {actionError && <p className="text-rose-300">{actionError}</p>}
        <p className="text-[11px] text-slate-500">
          The download runs in the background while you work and resumes if it stops. Installing extracts the
          archive next to the historical database, where the Studio looks for it; an existing library at that
          location is set aside, not deleted.
        </p>
      </div>
    </Card>
  );
}
