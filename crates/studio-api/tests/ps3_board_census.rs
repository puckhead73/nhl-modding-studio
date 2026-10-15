//! Receipt 67 — PS3 parity for the C37 board writer's admission (research 2026-09-28 handoff 10 §8.3): Studio's
//! `tdb_rx2::venue_board::admit` over every PS3 venue package of the six titles, read only (loose files on NHL 14 / 15 / Legacy,
//! archive members unwrapped in memory on NHL 11 / 12 / 13), must reproduce research's census edge counts; discovery finds no DXT5
//! board; NHL 11's `adbd_41` is refused by its slot word; on the pair titles every loose package equals its archive member.
use std::path::Path;

const ROOT: &str = r"X:\Games\Emulation\Roms\Sony Playstation 3";

fn packages(usrdir: &Path, archive: &str, loose: bool) -> Vec<(String, Vec<u8>)> {
    let arc = usrdir.join(archive);
    let entries = studio_domain::ps4_vault::eb_index(&arc).unwrap();
    let mut out = Vec::new();
    for e in entries.iter().filter(|e| {
        let p = e.path.to_ascii_lowercase().replace('\\', "/");
        p.starts_with("rendering/stadium/stadium_") && p.ends_with(".rpsgl")
    }) {
        let member = studio_domain::ps4_vault::eb_read(&arc, e).unwrap();
        if loose {
            let lp = usrdir.join(e.path.replace('/', "\\"));
            let l = std::fs::read(&lp).unwrap_or_else(|_| panic!("{}: loose copy missing", lp.display()));
            assert!(l == member, "{}: the pair diverges", e.path);
        }
        out.push((e.path.clone(), member));
    }
    out
}

#[test]
#[ignore = "reads every PS3 venue package of NHL 11–15 / Legacy under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn ps3_admission_reproduces_the_research_census() {
    let titles: [(&str, &str, bool, usize); 6] = [
        (r"NHL 11\PS3_GAME\USRDIR", "stadium.big", false, 136),
        (r"NHL 12\[BLUS 30791] NHL® 12\PS3_GAME\USRDIR", "cacherender.big", false, 226),
        (r"NHL 13\PS3_GAME\USRDIR", "cacherender.big", false, 232),
        (r"NHL 14\PS3_GAME\USRDIR", "cacherender.big", true, 243),
        (r"NHL 15\PS3_GAME\USRDIR", "cacherender.big", true, 243),
        (r"NHL Legacy - Vanilla\PS3_GAME\USRDIR", "cacherender.big", true, 243),
    ];
    for (dir, archive, pair, want) in titles {
        let usrdir = Path::new(ROOT).join(dir);
        assert!(usrdir.join(archive).is_file(), "{}: missing (fails, never skips)", usrdir.display());
        let pkgs = packages(&usrdir, archive, pair);
        let (mut edges, mut refusals, mut dxt5) = (0usize, Vec::new(), 0usize);
        let mut formats = std::collections::BTreeMap::<String, usize>::new();
        for (path, b) in &pkgs {
            let (e, r) = tdb_rx2::venue_board::admit(b).unwrap();
            edges += e.len();
            for x in &e {
                *formats.entry(format!("{:?}", x.format)).or_default() += 1;
            }
            refusals.extend(r.into_iter().map(|(m, why)| format!("{path}: {m}: {why}")));
            dxt5 += tdb_rx2::venue_board::discover(b).unwrap().iter().filter(|(_, f)| f == "DXT5").count();
        }
        assert_eq!(edges, want, "{dir}: admitted edges vs research R142b; refusals {refusals:?}");
        assert_eq!(dxt5, 0, "{dir}: no DXT5 board material");
        if archive == "stadium.big" {
            assert!(refusals.len() == 1 && refusals[0].contains("adbd_41") && refusals[0].contains("slot word"), "{refusals:?}");
        } else {
            assert!(refusals.is_empty(), "{dir}: {refusals:?}");
        }
        eprintln!("{dir}: {} packages, {edges} admitted edges {formats:?}{}", pkgs.len(), if pair { ", every loose package = its member" } else { "" });
    }
}
