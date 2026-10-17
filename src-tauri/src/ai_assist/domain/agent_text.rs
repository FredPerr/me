//! Reconstructing an AI tool's final answer from its output lines.
//!
//! Kiro's `--output-format stream-json` prints one self-describing JSON object
//! (an ACP event) per line on stdout. [`extract_agent_text`] rebuilds the
//! agent's natural-language answer from those lines, preferring the
//! authoritative `runFinished.finalText` and otherwise concatenating the
//! `agent_message_chunk` text. When the lines are not an ACP stream at all
//! (a tool that prints plain text), it falls back to joining them with
//! newlines, so nothing is dropped.
//!
//! Ported from the frontend `acpEvents.ts`/`acpStreamToParts.ts` as a pure
//! function over `&[String]`, mirroring how the Kiro `conversation_id` parser
//! was moved to Rust: lines are parsed with `serde_json::Value`, and anything
//! that is not JSON or not a modeled event is ignored (or, for a wholly
//! non-ACP stream, joined verbatim).

use serde_json::Value;

/// The modeled subset of ACP events this reconstruction cares about. Tool calls
/// and their updates are irrelevant to the final answer, so they are not
/// modeled here (unlike the frontend, which renders them).
enum AgentTextEvent {
    /// A streamed chunk of the agent's natural-language answer.
    MessageChunk { text: String },
    /// The run finished; `final_text` is the authoritative, de-chunked answer.
    RunFinished { final_text: Option<String> },
}

/// Parses one line into a modeled [`AgentTextEvent`], or `None` when the line is
/// not JSON, not an object, or an event we do not model.
fn parse_agent_text_event(line: &str) -> Option<AgentTextEvent> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let envelope: Value = serde_json::from_str(trimmed).ok()?;
    let event_type = envelope.get("type")?.as_str()?;
    let data = envelope.get("data")?;

    if event_type == "runFinished" {
        let final_text = data
            .get("finalText")
            .and_then(Value::as_str)
            .map(str::to_string);
        return Some(AgentTextEvent::RunFinished { final_text });
    }

    if event_type != "sessionUpdate" {
        return None;
    }

    let update = data.get("update")?;
    let session_update = update.get("sessionUpdate")?.as_str()?;
    if session_update != "agent_message_chunk" {
        return None;
    }

    let text = update.get("content")?.get("text")?.as_str()?;
    Some(AgentTextEvent::MessageChunk {
        text: text.to_string(),
    })
}

/// True when at least one line parses as a modeled ACP event, mirroring the
/// frontend `looksLikeAcpStream`. Decides ACP reconstruction vs. plain joining.
fn looks_like_acp_stream(lines: &[String]) -> bool {
    lines
        .iter()
        .any(|line| parse_agent_text_event(line).is_some())
}

/// Reconstructs the agent's answer from `lines`.
///
/// For an ACP stream: returns `runFinished.finalText` when present (the
/// authoritative answer), otherwise the concatenation of the
/// `agent_message_chunk` text. For a non-ACP stream (no modeled event on any
/// line): joins the lines with newlines, so a plain-text tool's output survives.
pub fn extract_agent_text(lines: &[String]) -> String {
    if !looks_like_acp_stream(lines) {
        return lines.join("\n");
    }

    let mut chunks = String::new();
    for line in lines {
        match parse_agent_text_event(line) {
            // An empty finalText is not authoritative (mirrors the frontend's
            // truthy check), so it falls through to the concatenated chunks.
            Some(AgentTextEvent::RunFinished {
                final_text: Some(final_text),
            }) if !final_text.is_empty() => return final_text,
            Some(AgentTextEvent::MessageChunk { text }) => chunks.push_str(&text),
            _ => {}
        }
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    // Lines captured from kiro-cli 2.27 `--output-format stream-json`, matching
    // the frontend `acpEvents.test.ts` fixtures.
    const METADATA: &str =
        r#"{"type":"metadata","data":{"sessionId":"s1","contextUsagePercentage":4.4}}"#;
    const MESSAGE_CHUNK_HI: &str = r#"{"type":"sessionUpdate","data":{"sessionId":"s1","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"Hi"}}}}"#;
    const MESSAGE_CHUNK_THERE: &str = r#"{"type":"sessionUpdate","data":{"sessionId":"s1","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":" there"}}}}"#;
    const TOOL_CALL: &str = r#"{"type":"sessionUpdate","data":{"update":{"sessionUpdate":"tool_call","toolCallId":"t1","title":"Running: echo hi","kind":"execute","rawInput":{"command":"echo hi"},"_meta":{"kiro":{"toolName":"shell"}}}}}"#;
    const RUN_FINISHED: &str = r#"{"type":"runFinished","data":{"status":"success","stopReason":"end_turn","finalText":"Output: hi"}}"#;

    fn lines(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn prefers_the_authoritative_final_text_when_present() {
        let text = extract_agent_text(&lines(&[MESSAGE_CHUNK_HI, RUN_FINISHED]));

        assert_eq!(text, "Output: hi");
    }

    #[test]
    fn concatenates_message_chunks_when_there_is_no_final_text() {
        let text = extract_agent_text(&lines(&[MESSAGE_CHUNK_HI, MESSAGE_CHUNK_THERE]));

        assert_eq!(text, "Hi there");
    }

    #[test]
    fn ignores_metadata_and_non_json_noise_in_an_acp_stream() {
        let text = extract_agent_text(&lines(&["noise", METADATA, MESSAGE_CHUNK_HI]));

        assert_eq!(text, "Hi");
    }

    #[test]
    fn a_tool_call_is_not_treated_as_answer_text() {
        // The stream is ACP (the tool call is a modeled envelope type), but a
        // tool call carries no answer text, so only the chunk survives.
        let text = extract_agent_text(&lines(&[TOOL_CALL, MESSAGE_CHUNK_HI]));

        assert_eq!(text, "Hi");
    }

    #[test]
    fn joins_plain_non_acp_lines_with_newlines() {
        let text = extract_agent_text(&lines(&["just", "plain", "lines"]));

        assert_eq!(text, "just\nplain\nlines");
    }

    #[test]
    fn an_empty_final_text_falls_back_to_concatenated_chunks() {
        let run_finished_empty =
            r#"{"type":"runFinished","data":{"status":"success","finalText":""}}"#;

        let text = extract_agent_text(&lines(&[MESSAGE_CHUNK_HI, run_finished_empty]));

        // An empty finalText is not authoritative; the chunk is the real answer.
        assert_eq!(text, "Hi");
    }
}
