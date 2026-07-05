use serde_json::Value;

use crate::ProviderError;

/// Extracts the model-generated text from an OpenAI Responses API envelope:
/// `{"output":[{"type":"message","content":[{"type":"output_text","text":"..."}]}]}`.
/// A top-level `"output_text"` string is also accepted for servers that
/// return the SDK convenience field directly.
pub(crate) fn openai_responses_text(body: &str) -> Result<String, ProviderError> {
    let envelope = parse_envelope(body, "OpenAI")?;

    if let Some(text) = envelope.get("output_text").and_then(Value::as_str) {
        return non_empty(text.to_owned(), "OpenAI");
    }

    let mut collected = String::new();
    if let Some(output) = envelope.get("output").and_then(Value::as_array) {
        for item in output {
            let Some(content) = item.get("content").and_then(Value::as_array) else {
                continue;
            };
            for part in content {
                if part.get("type").and_then(Value::as_str) == Some("output_text") {
                    if let Some(text) = part.get("text").and_then(Value::as_str) {
                        collected.push_str(text);
                    }
                }
            }
        }
    }

    non_empty(collected, "OpenAI")
}

/// Extracts the assistant message text from a chat-completions envelope:
/// `{"choices":[{"message":{"content":"..."}}]}`. Content given as an array
/// of `{"type":"text","text":"..."}` parts is concatenated.
pub(crate) fn chat_completions_text(body: &str) -> Result<String, ProviderError> {
    let envelope = parse_envelope(body, "OpenAI-compatible")?;

    let content = envelope
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"));

    let collected = match content {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(parts)) => parts
            .iter()
            .filter(|part| part.get("type").and_then(Value::as_str) == Some("text"))
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .collect(),
        _ => String::new(),
    };

    non_empty(collected, "OpenAI-compatible")
}

/// Extracts the model output from an Anthropic Messages envelope:
/// `{"content":[{"type":"text","text":"..."}]}`. When the response carries a
/// `tool_use` block (structured output), its `input` object is returned as
/// serialized JSON.
pub(crate) fn anthropic_messages_text(body: &str) -> Result<String, ProviderError> {
    let envelope = parse_envelope(body, "Anthropic")?;

    let Some(content) = envelope.get("content").and_then(Value::as_array) else {
        return non_empty(String::new(), "Anthropic");
    };

    for block in content {
        if block.get("type").and_then(Value::as_str) == Some("tool_use") {
            if let Some(input) = block.get("input") {
                let serialized =
                    serde_json::to_string(input).map_err(|_| invalid_envelope("Anthropic"))?;
                return non_empty(serialized, "Anthropic");
            }
        }
    }

    let collected: String = content
        .iter()
        .filter(|block| block.get("type").and_then(Value::as_str) == Some("text"))
        .filter_map(|block| block.get("text").and_then(Value::as_str))
        .collect();

    non_empty(collected, "Anthropic")
}

/// Extracts the model text from a Gemini generateContent envelope:
/// `{"candidates":[{"content":{"parts":[{"text":"..."}]}}]}`.
pub(crate) fn gemini_candidates_text(body: &str) -> Result<String, ProviderError> {
    let envelope = parse_envelope(body, "Gemini")?;

    let collected: String = envelope
        .get("candidates")
        .and_then(Value::as_array)
        .and_then(|candidates| candidates.first())
        .and_then(|candidate| candidate.get("content"))
        .and_then(|content| content.get("parts"))
        .and_then(Value::as_array)
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect()
        })
        .unwrap_or_default();

    non_empty(collected, "Gemini")
}

fn parse_envelope(body: &str, provider: &str) -> Result<Value, ProviderError> {
    serde_json::from_str(body).map_err(|_| ProviderError::InvalidResponse {
        message: format!("{provider} response body was not valid JSON"),
    })
}

fn non_empty(text: String, provider: &str) -> Result<String, ProviderError> {
    if text.trim().is_empty() {
        Err(invalid_envelope(provider))
    } else {
        Ok(text)
    }
}

fn invalid_envelope(provider: &str) -> ProviderError {
    ProviderError::InvalidResponse {
        message: format!("{provider} response contained no model output text"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_extracts_output_text_blocks() {
        let body = r#"{
            "id": "resp_123",
            "output": [
                {"type": "reasoning", "summary": []},
                {"type": "message", "role": "assistant", "content": [
                    {"type": "output_text", "text": "{\"tokens\":"},
                    {"type": "output_text", "text": "[]}"}
                ]}
            ],
            "usage": {"input_tokens": 10}
        }"#;
        assert_eq!(openai_responses_text(body).unwrap(), r#"{"tokens":[]}"#);
    }

    #[test]
    fn openai_accepts_top_level_output_text() {
        assert_eq!(
            openai_responses_text(r#"{"output_text":"{\"a\":1}"}"#).unwrap(),
            r#"{"a":1}"#
        );
    }

    #[test]
    fn openai_rejects_envelope_without_text() {
        let error = openai_responses_text(r#"{"output":[]}"#).unwrap_err();
        assert!(matches!(error, ProviderError::InvalidResponse { .. }));
    }

    #[test]
    fn chat_completions_extracts_string_content() {
        let body = r#"{"choices":[{"index":0,"message":{"role":"assistant","content":"{\"colors\":[]}"},"finish_reason":"stop"}]}"#;
        assert_eq!(chat_completions_text(body).unwrap(), r#"{"colors":[]}"#);
    }

    #[test]
    fn chat_completions_concatenates_text_parts() {
        let body = r#"{"choices":[{"message":{"content":[{"type":"text","text":"{\"a\":"},{"type":"text","text":"1}"}]}}]}"#;
        assert_eq!(chat_completions_text(body).unwrap(), r#"{"a":1}"#);
    }

    #[test]
    fn chat_completions_rejects_missing_choices() {
        let error = chat_completions_text(r#"{"choices":[]}"#).unwrap_err();
        assert!(matches!(error, ProviderError::InvalidResponse { .. }));
    }

    #[test]
    fn chat_completions_error_does_not_leak_body() {
        let error = chat_completions_text(r#"{"secret":"sk-leak"}"#).unwrap_err();
        assert!(!format!("{error}").contains("sk-leak"));
    }

    #[test]
    fn anthropic_extracts_text_blocks() {
        let body = r#"{"id":"msg_1","content":[{"type":"text","text":"{\"x\":1}"}],"stop_reason":"end_turn"}"#;
        assert_eq!(anthropic_messages_text(body).unwrap(), r#"{"x":1}"#);
    }

    #[test]
    fn anthropic_prefers_tool_use_input() {
        let body = r#"{"content":[{"type":"tool_use","id":"tu_1","name":"emit_design_spec","input":{"tokens":[]}}]}"#;
        assert_eq!(anthropic_messages_text(body).unwrap(), r#"{"tokens":[]}"#);
    }

    #[test]
    fn anthropic_rejects_empty_content() {
        let error = anthropic_messages_text(r#"{"content":[]}"#).unwrap_err();
        assert!(matches!(error, ProviderError::InvalidResponse { .. }));
    }

    #[test]
    fn gemini_extracts_candidate_parts() {
        let body = r#"{"candidates":[{"content":{"parts":[{"text":"{\"y\":"},{"text":"2}"}],"role":"model"},"finishReason":"STOP"}]}"#;
        assert_eq!(gemini_candidates_text(body).unwrap(), r#"{"y":2}"#);
    }

    #[test]
    fn gemini_rejects_missing_candidates() {
        let error = gemini_candidates_text(r#"{"candidates":[]}"#).unwrap_err();
        assert!(matches!(error, ProviderError::InvalidResponse { .. }));
    }

    #[test]
    fn invalid_json_is_rejected_without_leaking_body() {
        let error = openai_responses_text("not json sk-leak").unwrap_err();
        assert!(matches!(error, ProviderError::InvalidResponse { .. }));
        assert!(!format!("{error}").contains("sk-leak"));
    }
}
