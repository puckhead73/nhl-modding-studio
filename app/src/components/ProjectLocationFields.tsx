import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { useSettings } from "../lib/settings";
import { organizedProjectLocation } from "../lib/projectLocation";
import { Button } from "./primitives";

const inputClass = "w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1.5 text-sm text-slate-100";

/** One persisted storage preference shared by onboarding and Settings. */
export function ProjectsStorageField() {
  const { settings, setProjectsRoot } = useSettings();
  async function browse() {
    const picked = await openDialog({ directory: true, multiple: false, title: "Choose projects storage folder", defaultPath: settings.projectsRoot || undefined });
    if (typeof picked === "string") setProjectsRoot(picked);
  }
  return (
    <div>
      <label className="block text-xs text-slate-300">
        Projects storage folder
        <div className="mt-1 flex gap-2">
          <input aria-label="Projects storage folder" className={inputClass} value={settings.projectsRoot} onChange={(e) => setProjectsRoot(e.target.value)} placeholder="D:\Studio Projects" />
          <Button variant="secondary" onClick={() => void browse()}>Browse…</Button>
        </div>
      </label>
      <p className="mt-1 text-xs text-slate-500">Saved for all games. Changing this location affects new projects; existing projects stay where they are.</p>
    </div>
  );
}

export function ProjectLocationFields({ name, onNameChange, platform, game, folder, onFolderChange }: { name: string; onNameChange: (name: string) => void; platform?: string | null; game?: string | null; folder?: string; onFolderChange?: (folder: string) => void }) {
  const { settings } = useSettings();
  const location = organizedProjectLocation(settings.projectsRoot, name, platform, game);
  return (
    <div className="space-y-3">

      <label className="block text-xs text-slate-300">
        Project name
        <input aria-label="Project name" className={inputClass + " mt-1"} value={name} onChange={(e) => onNameChange(e.target.value)} placeholder="All Time Legends" />
      </label>
      <div className="block text-xs text-slate-300">
        Project folder
        <div className="flex gap-2">
          <input aria-label="Project folder" className={inputClass + " mt-1 font-mono"} value={folder || location.path} readOnly />
          {onFolderChange && <Button variant="secondary" onClick={async () => {
            const picked = await openDialog({ directory: true, multiple: false, title: "Choose project folder", defaultPath: folder || location.path || undefined });
            if (typeof picked === "string") onFolderChange(picked);
          }}>Browse…</Button>}
        </div>
      </div>
      {name.length > 0 && location.error && <p className="text-xs text-amber-300" role="status">{location.error}</p>}
    </div>
  );
}
