import { useEffect, useRef, useState } from "react";
import { Outlet } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { Button, EmptyState, Spinner } from "./primitives";

const PATH_KEY = "fhm_corpus_path";

/// Ensures the app-global FHM historical corpus is open before the Historical browse renders. The
/// corpus is a separately-distributed data pack (not shipped in-repo), so until a bundled-resource
/// path is wired, the user points at their built `.db` once; the path persists in localStorage and
/// auto-opens on subsequent visits. Renders the child routes via `<Outlet/>` once open.
export function CorpusGate() {
  const qc = useQueryClient();
  const status = useQuery({ queryKey: ["corpus_open"], queryFn: () => call("corpus_open", {}) });
  const [path, setPath] = useState(() => localStorage.getItem(PATH_KEY) ?? "");

  const open = useMutation({
    mutationFn: (p: string) => call("open_corpus", { path: p }),
    onSuccess: (_count, p) => {
      localStorage.setItem(PATH_KEY, p);
      qc.invalidateQueries({ queryKey: ["corpus_open"] });
    },
  });

  // Auto-open once from the remembered path when the corpus isn't already open.
  const tried = useRef(false);
  useEffect(() => {
    if (status.data === false && !tried.current && path.trim() && !open.isPending) {
      tried.current = true;
      open.mutate(path.trim());
    }
  }, [status.data, path, open]);

  if (status.isLoading) return <Spinner label="Checking historical corpus…" />;
  if (status.data) return <Outlet />;

  return (
    <div className="mx-auto max-w-xl space-y-4 py-8">
      <EmptyState
        title="Locate the historical data pack"
        hint="The FHM historical corpus ships as a separate data pack. Point at the built corpus .db (produced by tdb-fhm-build) to browse it. This is remembered for next time."
      />
      <form
        className="flex gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          if (path.trim()) open.mutate(path.trim());
        }}
      >
        <input
          value={path}
          onChange={(e) => setPath(e.target.value)}
          placeholder="e.g. C:\data\fhm-corpus.db"
          className="min-w-0 flex-1 rounded-md border border-slate-700 bg-slate-950 px-3 py-1.5 text-sm text-slate-100 placeholder-slate-600 focus:border-sky-600 focus:outline-none"
        />
        <Button type="submit" disabled={open.isPending || !path.trim()}>
          {open.isPending ? "Opening…" : "Open"}
        </Button>
      </form>
      {open.isError && <p className="text-sm text-rose-400">{String(open.error)}</p>}
    </div>
  );
}
