use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use codex_protocol::models::ResponseItem;
use pretty_assertions::assert_eq;

use super::ContextSection;
use super::ContextTarget;
use super::ConversationTranscriptConfig;
use super::ConversationTranscriptEntry;
use super::ConversationTranscriptEntryKind;
use super::ConversationTranscriptOptions;
use super::SectionContributor;
use super::SectionError;
use super::SectionInput;
use super::SectionRegistry;
use super::SectionScope;
use super::TranscriptEntryLimits;

struct TestContributor {
    outcome: Result<Option<&'static str>, SectionError>,
    scope: SectionScope,
    invocations: Arc<AtomicUsize>,
}

impl SectionContributor for TestContributor {
    fn scope(&self) -> SectionScope {
        self.scope
    }

    fn contribute(&self, input: &SectionInput<'_>) -> Result<Option<ContextSection>, SectionError> {
        self.invocations.fetch_add(/*val*/ 1, Ordering::Relaxed);
        let history_len = input.history.items().count();
        Ok(self
            .outcome
            .clone()?
            .map(|label| section(label, history_len)))
    }
}

fn section(label: &str, history_len: usize) -> ContextSection {
    let text = format!("{label}: history items: {history_len}");
    ContextSection {
        items: vec![ConversationTranscriptEntry {
            kind: ConversationTranscriptEntryKind::User,
            original_bytes: text.len(),
            text,
        }],
    }
}

fn transcript_config() -> ConversationTranscriptConfig {
    ConversationTranscriptConfig {
        options: ConversationTranscriptOptions::default(),
        entry_limits: TranscriptEntryLimits {
            message_tokens: 2_000,
            tool_tokens: 1_000,
            node_repl_output_tokens: 2_000,
        },
    }
}

#[test]
fn registry_collects_target_specific_sections_in_registration_order() {
    let mut registry = SectionRegistry::default();
    let mut invocations = Vec::new();
    for (label, scope) in [
        ("root", SectionScope::Shared),
        ("permissions", SectionScope::SyncOnly),
        ("reviews", SectionScope::AsyncOnly),
        ("action", SectionScope::Shared),
    ] {
        let calls = Arc::new(AtomicUsize::new(/*v*/ 0));
        registry.register(TestContributor {
            outcome: Ok(Some(label)),
            scope,
            invocations: Arc::clone(&calls),
        });
        invocations.push(calls);
    }
    let history = [ResponseItem::Other];
    let transcript = transcript_config();

    let sync_sections = registry.collect(&SectionInput {
        target: ContextTarget::Sync,
        history: &history,
        transcript: &transcript,
    });
    let async_sections = registry.collect(&SectionInput {
        target: ContextTarget::Async,
        history: &history,
        transcript: &transcript,
    });

    assert_eq!(
        sync_sections,
        Ok(vec![
            section("root", /*history_len*/ 1),
            section("permissions", /*history_len*/ 1),
            section("action", /*history_len*/ 1),
        ])
    );
    assert_eq!(
        async_sections,
        Ok(vec![
            section("root", /*history_len*/ 1),
            section("reviews", /*history_len*/ 1),
            section("action", /*history_len*/ 1),
        ])
    );
    assert_eq!(
        invocations
            .iter()
            .map(|calls| calls.load(Ordering::Relaxed))
            .collect::<Vec<_>>(),
        vec![2, 1, 1, 2]
    );
}

#[test]
fn registry_skips_optional_sections_and_stops_on_missing_required_evidence() {
    let error = SectionError::MissingRequiredEvidence {
        section: "permissions",
    };
    for target in [ContextTarget::Sync, ContextTarget::Async] {
        let transcript = transcript_config();
        let mut registry = SectionRegistry::default();
        let mut invocations = Vec::new();
        for outcome in [
            Ok(Some("root")),
            Ok(None),
            Err(error.clone()),
            Ok(Some("action")),
        ] {
            let calls = Arc::new(AtomicUsize::new(/*v*/ 0));
            registry.register(TestContributor {
                outcome,
                scope: SectionScope::Shared,
                invocations: Arc::clone(&calls),
            });
            invocations.push(calls);
        }

        assert_eq!(
            registry.collect(&SectionInput {
                target,
                history: &[ResponseItem::Other],
                transcript: &transcript,
            }),
            Err(error.clone())
        );
        assert_eq!(
            invocations
                .iter()
                .map(|calls| calls.load(Ordering::Relaxed))
                .collect::<Vec<_>>(),
            vec![1, 1, 1, 0]
        );
    }
}
