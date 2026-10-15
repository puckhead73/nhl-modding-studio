//! A minimal RFC-4180 CSV reader/writer.
//!
//! The crate already hand-writes CSV in [`crate::matrix`]; this module is the reading half plus a
//! shared quoting helper, so evidence files round-trip exactly. It deliberately supports only what
//! the evidence formats use: comma separator, `"` quoting with `""` escapes, LF or CRLF row
//! terminators, and no comment or type inference. Anything else is a parse error rather than a
//! silently reinterpreted row.

/// Quote a field only when it must be quoted, so output stays diff-friendly and deterministic.
pub fn cell(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Join one record and terminate it with LF.
pub fn row<I: IntoIterator<Item = S>, S: AsRef<str>>(fields: I) -> String {
    let mut out = String::new();
    for (i, f) in fields.into_iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&cell(f.as_ref()));
    }
    out.push('\n');
    out
}

/// A malformed record, with the 1-based line it starts on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvError {
    pub line: usize,
    pub detail: String,
}

impl std::fmt::Display for CsvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.detail)
    }
}

impl std::error::Error for CsvError {}

/// Parse a whole CSV document into records. Empty trailing lines are ignored; an embedded blank
/// line inside the data is a single empty field, which callers reject by width.
pub fn parse(text: &str) -> Result<Vec<Vec<String>>, CsvError> {
    let mut records = Vec::new();
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut line = 1usize;
    let mut chars = text.chars().peekable();
    let mut started = false;

    while let Some(c) = chars.next() {
        started = true;
        if in_quotes {
            match c {
                '"' => {
                    if chars.peek() == Some(&'"') {
                        chars.next();
                        field.push('"');
                    } else {
                        in_quotes = false;
                    }
                }
                '\n' => {
                    line += 1;
                    field.push('\n');
                }
                other => field.push(other),
            }
            continue;
        }
        match c {
            '"' if field.is_empty() => in_quotes = true,
            '"' => {
                return Err(CsvError {
                    line,
                    detail: "unexpected quote inside an unquoted field".into(),
                });
            }
            ',' => fields.push(std::mem::take(&mut field)),
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    continue;
                }
                return Err(CsvError {
                    line,
                    detail: "bare CR is not a record terminator".into(),
                });
            }
            '\n' => {
                fields.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut fields));
                line += 1;
                started = false;
            }
            other => field.push(other),
        }
    }
    if in_quotes {
        return Err(CsvError {
            line,
            detail: "unterminated quoted field".into(),
        });
    }
    if started {
        fields.push(field);
        records.push(fields);
    }
    Ok(records)
}

/// Parse and check the header, returning the data records only.
pub fn parse_with_header(text: &str, expected: &[&str]) -> Result<Vec<Vec<String>>, CsvError> {
    let mut records = parse(text)?;
    if records.is_empty() {
        return Err(CsvError {
            line: 1,
            detail: "file is empty; a header row is required".into(),
        });
    }
    let header = records.remove(0);
    if header != expected {
        return Err(CsvError {
            line: 1,
            detail: format!("header is {header:?}, expected {expected:?}"),
        });
    }
    for (i, record) in records.iter().enumerate() {
        if record.len() != expected.len() {
            return Err(CsvError {
                line: i + 2,
                detail: format!(
                    "record has {} fields, expected {}",
                    record.len(),
                    expected.len()
                ),
            });
        }
    }
    Ok(records)
}

/// How two CSV documents differ, once record terminators are out of the picture.
///
/// See [`compare_records`]. The variants carry enough to name the offending record without the
/// caller re-parsing anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Drift {
    /// One side is not CSV at all. `side` is `"generated"` or `"stored"`.
    Malformed { side: &'static str, error: CsvError },
    /// Both parse, but they hold a different number of records.
    RecordCount { generated: usize, stored: usize },
    /// Record `index` (0-based, header included) differs field-for-field.
    Record {
        index: usize,
        generated: Vec<String>,
        stored: Vec<String>,
    },
}

impl std::fmt::Display for Drift {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Drift::Malformed { side, error } => {
                write!(f, "the {side} document is not CSV: {error}")
            }
            Drift::RecordCount { generated, stored } => write!(
                f,
                "regenerated {generated} record(s), stored file has {stored}"
            ),
            Drift::Record {
                index,
                generated,
                stored,
            } => write!(
                f,
                "record {} differs:\n  regenerated: {generated:?}\n  stored:      {stored:?}",
                index + 1
            ),
        }
    }
}

impl std::error::Error for Drift {}

/// Compare a regenerated CSV document with a stored one **by record**, so that the two legal
/// record terminators — LF and CRLF — are equivalent and nothing else is.
///
/// This exists because every committed generated CSV is written LF (see [`row`]) while a Windows
/// checkout under `core.autocrlf=true` materialises it CRLF, which made byte-equality drift guards
/// fail on a clean checkout while passing in a working tree. Comparing parsed records fixes that
/// **without** normalising, trimming or rewriting either document:
///
/// * the separator/payload distinction is [`parse`]'s, not a string substitution's — a CR or a
///   CRLF **inside a quoted field** is payload, is preserved, and therefore still compares unequal
///   against a bare LF in the same position;
/// * field values, their whitespace, record order and record count are all compared verbatim;
/// * a bare CR outside a quoted field remains a parse error on the side that carries it.
///
/// A trailing terminator on the final record is not a difference (neither is its absence): [`parse`]
/// ignores an empty trailing line on both sides.
pub fn compare_records(generated: &str, stored: &str) -> Result<(), Drift> {
    // Fast path: identical bytes need no parse, and keep the guard exact where the stored file is
    // already LF.
    if generated == stored {
        return Ok(());
    }
    let left = parse(generated).map_err(|error| Drift::Malformed {
        side: "generated",
        error,
    })?;
    let right = parse(stored).map_err(|error| Drift::Malformed {
        side: "stored",
        error,
    })?;
    for (index, (l, r)) in left.iter().zip(right.iter()).enumerate() {
        if l != r {
            return Err(Drift::Record {
                index,
                generated: l.clone(),
                stored: r.clone(),
            });
        }
    }
    if left.len() != right.len() {
        return Err(Drift::RecordCount {
            generated: left.len(),
            stored: right.len(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoting_is_applied_only_where_it_is_needed() {
        assert_eq!(cell("plain"), "plain");
        assert_eq!(cell("a,b"), "\"a,b\"");
        assert_eq!(cell("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    #[test]
    fn a_quoted_field_round_trips_through_parse() {
        let text = row(["a", "b,c", "d\"e"]);
        assert_eq!(
            parse(&text).expect("parses"),
            vec![vec!["a", "b,c", "d\"e"]]
        );
    }

    #[test]
    fn crlf_and_lf_terminators_are_both_accepted() {
        assert_eq!(
            parse("a,b\r\nc,d\n").expect("parses"),
            vec![vec!["a", "b"], vec!["c", "d"]]
        );
    }

    // ------------------------------------------------- compare_records (C0-EVID-REPAIR1) ----

    /// Positive: the same records written with either terminator compare equal.
    #[test]
    fn the_two_legal_record_terminators_are_equivalent() {
        let lf = "a,b\nc,d\n";
        let crlf = "a,b\r\nc,d\r\n";
        assert_eq!(compare_records(lf, crlf), Ok(()));
        assert_eq!(compare_records(crlf, lf), Ok(()));
        // Mixed terminators, and a missing terminator on the last record, are also not drift.
        assert_eq!(compare_records(lf, "a,b\r\nc,d"), Ok(()));
    }

    /// Negative: real content drift still fails, in each of the three shapes a guard can see.
    #[test]
    fn genuine_content_drift_is_still_reported() {
        let stored = "a,b\r\nc,d\r\n";
        // A changed value.
        assert!(matches!(
            compare_records("a,b\nc,D\n", stored),
            Err(Drift::Record { index: 1, .. })
        ));
        // A changed field count in an otherwise identical record.
        assert!(matches!(
            compare_records("a,b\nc,d,e\n", stored),
            Err(Drift::Record { index: 1, .. })
        ));
        // An added record.
        assert!(matches!(
            compare_records("a,b\nc,d\ne,f\n", stored),
            Err(Drift::RecordCount {
                generated: 3,
                stored: 2
            })
        ));
        // A removed record.
        assert!(matches!(
            compare_records("a,b\n", stored),
            Err(Drift::RecordCount {
                generated: 1,
                stored: 2
            })
        ));
    }

    /// Whitespace inside a field is payload and is never trimmed away by the comparison.
    #[test]
    fn field_whitespace_and_record_order_are_not_normalized() {
        assert!(compare_records("a, b\n", "a,b\r\n").is_err());
        assert!(compare_records("a,b\nc,d\n", "c,d\r\na,b\r\n").is_err());
    }

    /// The load-bearing case: a CRLF **inside a quoted field** is payload, not a terminator, so it
    /// must not be silently equated with a bare LF. A naive `replace("\r\n", "\n")` would hide it.
    #[test]
    fn a_crlf_inside_a_quoted_field_is_payload_and_still_compares_unequal() {
        let generated = row(["note", "line1\nline2"]);
        let stored = "note,\"line1\r\nline2\"\r\n";
        assert_eq!(parse(&generated).expect("lf")[0][1], "line1\nline2");
        assert_eq!(parse(stored).expect("crlf")[0][1], "line1\r\nline2");
        assert!(
            compare_records(&generated, stored).is_err(),
            "quoted-field payload must not be normalized"
        );
        assert_eq!(
            generated.replace("\r\n", "\n"),
            stored.replace("\r\n", "\n")
        );
    }

    /// A lone CR inside a quoted field is likewise payload, and outside one it is still an error.
    #[test]
    fn bare_cr_is_payload_when_quoted_and_an_error_when_not() {
        assert_eq!(parse("a,\"b\rc\"\n").expect("quoted")[0][1], "b\rc");
        assert!(matches!(
            compare_records("a,b\rc\n", "a,b\r\n"),
            Err(Drift::Malformed {
                side: "generated",
                ..
            })
        ));
        assert!(matches!(
            compare_records("a,b\n", "a,b\rc\n"),
            Err(Drift::Malformed { side: "stored", .. })
        ));
    }

    #[test]
    fn a_malformed_document_names_its_line() {
        let err = parse("a,\"b\n").expect_err("unterminated");
        assert!(err.detail.contains("unterminated"), "{err}");
        let err = parse_with_header("x,y\n", &["a", "b"]).expect_err("bad header");
        assert_eq!(err.line, 1);
        let err = parse_with_header("a,b\n1\n", &["a", "b"]).expect_err("short record");
        assert_eq!(err.line, 2);
    }
}
