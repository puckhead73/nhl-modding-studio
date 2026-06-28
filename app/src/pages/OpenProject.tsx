import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useMutation } from "@tanstack/react-query";
import { call, type Platform, type Role } from "../lib/client";
import { useProject } from "../lib/project";
import styles from "./OpenProject.module.css";

// Dev convenience: prefill with the committed NHL Legacy anchor so the slice is one click to load.
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
      navigate("/app/players");
    },
  });

  const set = (k: keyof typeof form) => (e: { target: { value: string } }) =>
    setForm((f) => ({ ...f, [k]: e.target.value }));

  return (
    <div className={styles.wrap}>
      <h1>NHL Modding Studio</h1>
      <p className={styles.sub}>Open a TDB database or save container.</p>

      <label className={styles.field}>
        <span>Database / save path</span>
        <input value={form.dbPath} onChange={set("dbPath")} />
      </label>
      <label className={styles.field}>
        <span>Metadata XML</span>
        <input value={form.metaXml} onChange={set("metaXml")} />
      </label>
      <label className={styles.field}>
        <span>Defs root</span>
        <input value={form.defsRoot} onChange={set("defsRoot")} />
      </label>
      <div className={styles.row}>
        <label className={styles.field}>
          <span>Platform</span>
          <select value={form.platform} onChange={set("platform")}>
            <option value="ps3">PS3</option>
            <option value="xbox">Xbox 360</option>
            <option value="ps4">PS4</option>
            <option value="recomp">Legacy Recomp</option>
          </select>
        </label>
        <label className={styles.field}>
          <span>Role</span>
          <select value={form.role} onChange={set("role")}>
            <option value="nhlng">nhlng.db</option>
            <option value="roster">Roster save</option>
            <option value="localization">Localization</option>
          </select>
        </label>
      </div>

      <button className={styles.load} onClick={() => load.mutate()} disabled={load.isPending}>
        {load.isPending ? "Loading…" : "Load project"}
      </button>

      {load.isError && <p className={styles.error}>{String(load.error)}</p>}
    </div>
  );
}
