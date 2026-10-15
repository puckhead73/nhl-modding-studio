import { useEffect, useRef, useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import type { InstallRecordDto } from "../bindings/InstallRecordDto";
import { Button } from "./primitives";

/** Recover newest first so older installations cannot overwrite a newer stack. */
export function InstallationRecovery({ records, onClose, onRecovered }: {
  records: InstallRecordDto[];
  onClose: () => void;
  onRecovered: (message: string) => Promise<void>;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [confirmed, setConfirmed] = useState(false);
  const record = records[0];
  useEffect(() => { dialog.current?.showModal(); }, []);
  const review = useQuery({ queryKey: ["installation-recovery", record?.record_dir],
    queryFn: () => call("package_recovery_review", { recordDir: record.record_dir }),
    enabled: !!record, retry: false, staleTime: 0, refetchOnWindowFocus: false });
  const apply = useMutation({ mutationFn: async () => {
    if (!confirmed || !review.data) throw new Error("Review and confirm recovery first.");
    const result = await call("package_recovery_apply", { recordDir: review.data.record_dir, reviewId: review.data.review_id });
    if (result.state !== "restored") throw new Error(result.detail || "Recovery did not complete.");
    await onRecovered(result.detail || "Original files restored. You can activate your mod again.");
  }, onError: () => { setConfirmed(false); } });
  return <dialog ref={dialog} aria-label="Recover installation" onCancel={e => { e.preventDefault(); if (!apply.isPending) onClose(); }}
    className="w-[min(48rem,95vw)] rounded-xl border border-slate-700 bg-slate-950 p-6 text-slate-100 backdrop:bg-black/70">
    <h2 className="text-lg font-semibold">Recover installation</h2>
    <p className="mt-3 text-sm">Restore the files saved before installing {record?.package_name}. This restores that installation’s previous state; it does not certify that state as an untouched game extraction.</p>
    <p className="mt-2 text-sm">Studio will preserve copies of all files it replaces or removes in the installation record folder, then restore verified original backups. Your mod package is unchanged.</p>
    {records.length > 1 && <p className="mt-2 text-sm text-amber-300">There are {records.length} installation records. Recover the newest first, then repeat for older records if needed.</p>}
    {review.isFetching && <p role="status" className="mt-3">Checking files and backups…</p>}
    {review.isError && <p role="alert" className="mt-3 text-rose-300">Recovery unavailable: {String(review.error)}</p>}
    {review.data && <div className="my-4 max-h-64 overflow-auto space-y-3 text-sm">
      <h3 className="font-semibold">Files different from both the installed mod and the saved original ({review.data.conflicts.length})</h3>
      {review.data.conflicts.map(path => <p key={path} className="break-all text-amber-200">{path}</p>)}
      <details><summary>All files checked ({review.data.destinations.length})</summary>{review.data.destinations.map(path => <p key={path} className="break-all">{path}</p>)}</details>
      <p className="break-all">Preserved copies will be saved under: {record.record_dir}</p>
    </div>}
    {apply.isError && <p role="alert" className="my-3 text-rose-300">{String(apply.error)}</p>}
    <label className="my-4 flex gap-2 text-sm"><input type="checkbox" checked={confirmed} disabled={apply.isPending || review.isFetching || !review.data || review.isError} onChange={e => setConfirmed(e.target.checked)} />I understand this deactivates the recorded installation and restores its saved originals.</label>
    <div className="flex flex-wrap gap-2">
      <Button disabled={!confirmed || !review.data || review.isFetching || review.isError || apply.isPending || apply.isError} onClick={() => apply.mutate()}>{apply.isPending ? "Preserving and restoring…" : "Preserve files and restore originals"}</Button>
      <Button variant="secondary" disabled={apply.isPending || review.isFetching} onClick={() => { setConfirmed(false); apply.reset(); void review.refetch(); }}>Review again</Button>
      <Button variant="ghost" disabled={apply.isPending} onClick={onClose}>Cancel</Button>
    </div>
  </dialog>;
}
