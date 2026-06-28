import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import styles from "./Dashboard.module.css";

export function Dashboard() {
  const { project } = useProject();
  const projectId = project!.project_id;

  const report = useQuery({
    queryKey: ["validation", projectId],
    queryFn: () => call("validate_project", { projectId }),
  });

  return (
    <div className={styles.wrap}>
      <h2>Dashboard</h2>

      <section className={styles.stats}>
        <Stat label="Platform" value={project!.platform} />
        <Stat label="Role" value={project!.role} />
        <Stat label="Source" value={project!.source} />
        <Stat label="Tables" value={String(project!.table_count)} />
        <Stat label="Size" value={`${project!.db_size.toLocaleString()} B`} />
        <Stat label="Header CRC" value={project!.header_crc_ok ? "OK" : "MISMATCH"} />
        <Stat label="Unsaved" value={project!.dirty ? "yes" : "no"} />
      </section>

      <section className={styles.card}>
        <h3>Validation</h3>
        {report.isLoading && <p className={styles.muted}>Running audit…</p>}
        {report.isError && <p className={styles.err}>{String(report.error)}</p>}
        {report.data && (
          <>
            <p>
              <span className={report.data.clean ? styles.ok : styles.bad}>
                {report.data.clean ? "Clean — safe to write" : "Errors present"}
              </span>{" "}
              · {report.data.error_count} errors · {report.data.warning_count} warnings ·{" "}
              {report.data.info_count} info
            </p>
            <div className={styles.findings}>
              {report.data.findings.slice(0, 200).map((f, i) => (
                <div key={i} className={styles.finding}>
                  <span className={styles[f.severity] ?? styles.muted}>{f.severity}</span>
                  <span className={styles.pred}>{f.predicate}</span>
                  <span>{f.message}</span>
                </div>
              ))}
              {report.data.findings.length > 200 && (
                <p className={styles.muted}>…and {report.data.findings.length - 200} more.</p>
              )}
              {report.data.findings.length === 0 && (
                <p className={styles.muted}>No findings.</p>
              )}
            </div>
          </>
        )}
      </section>
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className={styles.stat}>
      <span className={styles.statLabel}>{label}</span>
      <span className={styles.statValue}>{value}</span>
    </div>
  );
}
