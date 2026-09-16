use codex_protocol::models::FunctionCallOutputContentItem;
use codex_protocol::models::ResponseInputItem;
use codex_tools::FunctionCallError;
use codex_tools::ToolOutput;
use codex_tools::ToolPayload;
use codex_utils_output_truncation::TruncationPolicy;
use pretty_assertions::assert_eq;
use serde_json::json;

use super::HistoryNotesToolOutput;

#[test]
fn preserves_encrypted_history_output() {
    let result = HistoryNotesToolOutput::new(
        json!({"encrypted_output": "enc_payload"}),
        TruncationPolicy::Bytes(1024),
    )
    .expect("bounded encrypted output should be accepted")
    .to_response_item(
        "call-1",
        &ToolPayload::Function {
            arguments: "{}".to_string(),
        },
    );

    let ResponseInputItem::FunctionCallOutput { output, .. } = result else {
        panic!("expected function-call output");
    };
    assert_eq!(
        output.content_items(),
        Some(
            [FunctionCallOutputContentItem::EncryptedContent {
                encrypted_content: "enc_payload".to_string(),
            }]
            .as_slice()
        )
    );
}

#[test]
fn rejects_encrypted_history_output_over_the_tool_limit() {
    assert_eq!(
        HistoryNotesToolOutput::new(
            json!({"encrypted_output": "x".repeat(1025)}),
            TruncationPolicy::Bytes(1024),
        )
        .err(),
        Some(FunctionCallError::RespondToModel(
            "History returned an encrypted result larger than the 1024-byte tool-output limit; retry with narrower bounds".to_string()
        ))
    );
    assert_eq!(
        HistoryNotesToolOutput::new(
            json!({"encrypted_output": "x".repeat(40_001)}),
            TruncationPolicy::Tokens(20_000),
        )
        .err(),
        Some(FunctionCallError::RespondToModel(
            "History returned an encrypted result larger than the 40000-byte tool-output limit; retry with narrower bounds".to_string()
        ))
    );
}
