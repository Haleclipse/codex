//! Exercises retained review history through compaction, eviction, large images, and rollback.

use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use codex_core::TurnInputRequest;
use codex_core::config::Constrained;
use codex_features::Feature;
use codex_protocol::config_types::ApprovalsReviewer;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_protocol::request_user_input::RequestUserInputAnswer;
use codex_protocol::request_user_input::RequestUserInputResponse;
use codex_protocol::user_input::UserInput;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_function_call;
use core_test_support::responses::mount_sse_sequence;
use core_test_support::responses::sse;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::skip_if_wine_exec;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use core_test_support::wait_for_event_match;
use image::DynamicImage;
use image::RgbImage;
use pretty_assertions::assert_eq;
use rand::RngCore;
use rand::SeedableRng;
use rand::rngs::StdRng;
use serde_json::json;
use std::collections::HashMap;
use std::io::Cursor;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guardian_history_survives_compaction_and_eviction_but_not_rollback() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_wine_exec!(
        Ok(()),
        "Guardian approval actions require host-native paths"
    );
    let server = start_mock_server().await;
    let test = test_codex()
        .with_config(|config| {
            config.features.enable(Feature::TokenBudget).unwrap();
            config
                .features
                .enable(Feature::DefaultModeRequestUserInput)
                .unwrap();
            config.update_plan_enabled = true;
            config.permissions.approval_policy = Constrained::allow_any(AskForApproval::OnRequest);
            config.approvals_reviewer = ApprovalsReviewer::AutoReview;
        })
        .build_with_auto_env(&server)
        .await?;
    // Enough real tool traffic to evict earlier tools when retention starts.
    let plan = r#"{"plan":[{"step":"verify repository visibility","status":"completed"}]}"#;
    let mut inspection: Vec<_> = (0..130)
        .map(|index| ev_function_call(&format!("inspect-{index}"), "update_plan", plan))
        .collect();
    inspection.push(ev_completed("inspection"));
    mount_sse_sequence(
        &server,
        vec![
            sse(inspection),
            sse(vec![
                ev_function_call("inspect-latest", "update_plan", plan),
                ev_function_call(
                    "confirm-publish",
                    "request_user_input",
                    &json!({
                        "questions": [{
                            "id": "publish", "header": "Publish", "question": "May I publish?",
                            "options": [
                                {"label": "Yes", "description": "Publish the change."},
                                {"label": "No", "description": "Keep the change local."}
                            ]
                        }]
                    })
                    .to_string(),
                ),
                ev_completed("latest-inspection"),
            ]),
            sse(vec![
                ev_assistant_message("inspected", "Inspection complete."),
                ev_completed("inspection-done"),
            ]),
        ],
    )
    .await;
    test.codex
        .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
            text: "Only publish to a private repository.".into(),
            text_elements: Vec::new(),
        }]))
        .await?;
    let question = wait_for_event_match(&test.codex, |event| match event {
        EventMsg::RequestUserInput(request) => Some(request.clone()),
        _ => None,
    })
    .await;
    test.codex
        .submit(Op::UserInputAnswer {
            id: question.turn_id,
            response: RequestUserInputResponse {
                answers: HashMap::from([(
                    "publish".to_owned(),
                    RequestUserInputAnswer {
                        answers: vec!["Do not publish anything.".to_owned()],
                    },
                )]),
            },
        })
        .await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    test.codex.submit(Op::Compact).await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    // A valid, supported-size image whose encoded bytes exceed the retention budget.
    let (width, height) = (1200, 1200);
    let mut pixels = vec![0; (width * height * 3) as usize];
    let seed = 42;
    StdRng::seed_from_u64(seed).fill_bytes(&mut pixels);
    let image = RgbImage::from_raw(width, height, pixels).unwrap();
    let mut image_data = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(image).write_to(&mut image_data, image::ImageFormat::Png)?;
    let image_url = format!(
        "data:image/png;base64,{}",
        BASE64_STANDARD.encode(image_data.into_inner())
    );
    assert!(image_url.len() > 4 * 1024 * 1024);
    let command = r#"{"cmd":"echo publish","sandbox_permissions":"require_escalated","justification":"Publish the inspected change."}"#;
    for (prompt, retained) in [
        ("Do not publish the attached image.", true),
        ("Inspect a different repository.", false),
    ] {
        let review = mount_sse_sequence(
            &server,
            vec![
                sse(vec![
                    ev_function_call("publish", "exec_command", command),
                    ev_completed("publish"),
                ]),
                sse(vec![
                    ev_assistant_message("review", r#"{"outcome":"deny"}"#),
                    ev_completed("review"),
                ]),
                sse(vec![ev_completed("publish-done")]),
            ],
        )
        .await;
        test.codex
            .start_or_steer_turn(TurnInputRequest::user_input(vec![
                UserInput::Text {
                    text: prompt.to_owned(),
                    text_elements: Vec::new(),
                },
                UserInput::Image {
                    image_url: image_url.clone(),
                    detail: None,
                },
            ]))
            .await?;
        wait_for_event(&test.codex, |event| {
            matches!(event, EventMsg::TurnComplete(_))
        })
        .await;
        let requests = review.requests();
        assert!(
            requests[0]
                .input()
                .iter()
                .filter_map(|item| item["content"].as_array())
                .flatten()
                .any(|item| item["image_url"]
                    .as_str()
                    .is_some_and(|url| url.len() > 4 * 1024 * 1024))
        );
        let guardian = requests
            .iter()
            .find(|request| {
                request.body_json()["client_metadata"]["x-openai-subagent"] == "guardian"
            })
            .expect("Guardian request");
        let transcript = serde_json::to_string(&guardian.input())?;
        assert!(transcript.contains(prompt));
        if retained {
            let trusted_answers = transcript
                .split_once(">>> TRUSTED USER ANSWERS START")
                .expect("trusted answers survive compaction")
                .1
                .split_once(">>> TRUSTED USER ANSWERS END")
                .expect("trusted answers end marker")
                .0;
            assert!(trusted_answers.contains("user: Do not publish anything."));
            let positions = [
                "Only publish to a private repository.",
                "tool update_plan call",
                "tool update_plan result",
                "Do not publish the attached image.",
            ]
            .map(|text| {
                transcript
                    .find(text)
                    .unwrap_or_else(|| panic!("missing {text}: {transcript}"))
            });
            let mut ordered = positions;
            ordered.sort();
            assert_eq!(positions, ordered);
            assert!(
                requests[0]
                    .input()
                    .iter()
                    .all(|item| item["call_id"] != "inspect-0"
                        && item["call_id"] != "confirm-publish")
            );
            test.codex.ensure_rollout_materialized().await;
            test.codex
                .submit(Op::ThreadRollback { num_turns: 2 })
                .await?;
            wait_for_event(&test.codex, |event| {
                matches!(event, EventMsg::ThreadRolledBack(_))
            })
            .await;
        } else {
            assert!(!transcript.contains(">>> TRUSTED USER ANSWERS START"));
            assert!(!transcript.contains("Do not publish anything."));
            assert!(!transcript.contains("Only publish to a private repository."));
            assert!(!transcript.contains("tool update_plan call"));
            assert!(!transcript.contains("tool update_plan result"));
        }
    }
    Ok(())
}
