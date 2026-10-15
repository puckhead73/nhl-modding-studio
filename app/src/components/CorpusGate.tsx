import { useEffect, useRef, useState } from "react";
import { Outlet } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { Button, EmptyState, Spinner } from "./primitives";

const PATH_KEY = "fhm_corpus_path";

/// Ensures the app-global FHM historical corpus is open before the Historical browse renders. The
/// Empty paths select the bundled portable data pack; explicit selections remain remembered.
/// Renders the child routes via `<Outlet/>` once open.
export function CorpusGate() {
  const qc = useQueryClient();
  const status = useQuery({ queryKey: ["corpus_open"], queryFn: () => call("corpus_open", {}) });
  const [path, setPath] = useState(() => localStorage.getItem(PATH_KEY) ?? "");

  const open = useMutation({
    mutationFn: (p: string) => call("open_corpus", { path: p }),
    onSuccess: (_count, p) => {
      if (p) localStorage.setItem(PATH_KEY, p);
      else localStorage.removeItem(PATH_KEY);
      qc.invalidateQueries({ queryKey: ["corpus_open"] });
    },
  });

  // An empty selection resolves beside the executable, so a moved archive still works.
  const tried = useRef(false);
  useEffect(() => {
    if (status.data === false && !tried.current && !open.isPending) {
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
        hint="Studio opens the bundled historical database automatically. Leave the path empty to use it, or enter another historical database path."
      />
      <form
        className="flex gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          open.mutate(path.trim());
        }}
      >
        <input
          aria-label="Historical database path"
          value={path}
          onChange={(e) => setPath(e.target.value)}
          placeholder="Bundled historical database"
          className="min-w-0 flex-1 rounded-md border border-slate-700 bg-slate-950 px-3 py-1.5 text-sm text-slate-100 placeholder-slate-600 focus:border-sky-600 focus:outline-none"
        />
        <Button type="submit" disabled={open.isPending}>
          {open.isPending ? "Opening…" : path.trim() ? "Open" : "Use bundled data"}
        </Button>
      </form>
      {open.isError && <p className="text-sm text-rose-400">{String(open.error)}</p>}
    </div>
  );
}
