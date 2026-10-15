//! Backend-dev-only player identification slug shared by all bridges into and
//! out of the EA NHL database. The format is
//! `lastname_firstname_dd_mm_yyyy`, lowercased, with diacritics
//! transliterated to ASCII and non-alphanumerics dropped.
//!
//! It is a *bridge* key, not an EA-native identifier — nothing in `nhlng.db`
//! stores it. Both directions of the FHM↔EA bridge derive it the same way: the
//! EA side composes it from `ubPc` (`HdeP`/`QbMR` names + the `qFnd`/`JKLp`/
//! `Kswi` birthdate encoding), and the FHM side re-derives it from the corpus's
//! name + real calendar DOB via [`historical_player_key`]. Deriving both in one
//! place is what lets an exported asset find its historical counterpart.
//!
//! The EA birthdate encoding this relies on (`qFnd + 1900`, `JKLp + 1`,
//! `Kswi + 1`) is the same one `studio-domain`'s `ea_semantic_display` applies.

/// Build the backend-dev-only player identification slug:
/// `lastname_firstname_dd_mm_yyyy`, lowercased, with diacritics
/// transliterated to ASCII and non-alphanumerics dropped.
///
/// Returns `None` when any of first/last name is empty after
/// transliteration, or when any of year/month/day is missing /
/// implausible. Birthdate components follow the on-disk encoding:
/// `qFnd + 1900` (year), `JKLp + 1` (month, 1..12), `Kswi + 1`
/// (day-of-month, 1..31).
pub fn dev_player_key(
    first_name: Option<&str>,
    last_name: Option<&str>,
    qfnd: Option<u64>,
    jklp: Option<u64>,
    kswi: Option<u64>,
) -> Option<String> {
    let last = slug_ascii(last_name?);
    let first = slug_ascii(first_name?);
    if last.is_empty() || first.is_empty() {
        return None;
    }
    let year = (qfnd? as i64) + 1900;
    let month = (jklp? as i64) + 1;
    let day = (kswi? as i64) + 1;
    if !(1900..=2100).contains(&year) || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(format!("{last}_{first}_{day:02}_{month:02}_{year:04}"))
}

/// Build a historical-player identity slug from a real calendar date.
///
/// Unlike [`dev_player_key`], this does not pass the year through EA's
/// `qFnd = year - 1900` storage encoding. Historical players born before
/// 1900 therefore retain their actual birth year in the stable identity.
pub fn historical_player_key(
    first_name: Option<&str>,
    last_name: Option<&str>,
    year: Option<i32>,
    month: Option<u8>,
    day: Option<u8>,
) -> Option<String> {
    let last = slug_ascii(last_name?);
    let first = slug_ascii(first_name?);
    if last.is_empty() || first.is_empty() {
        return None;
    }
    let year = year?;
    let month = month?;
    let day = day?;
    if !(1..=2100).contains(&year) || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(format!("{last}_{first}_{day:02}_{month:02}_{year:04}"))
}

/// Defensive validator for a player identity slug produced by
/// [`dev_player_key`] / [`historical_player_key`]. A well-formed slug is
/// exactly `last_first_dd_mm_yyyy`: five `_`-separated parts where the
/// two name tokens are non-empty ASCII-alphanumeric (the builders strip
/// every other character, so a real name token never contains `_`), and
/// the date tokens are a plausible 2/2/4-digit `dd`/`mm`/`yyyy`.
///
/// Both builders already guarantee this by construction, so for slugs
/// they produce this always returns `true`. The check exists so any
/// slug arriving from another source (a hand-edited bundle, a future
/// importer) that is degenerate — empty name parts, missing date — is
/// rejected up front and treated as "needs a manual target" instead of
/// being fed into the EA index where it could mis-resolve.
pub fn is_well_formed_player_slug(slug: &str) -> bool {
    let parts: Vec<&str> = slug.split('_').collect();
    if parts.len() != 5 {
        return false;
    }
    let name_ok = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric());
    if !name_ok(parts[0]) || !name_ok(parts[1]) {
        return false;
    }
    let digits_in = |s: &str, len: usize, lo: u32, hi: u32| -> bool {
        s.len() == len
            && s.chars().all(|c| c.is_ascii_digit())
            && s.parse::<u32>().is_ok_and(|v| (lo..=hi).contains(&v))
    };
    digits_in(parts[2], 2, 1, 31)
        && digits_in(parts[3], 2, 1, 12)
        && digits_in(parts[4], 4, 1, 2100)
}

/// Transliterate a name to lowercase ASCII alphanumeric only.
/// Diacritics decompose to their base letter; common ligatures /
/// Latin Extended letters map by hand (ß→ss, æ→ae, ø→oe, đ→d, ł→l,
/// þ→t, ð→d). Anything that remains non-alphanumeric is dropped, so
/// spaces, hyphens, apostrophes, periods all vanish ("St. Louis" →
/// "stlouis", "Pierre-Luc" → "pierreluc").
pub fn slug_ascii(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        let lower = ch.to_lowercase();
        for lc in lower {
            match transliterate_char(lc) {
                Some(s) => {
                    for c in s.chars() {
                        if c.is_ascii_alphanumeric() {
                            out.push(c);
                        }
                    }
                }
                None => {
                    if lc.is_ascii_alphanumeric() {
                        out.push(lc);
                    }
                }
            }
        }
    }
    out
}

/// Map one lowercased character to its ASCII equivalent. Returns
/// `None` when the character is already ASCII or has no sensible
/// mapping (caller drops it). The handled set covers the diacritics
/// and ligatures present in the NHL player corpus (Czech, Slovak,
/// Finnish, Swedish, Norwegian, Icelandic, French, German, Polish).
fn transliterate_char(c: char) -> Option<&'static str> {
    Some(match c {
        // a-family
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => "a",
        'æ' => "ae",
        // c-family
        'ç' | 'ć' | 'č' | 'ĉ' | 'ċ' => "c",
        // d-family
        'ď' | 'đ' | 'ð' => "d",
        // e-family
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => "e",
        // g/h
        'ĝ' | 'ğ' | 'ġ' | 'ģ' => "g",
        'ĥ' | 'ħ' => "h",
        // i-family
        'ì' | 'í' | 'î' | 'ï' | 'ī' | 'ĭ' | 'į' | 'ı' => "i",
        // j
        'ĵ' => "j",
        // k
        'ķ' => "k",
        // l-family
        'ĺ' | 'ļ' | 'ľ' | 'ŀ' | 'ł' => "l",
        // n-family
        'ñ' | 'ń' | 'ņ' | 'ň' | 'ŋ' => "n",
        // o-family
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ō' | 'ŏ' | 'ő' => "o",
        // ø/Ø — Norwegian/Danish o-slash. Conventional Latin-alphabet
        // transliteration is "oe" (Søren → soeren), so it groups with
        // the œ ligature instead of plain "o".
        'ø' | 'œ' => "oe",
        // r-family
        'ŕ' | 'ŗ' | 'ř' => "r",
        // s-family
        'ś' | 'ŝ' | 'ş' | 'š' => "s",
        'ß' => "ss",
        // t-family
        'ţ' | 'ť' | 'ŧ' | 'þ' => "t",
        // u-family
        'ù' | 'ú' | 'û' | 'ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' => "u",
        // w/y
        'ŵ' => "w",
        'ý' | 'ÿ' | 'ŷ' => "y",
        // z-family
        'ź' | 'ż' | 'ž' => "z",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(first: &str, last: &str, qfnd: u64, jklp: u64, kswi: u64) -> Option<String> {
        dev_player_key(Some(first), Some(last), Some(qfnd), Some(jklp), Some(kswi))
    }

    #[test]
    fn dev_player_key_basic_ascii() {
        // Sidney Crosby, 1987-08-07. qFnd=87, JKLp=7 (Aug-1), Kswi=6 (7-1).
        assert_eq!(
            key("Sidney", "Crosby", 87, 7, 6).as_deref(),
            Some("crosby_sidney_07_08_1987"),
        );
    }

    #[test]
    fn dev_player_key_strips_umlauts_and_accents() {
        assert_eq!(
            key("Teemu", "Selänne", 70, 6, 2).as_deref(),
            Some("selanne_teemu_03_07_1970"),
        );
        assert_eq!(
            key("Jaromír", "Jágr", 72, 1, 14).as_deref(),
            Some("jagr_jaromir_15_02_1972"),
        );
        assert_eq!(
            key("Éric", "Gélinas", 91, 4, 6).as_deref(),
            Some("gelinas_eric_07_05_1991"),
        );
    }

    #[test]
    fn dev_player_key_handles_ligatures_and_extras() {
        assert_eq!(
            key("Wieß", "Ørstad", 90, 0, 0).as_deref(),
            Some("oerstad_wiess_01_01_1990"),
        );
        assert_eq!(
            key("Mats", "Złoty", 85, 11, 30).as_deref(),
            Some("zloty_mats_31_12_1985"),
        );
        assert_eq!(
            key("François", "Cœur", 80, 0, 0).as_deref(),
            Some("coeur_francois_01_01_1980"),
        );
    }

    #[test]
    fn dev_player_key_drops_spaces_hyphens_periods() {
        assert_eq!(
            key("Pierre-Luc", "Dubois", 98, 5, 23).as_deref(),
            Some("dubois_pierreluc_24_06_1998"),
        );
        assert_eq!(
            key("Martin", "St. Louis", 75, 5, 17).as_deref(),
            Some("stlouis_martin_18_06_1975"),
        );
        assert_eq!(
            key("James", "van Riemsdyk", 89, 4, 3).as_deref(),
            Some("vanriemsdyk_james_04_05_1989"),
        );
    }

    #[test]
    fn dev_player_key_returns_none_for_missing_parts() {
        assert!(dev_player_key(None, Some("X"), Some(0), Some(0), Some(0)).is_none());
        assert!(dev_player_key(Some("X"), None, Some(0), Some(0), Some(0)).is_none());
        assert!(dev_player_key(Some("X"), Some("Y"), None, Some(0), Some(0)).is_none());
        assert!(key("---", "Smith", 0, 0, 0).is_none());
    }

    #[test]
    fn dev_player_key_rejects_implausible_dates() {
        assert!(key("A", "B", 80, 12, 0).is_none());
        assert!(key("A", "B", 80, 0, 31).is_none());
    }

    #[test]
    fn well_formed_slug_accepts_builder_output() {
        assert!(is_well_formed_player_slug("crosby_sidney_07_08_1987"));
        assert!(is_well_formed_player_slug("malone_joe_28_02_1890"));
        // Every slug the builders emit must validate.
        let s = key("Sidney", "Crosby", 87, 7, 6).unwrap();
        assert!(is_well_formed_player_slug(&s));
    }

    #[test]
    fn well_formed_slug_rejects_degenerate() {
        assert!(!is_well_formed_player_slug("")); // empty
        assert!(!is_well_formed_player_slug("crosby_sidney")); // missing date
        assert!(!is_well_formed_player_slug("_sidney_07_08_1987")); // empty last
        assert!(!is_well_formed_player_slug("crosby__07_08_1987")); // empty first
        assert!(!is_well_formed_player_slug("crosby_sidney_7_8_1987")); // not 2/2 digits
        assert!(!is_well_formed_player_slug("crosby_sidney_32_08_1987")); // day out of range
        assert!(!is_well_formed_player_slug("crosby_sidney_07_13_1987")); // month out of range
        assert!(!is_well_formed_player_slug("crosby_sidney_07_08_87")); // year not 4 digits
    }

    #[test]
    fn historical_player_key_preserves_pre_1900_years() {
        assert_eq!(
            historical_player_key(Some("Joe"), Some("Malone"), Some(1890), Some(2), Some(28),)
                .as_deref(),
            Some("malone_joe_28_02_1890"),
        );
    }
}
