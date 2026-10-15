// Source folders are read locations; Mod Manager owns installation destinations.
import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { Button } from "./primitives";

export function ProjectInputsCard() {
  const { project, info, updateInfo } = useProject();
  const qc = useQueryClient();
  const [root, setRoot] = useState<string | null>(null);
  const [assets, setAssets] = useState<string | null>(null);
  const [pickerError, setPickerError] = useState<string | null>(null);
  const relink = useMutation({
    mutationFn: () => call("project_relink", {
      projectId: project!.project_id, root: root ?? info!.destinations.picked_root,
      assetsDir: assets ?? info!.destinations.assets_dir ?? "", savePath: null, rosterSave: null,
    }),
    onSuccess: r => {
      updateInfo(r.project, r.discovery, project!.project_id);
      setRoot(null); setAssets(null); void qc.invalidateQueries();
    },
  });
  if (!project || !info?.destinations) return null;
  const browse = async (set: (v: string) => void) => {
    try {
      setPickerError(null);
      const path = await openDialog({ directory: true, multiple: false, title: "Choose a game source folder" });
      if (typeof path === "string") set(path);
    } catch (error) { setPickerError(String(error)); }
  };
  const field = (label: string, value: string, set: (v: string) => void) => <label className="block text-xs text-slate-300">
    {label}<div className="mt-1 flex gap-2">
      <input value={value} onChange={e => set(e.target.value)} disabled={relink.isPending}
        className="min-w-0 flex-1 rounded border border-slate-700 bg-slate-950 px-3 py-2 text-xs" />
      <Button type="button" variant="secondary" disabled={relink.isPending} onClick={() => void browse(set)}>Browse…</Button>
    </div>
  </label>;
  return <details className="mt-4 border-t border-slate-800 pt-3">
    <summary className="cursor-pointer text-sm text-slate-300">Game source folders</summary>
    <p className="my-3 text-xs text-slate-400">Studio uses these folders to find source files. Mod Manager chooses where your built mod is installed.</p>
    <div className="space-y-3">
      {field("Game source folder", root ?? info.destinations.picked_root, setRoot)}
      {field("Extracted assets source folder", assets ?? info.destinations.assets_dir ?? "", setAssets)}
      <Button type="button" variant="secondary" disabled={relink.isPending || (root == null && assets == null)} onClick={() => relink.mutate()}>{relink.isPending ? "Updating…" : "Update sources"}</Button>
      {(relink.isError || pickerError) && <p role="alert" className="text-xs text-rose-300">{String(relink.error ?? pickerError)}</p>}
      {relink.data?.warnings.map((warning, i) => <p key={i} className="text-xs text-amber-300">{warning}</p>)}
    </div>
  </details>;
}
