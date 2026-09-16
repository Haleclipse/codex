use anyhow::Result;
use codex_core::StartThreadOptions;
use codex_core::TurnInputRequest;
use codex_protocol::items::AgentMessageContent;
use codex_protocol::items::AgentMessageDelivery;
use codex_protocol::items::AgentMessageItem;
use codex_protocol::items::TurnItem;
use codex_protocol::models::MessagePhase;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::openai_models::ReasoningEffortPreset;
use codex_protocol::openai_models::ToolMessage;
use codex_protocol::openai_models::ToolMessages;
use codex_protocol::openai_models::ToolMode;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_protocol::user_input::UserInput;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_function_call_with_namespace;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::mount_sse_once;
use core_test_support::responses::mount_sse_sequence;
use core_test_support::responses::sse;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use core_test_support::wait_for_event_match;
use pretty_assertions::assert_eq;
use serde_json::json;
use test_case::test_case;

#[test_case(SessionSource::Exec, false, false; "root_without_tool")]
#[test_case(SessionSource::Exec, true, true; "root_with_tool")]
#[test_case(SessionSource::SubAgent(SubAgentSource::Other("test".to_string())), true, false; "subagent")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn persistent_async_message_guidance_follows_tool_availability(
    session_source: SessionSource,
    model_supports_tool: bool,
    expect_tool_guidance: bool,
) -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let responses = mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]),
    )
    .await;
    let test = test_codex()
        .with_model_info_override("gpt-5.2", move |model| {
            model.tool_mode = Some(ToolMode::CodeModeOnly);
            model
                .experimental_supported_tools
                .retain(|tool| tool != "send_user_message_async");
            if model_supports_tool {
                model
                    .experimental_supported_tools
                    .push("send_user_message_async".to_string());
            }
            model
                .supported_reasoning_levels
                .push(ReasoningEffortPreset {
                    effort: ReasoningEffort::Persistent,
                    description: ReasoningEffort::Persistent.to_string(),
                });
            let messages = model
                .model_messages
                .as_mut()
                .expect("model instruction metadata");
            messages.persistent_instructions = None;
            messages.tools = Some(ToolMessages {
                send_user_message_async: Some(ToolMessage {
                    description: Some("Catalog async message description.".to_string()),
                }),
            });
        })
        .with_config(|config| {
            config.model_reasoning_effort = Some(ReasoningEffort::Persistent);
        })
        .build_with_auto_env(&server)
        .await?;
    let thread = test
        .thread_manager
        .start_thread(StartThreadOptions {
            session_source: Some(session_source),
            ..StartThreadOptions::new(test.config.clone())
        })
        .await?
        .thread;
    thread
        .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
            text: "Report progress.".to_string(),
            text_elements: Vec::new(),
        }]))
        .await?;
    wait_for_event(&thread, |event| matches!(event, EventMsg::TurnComplete(_))).await;

    let request = responses.single_request();
    let persistent_instructions = request
        .message_input_texts("developer")
        .into_iter()
        .find(|text| text.starts_with("<persistent_mode>"))
        .expect("persistent guidance should remain available to every agent");
    assert_eq!(
        persistent_instructions.contains("via functions.send_user_message_async"),
        expect_tool_guidance,
    );
    assert_eq!(
        request.body_json()["tools"]
            .as_array()
            .expect("request tools")
            .iter()
            .any(|tool| tool["name"] == "send_user_message_async"),
        expect_tool_guidance,
    );

    Ok(())
}

#[test_case(None; "fallback_description")]
#[test_case(Some(ToolMessages { send_user_message_async: None }); "missing_tool")]
#[test_case(Some(ToolMessages { send_user_message_async: Some(ToolMessage::default()) }); "missing_description")]
#[test_case(Some(ToolMessages { send_user_message_async: Some(ToolMessage { description: Some("Catalog async message description.".to_string()) }) }); "catalog_description")]
#[test_case(Some(ToolMessages { send_user_message_async: Some(ToolMessage { description: Some(String::new()) }) }); "empty_description")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn send_user_message_async_emits_item_and_does_not_end_the_turn(
    tool_messages: Option<ToolMessages>,
) -> Result<()> {
    skip_if_no_network!(Ok(()));

    const CALL_ID: &str = "async-message-call";
    const MESSAGE: &str = "Still investigating.";

    let server = start_mock_server().await;
    let responses = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_function_call_with_namespace(
                    CALL_ID,
                    "functions",
                    "send_user_message_async",
                    &json!({ "message": MESSAGE }).to_string(),
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("final-message", "Finished."),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;
    let expected_description = tool_messages
        .as_ref()
        .and_then(|tools| tools.send_user_message_async.as_ref())
        .and_then(|tool| tool.description.as_deref())
        .unwrap_or(
            "Send a concise message that needs the user's attention during ongoing work. The tool returns immediately without ending the turn or waiting for a reply; any reply arrives asynchronously as a new user message.\nOnly use this tool to ask for missing information, preferences, constraints, clarification, or approval. The message should be concise, easy to read and understand, and at the right level of abstraction that is appropriate for the user and task at hand.",
        )
        .to_string();
    let test = test_codex()
        .with_model_info_override("gpt-5.2", move |model| {
            model.tool_mode = Some(ToolMode::CodeModeOnly);
            model
                .experimental_supported_tools
                .push("send_user_message_async".to_string());
            model
                .model_messages
                .as_mut()
                .expect("model instruction metadata")
                .tools = tool_messages;
        })
        .build_with_auto_env(&server)
        .await?;

    test.codex
        .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
            text: "Keep me updated.".to_string(),
            text_elements: Vec::new(),
        }]))
        .await?;

    let started = wait_for_event_match(test.codex.as_ref(), |event| {
        let EventMsg::ItemStarted(event) = event else {
            return None;
        };
        let TurnItem::AgentMessage(message) = &event.item else {
            return None;
        };
        if message.delivery != Some(AgentMessageDelivery::Async) {
            return None;
        }
        Some(message.clone())
    })
    .await;
    assert_eq!(
        serde_json::to_value(&started)?,
        serde_json::to_value(AgentMessageItem {
            id: CALL_ID.to_string(),
            content: vec![AgentMessageContent::Text {
                text: MESSAGE.to_string(),
            }],
            phase: Some(MessagePhase::FinalAnswer),
            memory_citation: None,
            delivery: Some(AgentMessageDelivery::Async),
        })?
    );

    let completed = wait_for_event_match(test.codex.as_ref(), |event| {
        let EventMsg::ItemCompleted(event) = event else {
            return None;
        };
        let TurnItem::AgentMessage(message) = &event.item else {
            return None;
        };
        if message.delivery != Some(AgentMessageDelivery::Async) {
            return None;
        }
        Some(message.clone())
    })
    .await;
    assert_eq!(
        serde_json::to_value(completed)?,
        serde_json::to_value(started)?
    );

    wait_for_event(test.codex.as_ref(), |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let requests = responses.requests();
    assert_eq!(requests.len(), 2);
    for request in &requests {
        let body = request.body_json();
        let tool = body["tools"]
            .as_array()
            .expect("request tools")
            .iter()
            .find(|tool| tool["type"] == "function" && tool["name"] == "send_user_message_async")
            .expect("the async message tool should be directly visible to the model");
        assert_eq!(tool["description"], expected_description);
    }
    assert_eq!(
        requests[1].function_call_output_text(CALL_ID),
        Some(r#"{"accepted":true}"#.to_string())
    );
    let has_synthetic_assistant_message = requests[1].input().into_iter().any(|item| {
        item["type"] == "message"
            && item["role"] == "assistant"
            && item.to_string().contains(MESSAGE)
    });
    assert!(
        !has_synthetic_assistant_message,
        "the user-visible item should not inject a synthetic assistant message into model context"
    );

    Ok(())
}
