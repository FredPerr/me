//! Parsing the tool's answer into raw context drafts.
//!
//! The headless run is asked for a single JSON array, but a tool wraps it in
//! prose or a ```json fence anyway, so this scans for the first balanced
//! top-level `[...]` rather than trusting the whole answer to be JSON. The
//! recovered array is then parsed, each object row normalized, and rows without
//! a usable context name dropped. Ported verbatim from the frontend
//! `BulkContextDraft.ts`, with its four failure kinds preserved.

use serde_json::Value;

use super::context_draft::RawContextDraft;

/// Why the tool's answer could not be read as a list of drafts. The four kinds
/// map one-to-one onto the frontend parser's error strings, so the command
/// layer can show the same message for each:
///   - [`NoListFound`](BulkParseError::NoListFound) ← `no-json-found`
///   - [`Malformed`](BulkParseError::Malformed) ← `invalid-json`
///   - [`NotAList`](BulkParseError::NotAList) ← `not-an-array`
///   - [`NoValidEntries`](BulkParseError::NoValidEntries) ← `no-valid-rows`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BulkParseError {
    /// No `[...]` array is present anywhere in the answer.
    NoListFound,
    /// An array was found but it is not valid JSON.
    Malformed,
    /// The parsed JSON is valid but is not an array.
    NotAList,
    /// The array held no row with a usable context name.
    NoValidEntries,
}

/// Parses the tool's answer into raw drafts, or a typed failure. Invalid rows
/// are dropped; the parse only fails when no array is present, the array is
/// malformed, it is not an array, or no row survives normalization.
pub fn parse_context_drafts(raw: &str) -> Result<Vec<RawContextDraft>, BulkParseError> {
    let array_text = extract_json_array(raw).ok_or(BulkParseError::NoListFound)?;

    let parsed: Value = serde_json::from_str(array_text).map_err(|_| BulkParseError::Malformed)?;

    let Value::Array(rows) = parsed else {
        return Err(BulkParseError::NotAList);
    };

    let drafts: Vec<RawContextDraft> = rows.iter().filter_map(normalize_row).collect();

    if drafts.is_empty() {
        return Err(BulkParseError::NoValidEntries);
    }

    Ok(drafts)
}

/// Extracts the first balanced top-level `[...]` from free-form output. Scans
/// character by character, string- and escape-aware, so brackets inside JSON
/// strings do not unbalance the scan. Returns the slice from the first `[` to
/// its matching `]`, or `None` when there is no balanced array.
fn extract_json_array(raw: &str) -> Option<&str> {
    let bytes = raw.as_bytes();
    let start = raw.find('[')?;

    let mut depth = 0_usize;
    let mut in_string = false;
    let mut escaped = false;

    for index in start..bytes.len() {
        let character = bytes[index];

        if in_string {
            if escaped {
                escaped = false;
            } else if character == b'\\' {
                escaped = true;
            } else if character == b'"' {
                in_string = false;
            }
            continue;
        }

        match character {
            b'"' => in_string = true,
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&raw[start..=index]);
                }
            }
            _ => {}
        }
    }

    None
}

/// Normalizes one JSON row into a [`RawContextDraft`], or `None` when the row is
/// not an object or lacks a usable context name. The context name seeds the
/// branch name whenever the model omitted its own `branchName`; the optional
/// `preprompt` is carried over, trimmed, as the draft's `prompt`.
fn normalize_row(row: &Value) -> Option<RawContextDraft> {
    let object = row.as_object()?;
    let context_name = non_empty_trimmed(object.get("contextName"))?;
    let branch_name =
        non_empty_trimmed(object.get("branchName")).unwrap_or_else(|| context_name.clone());
    let prompt = non_empty_trimmed(object.get("preprompt"));

    Some(RawContextDraft {
        context_name,
        branch_name,
        prompt,
    })
}

/// The trimmed string value when it is a non-empty string, else `None` — the
/// Rust reading of the frontend `isNonEmptyString` check.
fn non_empty_trimmed(value: Option<&Value>) -> Option<String> {
    let trimmed = value?.as_str()?.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wraps rows the way the frontend `BulkContextDraft.test.ts` does: a JSON
    /// array embedded in surrounding prose.
    fn wrap_as_output(rows_json: &str) -> String {
        format!("Here are the contexts:\n{rows_json}\nDone.")
    }

    fn draft(context_name: &str, branch_name: &str, prompt: Option<&str>) -> RawContextDraft {
        RawContextDraft {
            context_name: context_name.to_string(),
            branch_name: branch_name.to_string(),
            prompt: prompt.map(str::to_string),
        }
    }

    #[test]
    fn extracts_a_json_array_embedded_in_surrounding_prose() {
        let output =
            wrap_as_output(r#"[{"contextName":"Right-size EC2","branchName":"right-size-ec2"}]"#);

        let drafts = parse_context_drafts(&output).unwrap();

        assert_eq!(
            drafts,
            vec![draft("Right-size EC2", "right-size-ec2", None)]
        );
    }

    #[test]
    fn keeps_the_preprompt_when_present_and_trims_it() {
        let output = r#"[{"contextName":"Delete volumes","branchName":"delete-volumes","preprompt":"  do the thing  "}]"#;

        let drafts = parse_context_drafts(output).unwrap();

        assert_eq!(drafts[0].prompt.as_deref(), Some("do the thing"));
    }

    #[test]
    fn omits_the_preprompt_when_it_is_missing_or_blank() {
        let output = r#"[{"contextName":"A","branchName":"a","preprompt":"   "},{"contextName":"B","branchName":"b"}]"#;

        let drafts = parse_context_drafts(output).unwrap();

        assert_eq!(drafts[0].prompt, None);
        assert_eq!(drafts[1].prompt, None);
    }

    #[test]
    fn keeps_the_proposed_branch_name_trimmed_for_the_project_to_name() {
        let output = r#"[{"contextName":"New API","branchName":"  New API!!  "}]"#;

        let drafts = parse_context_drafts(output).unwrap();

        assert_eq!(drafts[0].branch_name, "New API!!");
    }

    #[test]
    fn seeds_the_branch_name_from_the_context_name_when_branch_name_is_missing() {
        let output = r#"[{"contextName":"Clean Up Logs"}]"#;

        let drafts = parse_context_drafts(output).unwrap();

        assert_eq!(drafts[0].branch_name, "Clean Up Logs");
    }

    #[test]
    fn drops_rows_without_a_usable_context_name() {
        let output = r#"[{"branchName":"orphan"},{"contextName":"Valid","branchName":"valid"}]"#;

        let drafts = parse_context_drafts(output).unwrap();

        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].context_name, "Valid");
    }

    #[test]
    fn returns_no_list_found_when_there_is_no_array() {
        let result = parse_context_drafts("I could not do that.");

        assert_eq!(result, Err(BulkParseError::NoListFound));
    }

    #[test]
    fn returns_malformed_when_the_array_is_malformed() {
        let result = parse_context_drafts(r#"[{ "contextName": }]"#);

        assert_eq!(result, Err(BulkParseError::Malformed));
    }

    #[test]
    fn a_json_array_of_non_objects_drops_every_row() {
        // Every row that is not an object is dropped, so an array of strings is
        // NoValidEntries, mirroring the frontend. (The scanner always recovers a
        // bracketed slice that parses to an array, so NotAList is never reached
        // through extraction; it exists only for parity with the frontend's four
        // kinds and is exercised directly below.)
        let result = parse_context_drafts(r#"["not", "objects"]"#);

        assert_eq!(result, Err(BulkParseError::NoValidEntries));
    }

    #[test]
    fn returns_no_valid_entries_when_every_row_is_invalid() {
        let output = r#"[{"branchName":"x"},{"contextName":""}]"#;

        let result = parse_context_drafts(output);

        assert_eq!(result, Err(BulkParseError::NoValidEntries));
    }
}
