import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useMutation } from "@tanstack/react-query";
import { call, type Platform, type Role } from "../lib/client";
import { useProject } from "../lib/project";
import { Button, ErrorBanner } from "../components/primitives";
import headerLogo from "../assets/ui/NHL Modding Studio Header.png";

// Dev convenience: prefill with the committed NHL Legacy anchor.
const DEFAULTS = {
  dbPath: "anchors/ps3/nhllegacy/databases/nhlng.db",
  metaXml: "anchors/ps3/nhllegacy/databases/nhlng-meta.xml",
  defsRoot: "defs",
  platform: "ps3" as Platform,
  role: "nhlng" as Role,
};

export function OpenProject() {
  const { open } = useProject();
  const navigate = useNavigate();
  const [form, setForm] = useState(DEFAULTS);

  const load = useMutation({
    mutationFn: () =>
      call("load_project", {
        dbPath: form.dbPath,
        platform: form.platform,
        role: form.role,
        metaXml: form.metaXml,
        defsRoot: form.defsRoot,
      }),
    onSuccess: (summary) => {
      open(summary, form.dbPath);
      navigate("/app/dashboard");
    },
  });

  const set = (k: keyof typeof form) => (e: { target: { value: string } }) =>
    setForm((f) => ({ ...f, [k]: e.target.value }));

  const input =
    "w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1.5 text-sm text-slate-100 placeholder-slate-600 focus:border-sky-600 focus:outline-none";
  const labelCls = "mb-1 block text-xs font-medium text-slate-300";

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/80 p-6 backdrop-blur-sm">
      <div className="w-full max-w-2xl rounded-lg border border-slate-800 bg-slate-900 p-6 shadow-2xl">
        <img src={headerLogo} alt="NHL Modding Studio" className="mx-auto mb-5 h-12 w-auto object-contain" />

        <div className="space-y-3">
          <div>
            <label className={labelCls}>Database / save path</label>
            <input className={input} value={form.dbPath} onChange={set("dbPath")} />
          </div>
          <div>
            <label className={labelCls}>Metadata XML</label>
            <input className={input} value={form.metaXml} onChange={set("metaXml")} />
          </div>
          <div>
            <label className={labelCls}>Defs root</label>
            <input className={input} value={form.defsRoot} onChange={set("defsRoot")} />
          </div>
          <div className="flex gap-3">
            <div className="flex-1">
              <label className={labelCls}>Platform</label>
              <select className={input} value={form.platform} onChange={set("platform")}>
                <option value="ps3">PS3</option>
                <option value="xbox">Xbox 360</option>
                <option value="ps4">PS4</option>
                <option value="recomp">Legacy Recomp</option>
              </select>
            </div>
            <div className="flex-1">
              <label className={labelCls}>Role</label>
              <select className={input} value={form.role} onChange={set("role")}>
                <option value="nhlng">nhlng.db</option>
                <option value="roster">Roster save</option>
                <option value="localization">Localization</option>
              </select>
            </div>
          </div>
        </div>

        <div className="mt-5 flex justify-end">
          <Button onClick={() => load.mutate()} disabled={load.isPending}>
            {load.isPending ? "Loading…" : "Load project"}
          </Button>
        </div>

        {load.isError && (
          <div className="mt-4">
            <ErrorBanner title="Couldn't load" message={String(load.error)} />
          </div>
        )}
      </div>
    </div>
  );
}
