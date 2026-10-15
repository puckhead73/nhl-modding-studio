"""Regenerate every file in ../output from the committed anchors. Read-only with respect to the
anchors; writes only into research/nhl08-roster-schema/output. Run from the repository root:
    python research/nhl08-roster-schema/scripts/run_all.py
"""
import hashlib
import os
import pathlib
import shutil
import subprocess
import sys

root = pathlib.Path(__file__).resolve().parents[3]
os.chdir(root)
S = "research/nhl08-roster-schema/scripts"
O = pathlib.Path("research/nhl08-roster-schema/output")
O.mkdir(exist_ok=True)
G = "anchors/xbox360/nhl08/databases/nhlng.db"
M = "anchors/xbox360/nhl08/databases/nhlng-meta.xml"
R = "anchors/xbox360/nhl08/savefiles/ROSTER1 20260718064658/ROSTER1 20260718064658"
D = "anchors/xbox360/nhl08/savefiles/DYNASTY1 20260718064632/DYNASTY1 20260718064632"
J = "anchors/xbox360/nhl08/game-files.json"
G7 = "anchors/xbox360/nhl07/databases/NHL07NG.db"
M7 = "anchors/xbox360/nhl07/databases/NHL07NG-meta.xml"
R7 = "anchors/xbox360/nhl07/savefiles/ROSTER1/ROSTER1"


def run(out, *cmds):
    with open(O / out, "w", encoding="utf-8") as f:
        for c in cmds:
            r = subprocess.run([sys.executable] + list(c), capture_output=True, text=True, encoding="utf-8", errors="replace")
            f.write("$ python " + " ".join('"%s"' % a if " " in a else a for a in c) + "\n" + r.stdout + (r.stderr or "") + "\n")
    print("wrote", out)


with open(O / "input_hashes.txt", "w", encoding="utf-8") as f:
    for pth in (G, M, R, D, J, G7, M7, R7):
        data = open(pth, "rb").read()
        f.write("%s  %10d  %s\n" % (hashlib.sha256(data).hexdigest(), len(data), pth))
print("wrote input_hashes.txt")

rd = f"{S}/record_dump.py"
run("scan_and_tables.txt", [f"{S}/tdb_scan.py", R, "--scan"], [f"{S}/tdb_scan.py", R, "0x1c"], [f"{S}/tdb_scan.py", G, "--scan"],
    [f"{S}/tdb_scan.py", G], [f"{S}/tdb_scan.py", D, "--scan"], [f"{S}/tdb_scan.py", D, "0x8d9bc"])
run("save_fields.txt", [f"{S}/tdb_scan.py", R, "0x1c", "--fields"])
run("container_checks_roster.txt", [f"{S}/container_checks.py", R, G, M, "--group", "rosters"])
run("container_checks_dynasty.txt", [f"{S}/container_checks.py", D, G, M, "--group", "dynasty"])
run("container_checks_gamedb.txt", [f"{S}/container_checks.py", G, G, M])
run("difflist_map_roster.txt", [f"{S}/difflist_map.py", R, G, M, "--group", "rosters"])
run("difflist_map_nhl07_roster_declared.txt", [f"{S}/difflist_map.py", R7, G7, M7, "--group", "rosters"])
run("roster_group_meta.txt", [f"{S}/roster_group_meta.py", G, M, R])
run("update_semantics.txt", [f"{S}/update_semantics.py", R, G, M])
run("key_graph.txt", [f"{S}/key_graph.py", G, M])
run("exhibition_vs_stock.txt", [f"{S}/exhibition_vs_stock.py", G, M])
run("save_directory_order.txt", [f"{S}/save_directory_order.py", R, G, M])
run("crosstitle_compare.txt", [f"{S}/crosstitle_compare.py"])
run("dynasty_evidence.txt", [f"{S}/dynasty_evidence.py", D, G, M])
bio = "game_id,firstname,lastname,team,proteam,jersey,position,yeardrafted,contractdollars,headid,artid"
run("record_trace.txt",
    [rd, G, "0", M, "cPbu", "0", "4", "--only", bio],
    [rd, G, "0", M, "cPbu", "2958", "4", "--only", bio],
    [rd, G, "0", M, "caBZ", "0", "5"], [rd, G, "0", M, "caBZ", "2960", "4"], [rd, G, "0", M, "caBZ", "3253", "4"],
    [rd, G, "0", M, "ulGe", "0", "4", "--only", "key,playerindex,team,rosterstatus,captain,jersey,l1c,l1lw,l1rw,g1"],
    [rd, G, "0", M, "ulGe", "2521", "4", "--only", "key,playerindex,team,rosterstatus,jersey"],
    [rd, G, "0", M, "QEoV", "0", "5"],
    [rd, G, "0", M, "ttOk", "0", "3", "--only", "index,fullname,abbrname,league,conference,division,active,table,original"],
    [rd, G, "0", M, "ttOk", "30", "2", "--only", "index,fullname,abbrname,league,conference,division,active,table,original"],
    [rd, G, "0", M, "ttOk", "135", "3", "--only", "index,fullname,abbrname,league,conference,division,active,table,original"],
    [rd, G, "0", M, "oYeE", "0", "2"], [rd, G, "0", M, "OEtS", "0", "2", "--only", "index,arenaname,cityname,maxattendance"],
    [rd, G, "0", M, "yvSd", "0", "2", "--only", "game_id,p_speed,p_passing,p_potential,p_style"],
    [rd, G, "0", M, "yuHm", "0", "2", "--only", "game_id,g_gsl,g_gsh,g_style,g_potential"],
    [rd, G, "0", M, "ajmx", "0", "2"], [rd, G, "0", M, "lVMf", "0", "1", "--only", "game_id,stick,pads,blocker,trapper,masktype,keepcustomiftraded"],
    [rd, G, "0", M, "nAUy", "0", "3", "--only", "key,game_id,firstname,lastname,team,pads,blocker,trapper"],
    [rd, G, "0", M, "sozv", "0", "3"], [rd, G, "0", M, "vbHh", "0", "2", "--only", "index,team,era,brand,variant,equipment"],
    [rd, G, "0", M, "Vzjq", "0", "2"], [rd, G, "0", M, "Roup", "0", "2"], [rd, G, "0", M, "NDEo", "0", "2", "--only", "index,team,brand,variant,equipment"],
    [rd, G, "0", M, "WdaS", "0", "1"], [rd, G, "0", M, "LScw", "0", "2"], [rd, G, "0", M, "puAP", "0", "2"])
# one player traced end to end by stable key: a stock bio -> slot -> roster row -> team, AI, equipment
# Sidney Crosby: bio game_id 3109 -> three exhibitionplayers slots (2947/2948/2949) -> three roster rows
run("record_trace_player.txt",
    [rd, G, "0", M, "cPbu", "0", "1", "--where", "lastname=Crosby", "--only", bio],
    [rd, G, "0", M, "caBZ", "0", "4", "--where", "playerid=3109"],
    [rd, G, "0", M, "ulGe", "0", "2", "--where", "playerindex=2947", "--only", "key,playerindex,team,rosterstatus,jersey,captain,l1c,pp1c"],
    [rd, G, "0", M, "ulGe", "0", "2", "--where", "playerindex=2948", "--only", "key,playerindex,team,rosterstatus,jersey,captain,l1c,pp1c"],
    [rd, G, "0", M, "ulGe", "0", "2", "--where", "playerindex=2949", "--only", "key,playerindex,team,rosterstatus,jersey,captain,l1c,pp1c"],
    [rd, G, "0", M, "yvSd", "0", "1", "--where", "game_id=3109", "--only", "game_id,p_speed,p_passing,p_potential,p_offawareness"],
    [rd, G, "0", M, "ajmx", "0", "1", "--where", "game_id=3109"],
    [rd, G, "0", M, "ttOk", "0", "1", "--where", "index=23", "--only", "index,fullname,abbrname,league"],
    [rd, G, "0", M, "ttOk", "0", "1", "--where", "index=30", "--only", "index,fullname,abbrname,league"],
    [rd, G, "0", M, "ttOk", "0", "1", "--where", "index=89", "--only", "index,fullname,abbrname,league"],
    # a goalie: bio position 4, goalie AI + goalie equipment + a stockgoalieequipment preset row
    [rd, G, "0", M, "cPbu", "0", "1", "--where", "game_id=150", "--only", bio],
    [rd, G, "0", M, "yuHm", "0", "1", "--where", "game_id=150", "--only", "game_id,g_gsl,g_gsh,g_style,g_potential"],
    [rd, G, "0", M, "lVMf", "0", "1", "--where", "game_id=150", "--only", "game_id,stick,pads,blocker,trapper,masktype,keepcustomiftraded"],
    [rd, G, "0", M, "nAUy", "0", "1", "--where", "game_id=150", "--only", "key,game_id,firstname,lastname,team,pads,blocker,trapper"],
    # a free agent slot and a custom (table=1) slot
    [rd, G, "0", M, "caBZ", "0", "1", "--where", "index=152"],
    [rd, G, "0", M, "cPbu", "0", "1", "--where", "game_id=5000", "--only", bio],
    [rd, G, "0", M, "yvSd", "0", "1", "--where", "game_id=5000", "--only", "game_id,p_speed,p_passing,p_potential"],
    [rd, G, "0", M, "Vzjq", "0", "1", "--where", "playerid=5000"])
shutil.rmtree(pathlib.Path(S) / "__pycache__", ignore_errors=True)
print("done")
