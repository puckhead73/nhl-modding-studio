import { useEffect, useRef, useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { call, type Platform } from "../lib/client";
import type { InstallReportDto } from "../bindings/InstallReportDto";
import { ExtractionPanel } from "./ExtractionPanel";
import { Button, ErrorBanner } from "./primitives";

/** Scan an import source without linking it as a game/save deployment destination. */
export function SourceFolderPanel({ root, gameId, platform, onReady, onBusyChange }: {
  root: string;
  gameId: string;
  platform: Platform;
  onReady: (report: InstallReportDto) => void;
  onBusyChange: (busy: boolean) => void;
}) {
  const [packedRoot, setPackedRoot] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; };
  }, []);
  const scan = useMutation({
    mutationFn: () => call("detect_install", { gameId, platform, root }),
    onSuccess: (report) => {
      if (!mounted.current) return;
      if (report.nhlng_path) { setPackedRoot(null); onReady(report); }
      // PS4 can identify the title folder before its packed database is available.
      else setPackedRoot(report.extract_needed ? report.game_dir ?? report.install_dir : null);
    },
  });
  useEffect(() => {
    onBusyChange(busy || scan.isPending);
    return () => onBusyChange(false);
  }, [busy, scan.isPending, onBusyChange]);
  return <div className="space-y-2">
    <Button disabled={!root.trim() || busy || scan.isPending} onClick={() => scan.mutate()}>
      Find database in this folder
    </Button>
    {packedRoot && <>
      <p className="text-xs text-amber-200">The database is packed. Extract the containers, then choose Find database in this folder again.</p>
      <ExtractionPanel root={packedRoot} platform={platform} mode="settings" onBusyChange={setBusy} />
    </>}
    {scan.isSuccess && !scan.data.nhlng_path && !packedRoot && <p className="text-xs text-slate-400">
      No database was found here. Choose a game disc folder, or continue with an assets-only project if these are the assets you want.
    </p>}
    {scan.error && <ErrorBanner title="Source folder" message={String(scan.error)} />}
  </div>;
}
