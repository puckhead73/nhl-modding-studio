import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { call, type Platform } from "../lib/client";
import { Button, Spinner } from "./primitives";

export type GameFilesReport = {
  status: "vanilla" | "different" | "additional" | "unverified";
  reference: string;
  region: string;
  matched: number;
  loose: number;
  containers: number;
  findings: { path: string; issue: string }[];
};

/** A mismatch needs acknowledgement, never replacement of the user's files. */
export function VanillaCheck({ root, gameId, platform, onReady }: {
  root: string; gameId: string; platform: Platform; onReady: (ready: boolean) => void;
}) {
  const [accepted, setAccepted] = useState<string | null>(null);
  const scan = useQuery({
    queryKey: ["game_files_check", root, gameId, platform],
    queryFn: () => call("game_files_check", { root, gameId, platform }),
    retry: false, staleTime: 0, refetchOnWindowFocus: false,
  });
  const result = scan.data;
  const verified = !scan.isError && result && result.status !== "unverified";
  const status = verified ? result.status === "vanilla" ? "Vanilla" : "Modified" : "Unknown";
  // A differing installation does not establish the region of the selected files.
  const region = verified && (result.status === "vanilla" || result.status === "additional") ? result.region : "Unknown";
  // Acceptance belongs to this exact source and scan, not a previous folder/result.
  const identity = JSON.stringify([root, gameId, platform, scan.dataUpdatedAt, scan.errorUpdatedAt]);
  const warning = scan.isError || (!!result && result.status !== "vanilla");
  const ready = !scan.isFetching && (!!result || scan.isError) && (!warning || accepted === identity);
  useEffect(() => { onReady(ready); return () => onReady(false); }, [ready, onReady]);
  return <div className="mb-3 rounded-md border border-slate-700 p-3 text-xs text-slate-300" data-testid="vanilla-check">
    <div className="mb-2 flex items-center justify-between gap-3">
      <h2 className="text-sm font-semibold">Game file verification</h2>
      <Button type="button" variant="secondary" disabled={scan.isFetching} onClick={() => {
        setAccepted(null);
        onReady(false);
        void scan.refetch();
      }}>Rescan</Button>
    </div>
    {scan.isFetching ? <Spinner label="Checking game files…" /> : <>
      <p className="flex items-center gap-4"><span className={status === "Vanilla" ? "font-medium text-emerald-300" : "font-medium text-amber-200"}>{status}</span><span>Region: {region}</span></p>
      {warning && <label className="mt-3 flex items-start gap-2 text-amber-200"><input type="checkbox" checked={accepted === identity} onChange={e => setAccepted(e.target.checked ? identity : null)} />
        {status === "Modified" ? "Continue with modified files" : "Continue with unverified files"}
      </label>}
    </>}
  </div>;
}
