//! The person identity rule: a normalized `last|first|dob` slug that unifies one human across the
//! per-season FHM exports (whose numeric ids are namespaced to a single season).

/// Build the stable person slug from a raw player's last/first name and date-of-birth string.
///
/// Normalization (deterministic, auditable):
/// * lowercase, trim
/// * strip ASCII-fold common Latin diacritics (é→e, ø→o, …) so name spellings that vary across
///   exports still collide
/// * drop any character that is not `a-z`, `0-9`, or a separator
/// * DOB is canonicalized to `YYYY-MM-DD` (FHM exports `YYYY-M-D` with unpadded month/day)
///
/// The three parts are joined with `|`. DOB is 100% populated in the source, so there is no
/// name-only fallback — a missing/blank DOB yields a `|?` marker that keeps such rows distinct
/// rather than silently merging them.
pub fn person_slug(last: &str, first: &str, dob: &str) -> String {
    format!(
        "{}|{}|{}",
        normalize_name(last),
        normalize_name(first),
        normalize_dob(dob),
    )
}

/// Lowercase, diacritic-fold, and strip a name to `[a-z0-9 ]`, collapsing internal whitespace.
pub fn normalize_name(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for ch in s.trim().chars() {
        let folded = fold_diacritic(ch).to_ascii_lowercase();
        if folded.is_ascii_alphanumeric() {
            out.push(folded);
            prev_space = false;
        } else if folded.is_whitespace() || folded == '-' {
            // Whitespace and hyphens are word separators → a single collapsed space.
            if !prev_space && !out.is_empty() {
                out.push(' ');
                prev_space = true;
            }
        }
        // else: apostrophes, periods, and other punctuation are dropped (join the parts).
    }
    // Trim any trailing space introduced above.
    while out.ends_with(' ') {
        out.pop();
    }
    out
}

/// Canonicalize `YYYY-M-D` (unpadded) to `YYYY-MM-DD`. Returns `?` for blank/sentinel input.
pub fn normalize_dob(dob: &str) -> String {
    let t = dob.trim();
    if t.is_empty() || t.starts_with("0-") || t == "0-0-0" {
        return "?".to_string();
    }
    let mut parts = t.split('-');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(y), Some(m), Some(d)) => {
            let (y, m, d) = (y.trim(), m.trim(), d.trim());
            if let (Ok(mi), Ok(di)) = (m.parse::<u32>(), d.parse::<u32>()) {
                format!("{y}-{mi:02}-{di:02}")
            } else {
                t.to_string()
            }
        }
        _ => t.to_string(),
    }
}

/// Fold the handful of Latin-1/Latin-Extended letters that show up in hockey rosters to ASCII.
fn fold_diacritic(ch: char) -> char {
    match ch {
        'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | 'ą' => 'a',
        'ç' | 'č' | 'ć' => 'c',
        'ď' => 'd',
        'é' | 'è' | 'ê' | 'ë' | 'ě' | 'ę' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ł' => 'l',
        'ñ' | 'ń' | 'ň' => 'n',
        'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'ø' | 'ő' => 'o',
        'ř' => 'r',
        'š' | 'ś' => 's',
        'ť' => 't',
        'ú' | 'ù' | 'û' | 'ü' | 'ů' | 'ű' => 'u',
        'ý' | 'ÿ' => 'y',
        'ž' | 'ź' | 'ż' => 'z',
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_is_stable_across_name_spelling_and_dob_padding() {
        // The same human exported two ways collides on one slug.
        assert_eq!(
            person_slug("Crosby", "Sidney", "1987-8-7"),
            person_slug("crosby", "sidney", "1987-08-07"),
        );
    }

    #[test]
    fn diacritics_fold() {
        assert_eq!(
            person_slug("Ovechkin", "Alexander", "1985-9-17"),
            person_slug("Ovechkin", "Alexander", "1985-09-17"),
        );
        assert_eq!(normalize_name("Kämpf"), "kampf");
        assert_eq!(normalize_name("O'Reilly"), "oreilly");
        assert_eq!(normalize_name("Saint-Louis"), "saint louis");
    }

    #[test]
    fn blank_dob_marks_distinct() {
        assert!(person_slug("Nobody", "No", "").ends_with("|?"));
    }
}
