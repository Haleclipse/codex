use std::collections::HashMap;
use std::collections::VecDeque;
use std::time::Duration;
use std::time::Instant;

use codex_core::config::types::AgentReasoningTranslationConfig;
use codex_core::config::types::DEFAULT_AGENT_REASONING_TRANSLATION_UI_MAX_WAIT_MS;
use codex_protocol::ThreadId;

use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;
use crate::history_cell;
use crate::history_cell::HistoryCell;
use crate::tui::FrameRequester;

const AGENT_REASONING_TRANSLATION_MAX_WAIT_ENV: &str =
    "CODEX_TUI_AGENT_REASONING_TRANSLATION_MAX_WAIT_MS";

#[derive(Debug)]
struct AgentReasoningBodyTranslationBarrier {
    request_id: u64,
    thread_id: ThreadId,
    title: Option<String>,
    max_wait: Duration,
    deadline: Instant,
}

#[derive(Debug)]
pub(super) struct AgentReasoningBodyTranslationResult {
    request_id: u64,
    thread_id: ThreadId,
    title: Option<String>,
    translated: Option<String>,
    error: Option<String>,
}

impl AgentReasoningBodyTranslationResult {
    pub(super) fn new(
        request_id: u64,
        thread_id: ThreadId,
        title: Option<String>,
        translated: Option<String>,
        error: Option<String>,
    ) -> Self {
        Self {
            request_id,
            thread_id,
            title,
            translated,
            error,
        }
    }
}

#[derive(Debug)]
pub(crate) struct AgentReasoningTranslationOrchestrator {
    enabled: bool,
    title_translation_cache: HashMap<String, String>,
    current_reasoning_title_raw: Option<String>,
    body_translation_barrier: Option<AgentReasoningBodyTranslationBarrier>,
    deferred_history_cells: VecDeque<Box<dyn HistoryCell>>,
    body_translation_seq: u64,
    body_translation_results_tx:
        tokio::sync::mpsc::UnboundedSender<AgentReasoningBodyTranslationResult>,
    body_translation_results_rx:
        tokio::sync::mpsc::UnboundedReceiver<AgentReasoningBodyTranslationResult>,
}

pub(crate) struct OnBodyTranslatedResult {
    pub(crate) status_header_update: Option<String>,
    pub(crate) needs_redraw: bool,
}

impl Default for AgentReasoningTranslationOrchestrator {
    fn default() -> Self {
        Self::new(true)
    }
}

impl AgentReasoningTranslationOrchestrator {
    pub(crate) fn new(enabled: bool) -> Self {
        let (body_translation_results_tx, body_translation_results_rx) =
            tokio::sync::mpsc::unbounded_channel();
        Self {
            enabled,
            title_translation_cache: HashMap::new(),
            current_reasoning_title_raw: None,
            body_translation_barrier: None,
            deferred_history_cells: VecDeque::new(),
            body_translation_seq: 0,
            body_translation_results_tx,
            body_translation_results_rx,
        }
    }

    pub(crate) fn maybe_status_header_from_reasoning_buffer(
        &mut self,
        reasoning_buffer: &str,
    ) -> Option<String> {
        let header = super::extract_first_bold(reasoning_buffer)?;
        self.current_reasoning_title_raw = Some(header.clone());

        if let Some(translated) = self.title_translation_cache.get(&header) {
            Some(codex_core::translation::format_bilingual_title(
                &header, translated,
            ))
        } else {
            Some(header)
        }
    }

    pub(crate) fn maybe_translate_reasoning_body(
        &mut self,
        config: Option<&AgentReasoningTranslationConfig>,
        thread_id: Option<ThreadId>,
        full_reasoning: String,
        frame_requester: FrameRequester,
    ) {
        if !self.enabled {
            return;
        }
        let Some(config) = config.cloned() else {
            return;
        };
        let Some(thread_id) = thread_id else {
            return;
        };

        let title = super::extract_first_bold(&full_reasoning);
        let Some(body) = extract_reasoning_body_for_translation(&full_reasoning) else {
            return;
        };
        if body.trim().is_empty() {
            return;
        }

        let Some(request_id) = self.begin_body_translation_barrier(
            config.ui_max_wait,
            thread_id,
            title.clone(),
            frame_requester.clone(),
        ) else {
            return;
        };

        let result_tx = self.body_translation_results_tx.clone();
        tokio::spawn(async move {
            let result = codex_core::translation::translate_text(
                &config,
                codex_core::translation::TranslationKind::AgentReasoningBody,
                &full_reasoning,
            )
            .await;

            let msg = match result {
                Ok(translated) => AgentReasoningBodyTranslationResult::new(
                    request_id,
                    thread_id,
                    title,
                    Some(translated),
                    None,
                ),
                Err(err) => AgentReasoningBodyTranslationResult::new(
                    request_id,
                    thread_id,
                    title,
                    None,
                    Some(err.to_string()),
                ),
            };

            let _ = result_tx.send(msg);
            frame_requester.schedule_frame();
        });
    }

    pub(crate) fn drain_body_translation_results(
        &mut self,
        active_thread_id: Option<ThreadId>,
        config: Option<&AgentReasoningTranslationConfig>,
        app_event_tx: &AppEventSender,
        frame_requester: FrameRequester,
    ) -> OnBodyTranslatedResult {
        if !self.enabled {
            return OnBodyTranslatedResult {
                status_header_update: None,
                needs_redraw: false,
            };
        }
        let mut out = OnBodyTranslatedResult {
            status_header_update: None,
            needs_redraw: false,
        };

        loop {
            match self.body_translation_results_rx.try_recv() {
                Ok(msg) => {
                    let result = self.on_body_translated(
                        msg,
                        active_thread_id,
                        config,
                        app_event_tx,
                        frame_requester.clone(),
                    );
                    if result.status_header_update.is_some() {
                        out.status_header_update = result.status_header_update;
                    }
                    out.needs_redraw |= result.needs_redraw;
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => break,
            }
        }

        out
    }

    pub(crate) fn on_body_translated(
        &mut self,
        msg: AgentReasoningBodyTranslationResult,
        active_thread_id: Option<ThreadId>,
        config: Option<&AgentReasoningTranslationConfig>,
        app_event_tx: &AppEventSender,
        frame_requester: FrameRequester,
    ) -> OnBodyTranslatedResult {
        let AgentReasoningBodyTranslationResult {
            request_id,
            thread_id,
            title,
            translated,
            error,
        } = msg;

        let Some(barrier) = self.body_translation_barrier.as_ref() else {
            return OnBodyTranslatedResult {
                status_header_update: None,
                needs_redraw: false,
            };
        };
        if barrier.request_id != request_id {
            return OnBodyTranslatedResult {
                status_header_update: None,
                needs_redraw: false,
            };
        }
        if barrier.thread_id != thread_id {
            return OnBodyTranslatedResult {
                status_header_update: None,
                needs_redraw: false,
            };
        }
        if active_thread_id.as_ref() != Some(&thread_id) {
            return OnBodyTranslatedResult {
                status_header_update: None,
                needs_redraw: false,
            };
        }

        self.body_translation_barrier = None;

        let mut status_header_update = None;

        if let Some(translated) = translated {
            let translated_title = super::extract_first_bold(&translated);
            let translated_body = extract_reasoning_body_for_translation(&translated)
                .unwrap_or_else(|| translated.clone())
                .trim()
                .to_string();

            if let (Some(original), Some(translated_title)) =
                (title.as_deref(), translated_title.as_deref())
            {
                self.title_translation_cache
                    .insert(original.to_string(), translated_title.to_string());

                if self.current_reasoning_title_raw.as_deref() == Some(original) {
                    status_header_update = Some(codex_core::translation::format_bilingual_title(
                        original,
                        translated_title,
                    ));
                }
            }

            self.emit_history_cell(
                app_event_tx,
                history_cell::new_agent_reasoning_translation_block(
                    None,
                    if translated_body.is_empty() {
                        translated
                    } else {
                        translated_body
                    },
                ),
            );
        } else {
            let reason = error.unwrap_or_else(|| "unknown error".to_string());
            self.emit_history_cell(
                app_event_tx,
                history_cell::new_agent_reasoning_translation_error_block(title, reason),
            );
        }

        self.flush_deferred_history_cells(config, active_thread_id, app_event_tx, frame_requester);

        OnBodyTranslatedResult {
            status_header_update,
            needs_redraw: true,
        }
    }

    pub(crate) fn maybe_flush_timeout(
        &mut self,
        config: Option<&AgentReasoningTranslationConfig>,
        active_thread_id: Option<ThreadId>,
        app_event_tx: &AppEventSender,
        frame_requester: FrameRequester,
    ) -> bool {
        if !self.enabled {
            return false;
        }
        let Some(barrier) = self.body_translation_barrier.as_ref() else {
            return false;
        };
        if Instant::now() < barrier.deadline {
            return false;
        }

        let title = barrier.title.clone();
        let max_wait = barrier.max_wait;
        let max_wait_ms = max_wait.as_millis();

        self.body_translation_barrier = None;
        self.emit_history_cell(
            app_event_tx,
            history_cell::new_agent_reasoning_translation_error_block(
                title,
                format!("waiting timed out ({max_wait_ms}ms); skipped translation output"),
            ),
        );
        self.flush_deferred_history_cells(config, active_thread_id, app_event_tx, frame_requester);
        true
    }

    pub(crate) fn emit_history_cell(
        &mut self,
        app_event_tx: &AppEventSender,
        cell: Box<dyn HistoryCell>,
    ) {
        if self.body_translation_barrier.is_some() {
            self.deferred_history_cells.push_back(cell);
        } else {
            app_event_tx.send(AppEvent::InsertHistoryCell(cell));
        }
    }

    pub(crate) fn emit_history_cell_with_translation_hook(
        &mut self,
        app_event_tx: &AppEventSender,
        config: Option<&AgentReasoningTranslationConfig>,
        active_thread_id: Option<ThreadId>,
        frame_requester: FrameRequester,
        cell: Box<dyn HistoryCell>,
    ) {
        if self.body_translation_barrier.is_some() {
            self.deferred_history_cells.push_back(cell);
            return;
        }

        let maybe_reasoning_for_translation = cell
            .as_any()
            .downcast_ref::<history_cell::ReasoningSummaryCell>()
            .and_then(history_cell::ReasoningSummaryCell::full_markdown_for_translation);

        app_event_tx.send(AppEvent::InsertHistoryCell(cell));

        if let Some(full_reasoning) = maybe_reasoning_for_translation {
            self.maybe_translate_reasoning_body(
                config,
                active_thread_id,
                full_reasoning,
                frame_requester,
            );
        }
    }

    pub(crate) fn on_draw_tick(
        &mut self,
        active_thread_id: Option<ThreadId>,
        config: Option<&AgentReasoningTranslationConfig>,
        app_event_tx: &AppEventSender,
        frame_requester: FrameRequester,
    ) -> OnBodyTranslatedResult {
        if !self.enabled {
            return OnBodyTranslatedResult {
                status_header_update: None,
                needs_redraw: false,
            };
        }
        let mut result = self.drain_body_translation_results(
            active_thread_id,
            config,
            app_event_tx,
            frame_requester.clone(),
        );
        if self.maybe_flush_timeout(config, active_thread_id, app_event_tx, frame_requester) {
            result.needs_redraw = true;
        }
        result
    }

    fn flush_deferred_history_cells(
        &mut self,
        config: Option<&AgentReasoningTranslationConfig>,
        active_thread_id: Option<ThreadId>,
        app_event_tx: &AppEventSender,
        frame_requester: FrameRequester,
    ) {
        while let Some(cell) = self.deferred_history_cells.pop_front() {
            let maybe_reasoning_for_translation = cell
                .as_any()
                .downcast_ref::<history_cell::ReasoningSummaryCell>()
                .and_then(history_cell::ReasoningSummaryCell::full_markdown_for_translation);

            app_event_tx.send(AppEvent::InsertHistoryCell(cell));

            if let Some(full_reasoning) = maybe_reasoning_for_translation
                && self.body_translation_barrier.is_none()
            {
                self.maybe_translate_reasoning_body(
                    config,
                    active_thread_id,
                    full_reasoning,
                    frame_requester.clone(),
                );
                if self.body_translation_barrier.is_some() {
                    break;
                }
            }
        }
    }

    fn begin_body_translation_barrier(
        &mut self,
        config_max_wait: Duration,
        thread_id: ThreadId,
        title: Option<String>,
        frame_requester: FrameRequester,
    ) -> Option<u64> {
        if self.body_translation_barrier.is_some() {
            return None;
        }

        let request_id = self.body_translation_seq;
        self.body_translation_seq = self.body_translation_seq.saturating_add(1);

        let max_wait = self.max_wait_with_env_override(config_max_wait);
        let deadline = Instant::now()
            .checked_add(max_wait)
            .unwrap_or_else(Instant::now);
        self.body_translation_barrier = Some(AgentReasoningBodyTranslationBarrier {
            request_id,
            thread_id,
            title,
            max_wait,
            deadline,
        });

        frame_requester.schedule_frame_in(max_wait);
        Some(request_id)
    }

    fn max_wait_with_env_override(&self, config_max_wait: Duration) -> Duration {
        match std::env::var(AGENT_REASONING_TRANSLATION_MAX_WAIT_ENV) {
            Ok(raw) => match raw.trim().parse::<u64>() {
                Ok(ms) => Duration::from_millis(ms),
                Err(err) => {
                    tracing::warn!(
                        "failed to parse {AGENT_REASONING_TRANSLATION_MAX_WAIT_ENV}={raw:?}: {err}; using config value {}ms (default {}ms)",
                        config_max_wait.as_millis(),
                        DEFAULT_AGENT_REASONING_TRANSLATION_UI_MAX_WAIT_MS
                    );
                    config_max_wait
                }
            },
            Err(_) => config_max_wait,
        }
    }
}

pub(super) fn extract_reasoning_body_for_translation(
    full_reasoning_markdown: &str,
) -> Option<String> {
    let full_reasoning_markdown = full_reasoning_markdown.trim();
    let open = full_reasoning_markdown.find("**")?;
    let after_open = &full_reasoning_markdown[(open + 2)..];
    let close = after_open.find("**")?;

    let after_close_idx = open + 2 + close + 2;
    if after_close_idx >= full_reasoning_markdown.len() {
        return None;
    }
    let body = full_reasoning_markdown[after_close_idx..].trim_start();
    if body.is_empty() {
        None
    } else {
        Some(body.to_string())
    }
}

#[cfg(test)]
#[allow(dead_code)]
impl AgentReasoningTranslationOrchestrator {
    pub(super) fn begin_body_translation_barrier_for_tests(
        &mut self,
        config_max_wait: Duration,
        thread_id: ThreadId,
        title: Option<String>,
        frame_requester: FrameRequester,
    ) -> Option<u64> {
        self.begin_body_translation_barrier(config_max_wait, thread_id, title, frame_requester)
    }

    pub(super) fn barrier_max_wait_for_tests(&self) -> Option<Duration> {
        self.body_translation_barrier.as_ref().map(|b| b.max_wait)
    }

    pub(super) fn barrier_request_id_for_tests(&self) -> Option<u64> {
        self.body_translation_barrier.as_ref().map(|b| b.request_id)
    }

    pub(super) fn set_barrier_deadline_for_tests(&mut self, deadline: Instant) {
        if let Some(barrier) = self.body_translation_barrier.as_mut() {
            barrier.deadline = deadline;
        }
    }
}
