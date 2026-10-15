//! Known name corrections — a source-verified / high-confidence override table for FHM source-data
//! corruption that both ingest paths (`tdb-fhm-build` CSV ingest and `tdb-fhm-convert` prior-DB
//! conversion) apply identically at person-creation time.
//!
//! Context: the newest FHM season generation (2025-2026) mangled ~191 names carrying diacritics into
//! a form with literal `?` characters (each `?` marks one dropped accented letter, e.g. `"Sundstr?m"`
//! for `"Sundström"`). Both our CSV export and the separately exported prior-app `historical.db`
//! inherited this corruption identically — it originates at a single point (the 2025-2026 generation),
//! so for most affected people there is no clean copy in any source we have.
//!
//! Two tiers, both applied by [`known_name_fixup`]:
//! * [`VERIFIED_FIXUPS`] — the correct name was found in an independent clean record (an earlier
//!   season's export, cross-checked in both the CSV corpus and the prior DB).
//! * [`RECONSTRUCTED_FIXUPS`] — the accented letter is **linguistically unambiguous** from the base
//!   letters + nationality (e.g. Swedish `-str?m`→`-ström`, Finnish `M?kel?`→`Mäkelä`, Czech
//!   `Ond?ej`→`Ondřej`, French `?tienne`→`Étienne`). High-confidence but not source-confirmed. The
//!   remaining lower-confidence guesses live in `docs/reference/fhm-name-candidates.md` (not applied).

/// `(slug, first_name, last_name)` — the name was verified against an independent clean record.
const VERIFIED_FIXUPS: &[(&str, &str, &str)] = &[
    // Verified: 2019-2020..2024-2025 seasons (both the CSV corpus and the prior app's historical.db)
    // consistently show "Jaroslav Chmelar" (no diacritic); only the newest 2025-2026 season
    // introduced "Chmelar?".
    ("chmelar_jaroslav_20_07_2003", "Jaroslav", "Chmelar"),
];

/// `(slug, first_name, last_name)` — high-confidence linguistic reconstruction (see module docs).
/// Generated + reviewed from `docs/reference/fhm-name-candidates.md` (HIGH tier).
const RECONSTRUCTED_FIXUPS: &[(&str, &str, &str)] = &[
    ("fengos_timothe_27_02_2008", "Timothée", "Fengos"),
    ("roy_charlestienne_26_10_2006", "Charles-Étienne", "Roy"),
    ("vaillancourt_jrmy_25_07_2007", "Jérémy", "Vaillancourt"),
    ("mller_tomas_05_04_2001", "Tomas", "Müller"),
    ("mertl_vojtch_29_03_2010", "Vojtěch", "Mertl"),
    ("nabelek_ondej_23_11_2010", "Ondřej", "Nabelek"),
    ("novotn_antonn_14_07_2011", "Antonín", "Novotný"),
    ("sedlek_jaroslav_23_02_2011", "Jaroslav", "Sedláček"),
    ("stergaard_mads_01_01_2004", "Mads", "Østergaard"),
    ("srensen_phillip_01_01_2004", "Phillip", "Sørensen"),
    ("bostrm_joni_01_01_2007", "Joni", "Boström"),
    ("jrvinen_andreas_19_06_2010", "Andreas", "Järvinen"),
    ("khknen_jimi_01_01_2010", "Jimi", "Kähkönen"),
    ("khknen_weeti_01_01_2009", "Weeti", "Kähkönen"),
    ("lf_eetu_23_05_2000", "Eetu", "Löf"),
    ("leskel_neo_08_06_2008", "Neo", "Leskelä"),
    ("mkel_jere_19_09_2000", "Jere", "Mäkelä"),
    ("mkel_martti_18_04_2005", "Martti", "Mäkelä"),
    ("martinmki_ville_01_01_2009", "Ville", "Martinmäki"),
    ("mastomki_maisa_07_03_2005", "Maisa", "Mastomäki"),
    ("myyrylinen_samuel_01_01_2010", "Samuel", "Myyryläinen"),
    ("niemel_max_27_04_2004", "Max", "Niemelä"),
    ("riihimki_akseli_26_01_2010", "Akseli", "Riihimäki"),
    ("skkinen_leo_01_01_2008", "Leo", "Säkkinen"),
    ("siljamki_ossi_13_07_2007", "Ossi", "Siljamäki"),
    ("sjholm_topi_15_08_2010", "Topi", "Sjöholm"),
    ("sundstrm_kent_30_12_1995", "Kent", "Sundström"),
    ("sundstrm_conny_22_05_1997", "Conny", "Sundström"),
    ("vlj_jesse_13_12_2001", "Jesse", "Väljä"),
    ("ylimki_jyri_01_01_2002", "Jyri", "Ylimäki"),
    ("ahullo_matho_14_10_2010", "Mathéo", "Ahullo"),
    ("dalpan_lo_01_01_2009", "Léo", "Dal Pan"),
    ("eliard_raphal_10_04_2009", "Raphaël", "Eliard"),
    (
        "rossatmignot_timothe_17_09_2010",
        "Timothée",
        "Rossat-Mignot",
    ),
    ("bhler_jan_14_09_2009", "Jan", "Bühler"),
    ("dnkel_leon_12_02_2008", "Leon", "Dünkel"),
    ("grner_xaver_02_12_2009", "Xaver", "Grüner"),
    ("knig_alexander_06_04_2009", "Alexander", "König"),
    ("mller_maximilian_29_08_2009", "Maximilian", "Müller"),
    ("reu_paul_10_04_2011", "Paul", "Reuß"),
    ("schfer_maximilian_02_03_2011", "Maximilian", "Schäfer"),
    ("kovcs_zsombor_10_03_2008", "Zsombor", "Kovács"),
    ("szongoth_bendegerg_20_04_2008", "Bende Gergő", "Szongoth"),
    ("nilsgrd_lukakaatorp_24_09_2009", "Luka Kaatorp", "Nilsgård"),
    ("maral_martim_27_05_2008", "Martim", "Marçal"),
    ("bergstrm_casper_23_03_2009", "Casper", "Bergström"),
    ("bjrk_jack_29_04_2009", "Jack", "Björk"),
    ("bjrk_pelle_31_07_2009", "Pelle", "Björk"),
    ("bjrn_carladrian_01_04_2010", "Carl-Adrian", "Björn"),
    ("blombck_oskar_25_08_2009", "Oskar", "Blombäck"),
    ("borgstrm_alexander_07_09_2008", "Alexander", "Borgström"),
    ("carlsson_bjrn_28_02_2009", "Björn", "Carlsson"),
    (
        "dafgrdlarsson_charlie_22_03_2010",
        "Charlie",
        "Dafgård Larsson",
    ),
    ("eklf_simon_15_02_2009", "Simon", "Eklöf"),
    ("flisbck_johndiego_20_07_2009", "John-Diego", "Flisbäck"),
    ("hgardh_simon_27_09_2007", "Simon", "Högardh"),
    ("hglund_max_27_06_2008", "Max", "Höglund"),
    ("hglund_fabian_20_08_2009", "Fabian", "Höglund"),
    (
        "hagstrmgaljaard_elias_16_01_2008",
        "Elias",
        "Hagström Galjaard",
    ),
    ("hammarbck_per_08_12_2009", "Per", "Hammarbäck"),
    ("holmstrm_erik_06_03_2007", "Erik", "Holmström"),
    ("kck_albin_29_03_2008", "Albin", "Käck"),
    ("larsgrden_vincent_05_05_2010", "Vincent", "Larsgården"),
    ("lindstrm_vidar_09_02_2006", "Vidar", "Lindström"),
    ("navstrm_jonathan_12_12_2009", "Jonathan", "Navström"),
    ("sandstrm_vilmer_01_10_2008", "Vilmer", "Sandström"),
    ("sennstrm_leon_24_07_2008", "Leon", "Sennström"),
    ("sjblom_vilmer_02_03_2006", "Vilmer", "Sjöblom"),
    ("sjdin_erik_16_05_2008", "Erik", "Sjödin"),
    ("sjlander_fabian_03_11_2010", "Fabian", "Sjölander"),
    ("sjlin_tyler_17_09_2009", "Tyler", "Sjölin"),
    ("skyttgrdholm_hugo_21_08_2010", "Hugo", "Skyttgård Holm"),
    ("stlberg_anton_06_12_2006", "Anton", "Stålberg"),
    ("stlberg_oliver_06_12_2006", "Oliver", "Stålberg"),
    ("strm_gabriel_20_01_2010", "Gabriel", "Ström"),
    ("strmberg_noah_11_01_2012", "Noah", "Strömberg"),
    ("strmberg_alvin_22_02_2010", "Alvin", "Strömberg"),
    ("sundbck_viggo_24_03_2009", "Viggo", "Sundbäck"),
    ("wahnstrm_isak_03_04_2009", "Isak", "Wahnström"),
    (
        "tunakaya_hseyinkamil_24_06_2008",
        "Hüseyin Kamil",
        "Tuna Kaya",
    ),
    // --- MEDIUM tier (applied on request): plausible but not near-unambiguous. Notable
    // coin-flips: the two "Åberg" could be "Öberg"; "Poláček" could be "Polášek".
    ("fssl_lars_17_07_2006", "Lars", "Füssl"),
    ("novek_vojtch_26_03_2011", "Vojtěch", "Nováček"),
    ("polek_ondej_16_02_2011", "Ondřej", "Poláček"),
    ("hyvrinen_hugo_07_01_2004", "Hugo", "Hyvärinen"),
    ("vnttil_henry_06_11_1988", "Henry", "Vänttilä"),
    ("clavel_no_14_03_2010", "Noé", "Clavel"),
    ("hrmann_felix_29_08_2009", "Felix", "Hörmann"),
    ("frhlichs_tim_10_11_2007", "Tim", "Fröhlichs"),
    (
        "karadas_victorslettebrten_01_01_2009",
        "Victor Slettebråten",
        "Karadas",
    ),
    (
        "skrder_mariusrotefoss_01_01_2009",
        "Marius Rotefoss",
        "Skråder",
    ),
    (
        "smestad_viktorlangbrten_14_06_2009",
        "Viktor Langbråten",
        "Smestad",
    ),
    ("sabo_jrgen_17_06_2012", "Jürgen", "Sabo"),
    ("berg_ted_04_03_2009", "Ted", "Åberg"),
    ("berg_frans_11_03_2010", "Frans", "Åberg"),
    ("fjllman_eric_23_09_2009", "Eric", "Fjällman"),
    ("frojdn_felix_27_01_2010", "Felix", "Frojdén"),
    ("gllstrm_jonathan_01_11_2009", "Jonathan", "Gällström"),
    ("hlllillberg_malte_22_05_2009", "Malte", "Häll Lillberg"),
    ("hyln_ludvig_16_02_2009", "Ludvig", "Hylén"),
    ("lidn_melker_13_07_2010", "Melker", "Lidén"),
    ("lindn_elias_25_12_2008", "Elias", "Lindén"),
    ("nordnorn_simon_21_09_2010", "Simon", "Nord Norén"),
    ("nyrn_anton_02_01_2009", "Anton", "Nyrén"),
    ("olsn_lucas_12_03_2009", "Lucas", "Olsén"),
    ("rosn_ernst_21_05_2009", "Ernst", "Rosén"),
    ("wikhll_jacob_01_04_2009", "Jacob", "Wikhäll"),
    ("ylvn_emil_17_06_2009", "Emil", "Ylvén"),
    ("hrlimann_moris_20_03_2008", "Moris", "Hürlimann"),
    ("kppeli_sebastiano_07_08_2010", "Sebastiano", "Käppeli"),
    ("wlti_bastien_27_05_2009", "Bastien", "Wälti"),
    ("nsever_ferhatefe_05_07_2010", "Ferhat Efe", "Önsever"),
    ("yorat_yagizgktrk_09_07_2009", "Yagiz Göktürk", "Yorat"),
];

/// Look up a verified or high-confidence corrected name for `slug`, if one exists.
pub fn known_name_fixup(slug: &str) -> Option<(&'static str, &'static str)> {
    VERIFIED_FIXUPS
        .iter()
        .chain(RECONSTRUCTED_FIXUPS.iter())
        .find(|(s, _, _)| *s == slug)
        .map(|(_, f, l)| (*f, *l))
}
