//! Title-text keys on the presentation cards (songs, goal horns, genres) resolved through the
//! **open title's own attached dictionary** — the same service the team-text card uses
//! ([`crate::leagues::resolve_text`]; coordinator review of receipts 10–17, item 2, 22 Sep 2026).
//!
//! The PS4 catalogue CSVs write a dictionary reference as `$TXT_SONG_ALIEN_DANCE` /
//! `$NHLTeamName_ANA`; the dictionary rows carry the key without the `$` (probed on the NHL 15 /
//! 16 / 17 / 18 / 20 anchor dictionaries: `TXT_SONG_ALIEN_DANCE` resolves exactly on 17 / 18 / 20,
//! `$TXT_SONG_…` never). So one leading `$` is stripped before the lookup, and the raw token is
//! always kept beside the result. A key the dictionary does not carry — or no dictionary at all —
//! leaves the raw key as the display text with an explicit unresolved state; nothing borrows a
//! sibling title's dictionary, because the dictionary is the project's own attachment.

use serde::{Deserialize, Serialize};

use crate::leagues::{resolve_text, AttachedDictionary, Resolved};

/// How a title-text key resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeyLookup {
    /// The dictionary carries the key exactly.
    Exact,
    /// Resolved through the game's case-folded hash to a single row with another spelling.
    ByHash,
    /// A dictionary is attached but has no row for the key: the raw key stays on display.
    Absent,
    /// No dictionary is attached to this project: the raw key stays on display.
    NoDictionary,
    /// The token is not a dictionary reference (empty, or not a `$`-marked / `TXT_` key).
    NotAKey,
}

impl KeyLookup {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::ByHash => "by-hash",
            Self::Absent => "absent",
            Self::NoDictionary => "no-dictionary",
            Self::NotAKey => "not-a-key",
        }
    }
}

/// One resolved (or explicitly unresolved) title-text key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyText {
    /// The token exactly as the source carries it (`$TXT_SONG_ALIEN_DANCE`).
    pub raw: String,
    /// The key looked up (`TXT_SONG_ALIEN_DANCE`), or `None` when the token is not a key.
    pub key: Option<String>,
    pub lookup: KeyLookup,
    /// The dictionary text when it resolved; `None` otherwise — callers show `raw` then.
    pub text: Option<String>,
    /// The dictionary row's own key on a by-hash resolution.
    pub row_key: Option<String>,
}

impl KeyText {
    /// The text to display: the resolved text, else the raw token.
    pub fn display(&self) -> &str {
        self.text.as_deref().unwrap_or(&self.raw)
    }

    pub fn resolved(&self) -> bool {
        self.text.is_some()
    }
}

/// The key a catalogue token names: one leading `$` stripped; an empty token is not a key.
pub fn key_of(raw: &str) -> Option<&str> {
    let k = raw.strip_prefix('$').unwrap_or(raw);
    (!k.trim().is_empty()).then_some(k)
}

/// Resolve `raw` through `dictionary` (the project's own attachment, or `None`).
pub fn resolve_key_text(dictionary: Option<&AttachedDictionary>, raw: &str) -> KeyText {
    let Some(key) = key_of(raw) else {
        return KeyText { raw: raw.to_string(), key: None, lookup: KeyLookup::NotAKey, text: None, row_key: None };
    };
    let key = key.to_string();
    match dictionary {
        None => KeyText { raw: raw.to_string(), key: Some(key), lookup: KeyLookup::NoDictionary, text: None, row_key: None },
        Some(d) => match resolve_text(d, &key) {
            Some((t, Resolved::Exact)) => KeyText { raw: raw.to_string(), key: Some(key), lookup: KeyLookup::Exact, text: Some(t.to_string()), row_key: None },
            Some((t, Resolved::ByHash(rk))) => KeyText { raw: raw.to_string(), key: Some(key), lookup: KeyLookup::ByHash, text: Some(t.to_string()), row_key: Some(rk) },
            None => KeyText { raw: raw.to_string(), key: Some(key), lookup: KeyLookup::Absent, text: None, row_key: None },
        },
    }
}

/// The dictionary a listing resolved through, for the reply stamp (so a card can say which
/// attachment produced its labels and a late reply for another attachment is recognisable).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DictionaryStamp {
    pub locale: String,
    pub sha256: String,
    pub rows: usize,
}

pub fn stamp(dictionary: Option<&AttachedDictionary>) -> Option<DictionaryStamp> {
    dictionary.map(|d| DictionaryStamp { locale: d.locale.clone(), sha256: d.sha256.clone(), rows: d.dictionary.len() })
}

/// The one-line disclosure of how a set of keys resolved.
pub fn resolution_note(dictionary: Option<&AttachedDictionary>, what: &str, resolved: usize, unresolved: usize) -> String {
    match dictionary {
        None => format!("no dictionary is attached to this project: the {what} keys are shown as keys (attach the title's own localization file to resolve them; a sibling title's dictionary is never borrowed)"),
        Some(d) => format!(
            "{what} keys resolved through this project's attached dictionary `{}` ({} rows): {resolved} resolved, {unresolved} not in the dictionary and shown as keys; the `$` in the catalogue marks a dictionary reference and is not part of the key",
            d.locale,
            d.dictionary.len()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anchor_dictionary(title: &str) -> AttachedDictionary {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(format!("anchors/ps4/{title}/databases/eng_us.db"));
        AttachedDictionary::load(&p.to_string_lossy()).expect("the committed PS4 anchor dictionary loads")
    }

    /// The `$` marker is stripped, exact and absent keys are distinguished, no dictionary and
    /// not-a-key are their own states, and the raw token is always kept.
    #[test]
    fn keys_resolve_through_the_attached_dictionary_only() {
        let d = anchor_dictionary("nhl17");
        let k = resolve_key_text(Some(&d), "$TXT_SONG_ALIEN_DANCE");
        assert_eq!((k.key.as_deref(), k.lookup, k.text.as_deref()), (Some("TXT_SONG_ALIEN_DANCE"), KeyLookup::Exact, Some("Alien Dance")));
        assert_eq!(k.raw, "$TXT_SONG_ALIEN_DANCE");
        assert_eq!(k.display(), "Alien Dance");
        let k = resolve_key_text(Some(&d), "$NHLTeamName_ANA");
        assert_eq!((k.lookup, k.text.as_deref()), (KeyLookup::Exact, Some("Anaheim Ducks®")));
        let k = resolve_key_text(Some(&d), "$TXT_ROCK");
        assert_eq!((k.lookup, k.text.as_deref()), (KeyLookup::Exact, Some("Rock")));
        // Absent: the raw key stays on display, explicitly unresolved.
        let k = resolve_key_text(Some(&d), "$TXT_SONG_NOT_A_SHIPPED_SONG");
        assert_eq!((k.lookup, k.text.as_deref(), k.display()), (KeyLookup::Absent, None, "$TXT_SONG_NOT_A_SHIPPED_SONG"));
        // No dictionary.
        let k = resolve_key_text(None, "$TXT_SONG_ALIEN_DANCE");
        assert_eq!((k.lookup, k.display()), (KeyLookup::NoDictionary, "$TXT_SONG_ALIEN_DANCE"));
        assert_eq!(k.key.as_deref(), Some("TXT_SONG_ALIEN_DANCE"));
        // Not a key.
        let k = resolve_key_text(Some(&d), "");
        assert_eq!(k.lookup, KeyLookup::NotAKey);
        assert_eq!(resolve_key_text(Some(&d), "$").lookup, KeyLookup::NotAKey);
        // Never borrowed from a sibling: a key the NHL 15 dictionary lacks stays absent on it even
        // though NHL 17's carries it.
        let d15 = anchor_dictionary("nhl15");
        assert_eq!(resolve_key_text(Some(&d15), "$TXT_SONG_ALIEN_DANCE").lookup, KeyLookup::Absent);
        assert_eq!(resolve_key_text(Some(&d15), "$NHLTeamName_ANA").text.as_deref(), Some("Anaheim Ducks®"));
        assert!(resolution_note(None, "song title", 0, 3).contains("never borrowed"));
        assert!(resolution_note(Some(&d), "song title", 2, 1).contains("`eng_us`"));
        assert_eq!(stamp(Some(&d)).unwrap().locale, "eng_us");
        assert!(stamp(None).is_none());
    }
}
