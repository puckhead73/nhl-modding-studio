"""Regenerate every file in ../output from the committed anchors. Read-only with respect to the
anchors; only writes into research/nhl07-roster-schema/output. Run from the repository root:
    python research/nhl07-roster-schema/scripts/run_all.py
"""
import subprocess, sys, os, pathlib

root = pathlib.Path(__file__).resolve().parents[3]
os.chdir(root)
S = "research/nhl07-roster-schema/scripts"
O = pathlib.Path("research/nhl07-roster-schema/output")
O.mkdir(exist_ok=True)
G = "anchors/xbox360/nhl07/databases/NHL07NG.db"
M = "anchors/xbox360/nhl07/databases/NHL07NG-meta.xml"
R = "anchors/xbox360/nhl07/savefiles/ROSTER1/ROSTER1"
D = "anchors/xbox360/nhl07/savefiles/DYNASTY1/DYNASTY1"
R8 = "anchors/xbox360/nhl08/savefiles/ROSTER1 20260718064658/ROSTER1 20260718064658"
G8 = "anchors/xbox360/nhl08/databases/nhlng.db"
M8 = "anchors/xbox360/nhl08/databases/nhlng-meta.xml"
R9 = "anchors/xbox360/nhl09/savefiles/ROSTER1 20260717230804/ROSTER1 20260717230804"


def run(out, *cmds):
    with open(O / out, "w", encoding="utf-8") as f:
        for c in cmds:
            r = subprocess.run([sys.executable] + list(c), capture_output=True, text=True, encoding="utf-8", errors="replace")
            f.write("$ python " + " ".join(c) + "\n" + r.stdout + (r.stderr or "") + "\n")
    print("wrote", out)


rd = f"{S}/record_dump.py"
run("scan_and_tables.txt", [f"{S}/tdb_scan.py", R, "--scan"], [f"{S}/tdb_scan.py", R, "0x1c"], [f"{S}/tdb_scan.py", G, "--scan"],
    [f"{S}/tdb_scan.py", G], [f"{S}/tdb_scan.py", R8, "--scan"], [f"{S}/tdb_scan.py", R9, "--scan"], [f"{S}/tdb_scan.py", R9, "0x1c"],
    [f"{S}/tdb_scan.py", D, "--scan"])
run("container_checks_nhl07.txt", [f"{S}/container_checks.py", R])
run("schema_compare_save_vs_game.txt", [f"{S}/schema_compare.py", R, "0x1c", G, "0"])
run("fieldset_match_save_vs_game.txt", [f"{S}/fieldset_match.py", R, "0x1c", G, "0", M])
run("difflist_map_nhl07.txt", [f"{S}/difflist_map.py", R, "0x1c", G, M])
run("difflist_map_nhl08.txt", [f"{S}/difflist_map.py", R8, "0x1c", G8, M8])
run("difflist_map_nhl07_dynasty.txt", [f"{S}/difflist_map.py", D, "0x1c", G, M])
run("update_table_match_nhl07.txt", [f"{S}/update_table_match.py", R, "0x1c", G, M])
run("update_table_match_nhl08.txt", [f"{S}/update_table_match.py", R8, "0x1c", G8, M8])
run("roster_group_meta_nhl07.txt", [f"{S}/roster_group_meta.py", G, M, R, "0x1c"])
run("key_graph_nhl07.txt", [f"{S}/key_graph.py", G, M])
run("crosstitle_ids.txt", [f"{S}/crosstitle_ids.py"])
run("save_directory_order_nhl07.txt", [f"{S}/save_directory_order.py"])
run("exhibition_vs_stock_nhl07.txt", [f"{S}/exhibition_vs_stock.py"])
run("record_trace_nhl07.txt",
    [rd, G, "0", M, "cPbu", "0", "4", "--only", "game_id,firstname,lastname,team,jersey,position,contractdollars,yeardrafted"],
    [rd, G, "0", M, "aUhv", "0", "4", "--only", "game_id,firstname,lastname,team,jersey,position"],
    [rd, G, "0", M, "caBZ", "0", "6"], [rd, G, "0", M, "caBZ", "198", "4"], [rd, G, "0", M, "caBZ", "2248", "5"], [rd, G, "0", M, "caBZ", "2786", "4"],
    [rd, G, "0", M, "ulGe", "0", "4", "--only", "playerindex,team,rosterstatus,captain"],
    [rd, G, "0", M, "ulGe", "2512", "4", "--only", "playerindex,team,rosterstatus"],
    [rd, G, "0", M, "cPbu", "2248", "2", "--only", "game_id,firstname,lastname,team"],
    [rd, G, "0", M, "ttOk", "0", "6"], [rd, G, "0", M, "ttOk", "89", "4"],
    [rd, G, "0", M, "UzqH", "0", "1", "--only", "index,fullname"], [rd, G, "0", M, "UzqH", "64", "1", "--only", "index,fullname"],
    [rd, G, "0", M, "UzqH", "89", "1", "--only", "index,fullname"],
    [rd, G, "0", M, "QEoV", "0", "4"], [rd, G, "0", M, "bZvB", "0", "1"], [rd, G, "0", M, "OEtS", "0", "3"], [rd, G, "0", M, "oYeE", "0", "2"],
    [rd, G, "0", M, "EhOn", "0", "3", "--only", "game_id,firstname,lastname,team"], [rd, G, "0", M, "MqdK", "0", "1"])
run("dynasty_difflist_evidence_nhl07.txt", [f"{S}/tdb_scan.py", D, "0x1c"],
    [rd, D, "0x1c", M, "fcdR", "0", "3", "--as", "MMxP", "--only", "game_id,firstname,lastname,team,jersey,position"],
    [rd, D, "0x1c", M, "Pwkd", "0", "3", "--as", "rgQY", "--only", "playerindex,team,rosterstatus"],
    [rd, D, "0x1c", M, "ioPQ", "0", "3", "--as", "kGzd"], [rd, D, "0x1c", M, "oiBf", "0", "3", "--as", "kGzd"])
