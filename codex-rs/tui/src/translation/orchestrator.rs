//! Agent reasoning translation orchestrator.
//!
//! This module implements a barrier mechanism to ensure translation results
//! appear immediately after their corresponding reasoning content in the UI.

use std::collections::VecDeque;
use std::time::Duration;
use std::time::Instant;

use codex_protocol::ThreadId;

use super::client::TranslationClient;
use super::config::TranslationConfig;
use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;
use crate::history_cell;
use crate::history_cell::HistoryCell;
use crate::tui::FrameRequester;

/// Default maximum wait time for translation (in milliseconds).
const DEFAULT_TRANSLATION_MAX_WAIT_MS: u64 = 5000;

/// Environment variable to override the max wait time.
const TRANSLATION_MAX_WAIT_ENV: &str = "CODEX_TUI_TRANSLATION_MAX_WAIT_MS";

#[derive(Debug)]
struct TranslationBarrier {
    request_id: u64,
    thread_id: ThreadId,
    /// Original title for timeout error display.
    title: Option<String>,
    max_wait: Duration,
    deadline: Instant,
}

#[derive(Debug)]
pub(super) struct TranslationResult {
    request_id: u64,
    thread_id: ThreadId,
    /// Original title (e.g., "Thinking") for error display.
    title: Option<String>,
    translated: Option<String>,
    error: Option<String>,
}

impl TranslationResult {
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
pub(crate) struct ReasoningTranslator {
    enabled: bool,
    /// Translation configuration.
    config: TranslationConfig,
    /// Barrier for aligning translation with original content.
    translation_barrier: Option<TranslationBarrier>,
    /// History cells deferred during barrier period.
    deferred_history_cells: VecDeque<Box<dyn HistoryCell>>,
    /// Sequence number for binding async results to current barrier.
    translation_seq: u64,
    /// Channel for receiving translation results.
    results_tx: tokio::sync::mpsc::UnboundedSender<TranslationResult>,
    results_rx: tokio::sync::mpsc::UnboundedReceiver<TranslationResult>,
}

pub(crate) struct OnTranslationResult {
    pub(crate) needs_redraw: bool,
}

impl Default for ReasoningTranslator {
    fn default() -> Self {
        // Default to disabled, will be enabled when translation config is set
        Self::from_config(TranslationConfig::default())
    }
}

impl ReasoningTranslator {
    #[allow(dead_code)]
    pub(crate) fn new(enabled: bool) -> Self {
        Self::from_config(TranslationConfig {
            enabled,
            ..Default::default()
        })
    }

    /// Create from configuration.
    pub(crate) fn from_config(config: TranslationConfig) -> Self {
        let (results_tx, results_rx) = tokio::sync::mpsc::unbounded_channel();
        let enabled = config.enabled;
        Self {
            enabled,
            config,
            translation_barrier: None,
            deferred_history_cells: VecDeque::new(),
            translation_seq: 0,
            results_tx,
            results_rx,
        }
    }

    /// Update configuration.
    pub(crate) fn update_config(&mut self, config: TranslationConfig) {
        self.enabled = config.enabled;
        self.config = config;
    }

    /// Get current configuration.
    #[allow(dead_code)]
    pub(crate) fn config(&self) -> &TranslationConfig {
        &self.config
    }

    /// Set whether translation is enabled.
    #[allow(dead_code)]
    pub(crate) fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.config.enabled = enabled;
    }

    /// Returns whether translation is enabled.
    #[allow(dead_code)]
    pub(crate) fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Start translation for reasoning content.
    /// Returns true if translation was started.
    pub(crate) fn maybe_translate_reasoning(
        &mut self,
        thread_id: Option<ThreadId>,
        full_reasoning: String,
        frame_requester: FrameRequester,
    ) -> bool {
        if !self.enabled {
            return false;
        }
        let Some(thread_id) = thread_id else {
            return false;
        };

        // Extract title (e.g., "Thinking") for error display
        let title = extract_first_bold(&full_reasoning);

        // Extract body for translation (skip the **title**)
        let Some(body) = extract_reasoning_body(&full_reasoning) else {
            return false;
        };
        if body.trim().is_empty() {
            return false;
        }

        // Begin barrier to ensure translation follows original content
        let Some(request_id) =
            self.begin_barrier(thread_id, title.clone(), frame_requester.clone())
        else {
            return false;
        };

        let result_tx = self.results_tx.clone();
        let config = self.config.clone();
        // Translate the full reasoning (header + body) so translator can produce bilingual output
        let full_reasoning_owned = full_reasoning;

        // Spawn async translation task
        tokio::spawn(async move {
            let result = Self::do_translate(&config, &full_reasoning_owned).await;

            let msg = match result {
                Ok(translated) => {
                    TranslationResult::new(request_id, thread_id, title, Some(translated), None)
                }
                Err(e) => {
                    TranslationResult::new(request_id, thread_id, title, None, Some(e.to_string()))
                }
            };

            let _ = result_tx.send(msg);
            frame_requester.schedule_frame();
        });

        true
    }

    /// Perform the actual translation.
    async fn do_translate(
        config: &TranslationConfig,
        text: &str,
    ) -> Result<String, super::error::TranslationError> {
        let client = TranslationClient::from_config(config)?;
        client.translate(text, &config.target_language).await
    }

    /// Drain pending translation results.
    pub(crate) fn drain_results(
        &mut self,
        active_thread_id: Option<ThreadId>,
        app_event_tx: &AppEventSender,
        frame_requester: FrameRequester,
    ) -> OnTranslationResult {
        if !self.enabled {
            return OnTranslationResult {
                needs_redraw: false,
            };
        }

        let mut out = OnTranslationResult {
            needs_redraw: false,
        };

        loop {
            match self.results_rx.try_recv() {
                Ok(msg) => {
                    let result = self.on_translation_completed(
                        msg,
                        active_thread_id,
                        app_event_tx,
                        frame_requester.clone(),
                    );
                    out.needs_redraw |= result.needs_redraw;
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => break,
            }
        }

        out
    }

    fn on_translation_completed(
        &mut self,
        msg: TranslationResult,
        active_thread_id: Option<ThreadId>,
        app_event_tx: &AppEventSender,
        frame_requester: FrameRequester,
    ) -> OnTranslationResult {
        let TranslationResult {
            request_id,
            thread_id,
            title,
            translated,
            error,
        } = msg;

        // Validate barrier is still active and matches
        let Some(barrier) = self.translation_barrier.as_ref() else {
            return OnTranslationResult {
                needs_redraw: false,
            };
        };
        if barrier.request_id != request_id || barrier.thread_id != thread_id {
            return OnTranslationResult {
                needs_redraw: false,
            };
        }
        if active_thread_id.as_ref() != Some(&thread_id) {
            return OnTranslationResult {
                needs_redraw: false,
            };
        }

        // Release barrier before inserting content
        self.translation_barrier = None;

        if let Some(translated) = translated {
            // Extract body for display; translated content already contains the title
            // (e.g., "**思考中**\n内容...")
            let translated_body = extract_reasoning_body(&translated)
                .unwrap_or_else(|| translated.clone())
                .trim()
                .to_string();

            self.emit_history_cell(
                app_event_tx,
                history_cell::new_agent_reasoning_translation_block(
                    None, // title not needed for success; content already has it
                    if translated_body.is_empty() {
                        translated
                    } else {
                        translated_body
                    },
                ),
            );
        } else {
            let reason = error.unwrap_or_else(|| "unknown error".to_string());
            tracing::warn!(
                title = title.as_deref().unwrap_or("unknown"),
                error = %reason,
                "translation failed"
            );
            self.emit_history_cell(
                app_event_tx,
                history_cell::new_agent_reasoning_translation_error_block(title, reason),
            );
        }

        self.flush_deferred_cells(active_thread_id, app_event_tx, frame_requester);

        OnTranslationResult { needs_redraw: true }
    }

    /// Check and handle timeout.
    pub(crate) fn maybe_flush_timeout(
        &mut self,
        active_thread_id: Option<ThreadId>,
        app_event_tx: &AppEventSender,
        frame_requester: FrameRequester,
    ) -> bool {
        if !self.enabled {
            return false;
        }
        let Some(barrier) = self.translation_barrier.as_ref() else {
            return false;
        };
        if Instant::now() < barrier.deadline {
            return false;
        }

        let title = barrier.title.clone();
        let max_wait_ms = barrier.max_wait.as_millis();

        // Release barrier
        self.translation_barrier = None;

        // Log timeout
        tracing::warn!(
            title = title.as_deref().unwrap_or("unknown"),
            max_wait_ms = %max_wait_ms,
            "translation timeout, barrier released"
        );

        // Insert error block with title
        self.emit_history_cell(
            app_event_tx,
            history_cell::new_agent_reasoning_translation_error_block(
                title,
                format!("Translation timeout ({max_wait_ms}ms)"),
            ),
        );

        self.flush_deferred_cells(active_thread_id, app_event_tx, frame_requester);
        true
    }

    /// Emit a history cell, deferring if barrier is active.
    pub(crate) fn emit_history_cell(
        &mut self,
        app_event_tx: &AppEventSender,
        cell: Box<dyn HistoryCell>,
    ) {
        if self.translation_barrier.is_some() {
            self.deferred_history_cells.push_back(cell);
        } else {
            app_event_tx.send(AppEvent::InsertHistoryCell(cell));
        }
    }

    /// Emit a history cell and potentially start translation.
    pub(crate) fn emit_history_cell_with_translation_hook(
        &mut self,
        app_event_tx: &AppEventSender,
        active_thread_id: Option<ThreadId>,
        frame_requester: FrameRequester,
        cell: Box<dyn HistoryCell>,
    ) {
        if self.translation_barrier.is_some() {
            self.deferred_history_cells.push_back(cell);
            return;
        }

        // Check if this is a reasoning cell that needs translation
        let maybe_reasoning = cell
            .as_any()
            .downcast_ref::<history_cell::ReasoningSummaryCell>()
            .and_then(history_cell::ReasoningSummaryCell::full_markdown_for_translation);

        app_event_tx.send(AppEvent::InsertHistoryCell(cell));

        if let Some(full_reasoning) = maybe_reasoning {
            self.maybe_translate_reasoning(active_thread_id, full_reasoning, frame_requester);
        }
    }

    /// Called on each draw tick to process results and timeouts.
    pub(crate) fn on_draw_tick(
        &mut self,
        active_thread_id: Option<ThreadId>,
        app_event_tx: &AppEventSender,
        frame_requester: FrameRequester,
    ) -> OnTranslationResult {
        if !self.enabled {
            return OnTranslationResult {
                needs_redraw: false,
            };
        }

        let mut result =
            self.drain_results(active_thread_id, app_event_tx, frame_requester.clone());

        if self.maybe_flush_timeout(active_thread_id, app_event_tx, frame_requester) {
            result.needs_redraw = true;
        }

        result
    }

    fn flush_deferred_cells(
        &mut self,
        active_thread_id: Option<ThreadId>,
        app_event_tx: &AppEventSender,
        frame_requester: FrameRequester,
    ) {
        while let Some(cell) = self.deferred_history_cells.pop_front() {
            // Check if this deferred cell is also a reasoning cell
            let maybe_reasoning = cell
                .as_any()
                .downcast_ref::<history_cell::ReasoningSummaryCell>()
                .and_then(history_cell::ReasoningSummaryCell::full_markdown_for_translation);

            app_event_tx.send(AppEvent::InsertHistoryCell(cell));

            // If we encounter another reasoning cell during flush, start its translation
            // and stop flushing to maintain order
            if let Some(full_reasoning) = maybe_reasoning
                && self.translation_barrier.is_none()
            {
                // Use current active_thread_id for translation
                self.maybe_translate_reasoning(
                    active_thread_id,
                    full_reasoning,
                    frame_requester.clone(),
                );
                if self.translation_barrier.is_some() {
                    // New barrier started, stop flushing to maintain order
                    break;
                }
            }
        }
    }

    fn begin_barrier(
        &mut self,
        thread_id: ThreadId,
        title: Option<String>,
        frame_requester: FrameRequester,
    ) -> Option<u64> {
        if self.translation_barrier.is_some() {
            // Only one barrier at a time
            return None;
        }

        let request_id = self.translation_seq;
        self.translation_seq = self.translation_seq.saturating_add(1);

        let max_wait = self.resolve_max_wait();
        let deadline = Instant::now()
            .checked_add(max_wait)
            .unwrap_or_else(Instant::now);

        self.translation_barrier = Some(TranslationBarrier {
            request_id,
            thread_id,
            title,
            max_wait,
            deadline,
        });

        // Schedule a frame for timeout handling
        frame_requester.schedule_frame_in(max_wait);
        Some(request_id)
    }

    /// Resolve max wait duration.
    /// Priority: config.timeout_ms > env var > default (5000ms).
    fn resolve_max_wait(&self) -> Duration {
        // 1. Config file value
        if let Some(ms) = self.config.timeout_ms {
            if ms > 0 {
                return Duration::from_millis(ms);
            }
        }
        // 2. Environment variable
        if let Ok(raw) = std::env::var(TRANSLATION_MAX_WAIT_ENV) {
            if let Ok(ms) = raw.trim().parse::<u64>() {
                return Duration::from_millis(ms);
            }
        }
        // 3. Default
        Duration::from_millis(DEFAULT_TRANSLATION_MAX_WAIT_MS)
    }
}

/// Extract the first bold text (e.g., "Thinking" from "**Thinking**").
fn extract_first_bold(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i + 1 < bytes.len() {
        if bytes[i] == b'*' && bytes[i + 1] == b'*' {
            let start = i + 2;
            let mut j = start;
            while j + 1 < bytes.len() {
                if bytes[j] == b'*' && bytes[j + 1] == b'*' {
                    let inner = &s[start..j];
                    let trimmed = inner.trim();
                    if !trimmed.is_empty() {
                        return Some(trimmed.to_string());
                    } else {
                        break;
                    }
                }
                j += 1;
            }
            i = j + 2;
        } else {
            i += 1;
        }
    }
    None
}

/// Extract reasoning body (content after `**title**`).
fn extract_reasoning_body(full_reasoning: &str) -> Option<String> {
    let full_reasoning = full_reasoning.trim();
    let open = full_reasoning.find("**")?;
    let after_open = &full_reasoning[(open + 2)..];
    let close = after_open.find("**")?;

    let after_close_idx = open + 2 + close + 2;
    if after_close_idx >= full_reasoning.len() {
        return None;
    }
    let body = full_reasoning[after_close_idx..].trim_start();
    if body.is_empty() {
        None
    } else {
        Some(body.to_string())
    }
}
