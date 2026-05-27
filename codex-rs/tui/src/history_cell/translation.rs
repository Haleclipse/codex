// @cometix: cells for displaying translated reasoning content.

use super::*;

pub(crate) fn new_agent_reasoning_translation_block(
    title: Option<String>,
    translated: String,
) -> Box<dyn HistoryCell> {
    Box::new(AgentReasoningTranslationCell::new(title, translated, false))
}

pub(crate) fn new_agent_reasoning_translation_error_block(
    title: Option<String>,
    reason: String,
) -> Box<dyn HistoryCell> {
    Box::new(AgentReasoningTranslationCell::new(title, reason, true))
}

#[derive(Debug)]
pub(crate) struct AgentReasoningTranslationCell {
    title: Option<String>,
    content: String,
    is_error: bool,
}

impl AgentReasoningTranslationCell {
    pub(crate) fn new(title: Option<String>, content: String, is_error: bool) -> Self {
        Self {
            title,
            content,
            is_error,
        }
    }

    fn lines(&self, width: u16) -> Vec<Line<'static>> {
        let mut md_lines: Vec<Line<'static>> = Vec::new();
        append_markdown(
            &self.content,
            Some((width as usize).saturating_sub(4).max(1)),
            None,
            &mut md_lines,
        );

        let translation_style = Style::default().dim();
        let styled_md_lines = md_lines
            .into_iter()
            .map(|mut line| {
                line.spans = line
                    .spans
                    .into_iter()
                    .map(|span| span.patch_style(translation_style))
                    .collect();
                line
            })
            .collect::<Vec<_>>();

        if self.is_error {
            let mut out: Vec<Line<'static>> = Vec::new();
            let mut header: Vec<Span<'static>> = Vec::new();
            header.push("  └ ".dim());
            header.push("Translation failed".red().bold());
            if let Some(title) = &self.title {
                header.push(" ".into());
                header.push(format!("({title})").dim());
            }
            out.push(Line::from(header));
            out.extend(prefix_lines(styled_md_lines, "    ".into(), "    ".into()));
            return out;
        }

        prefix_lines(styled_md_lines, "  └ ".dim(), "    ".into())
    }
}

impl HistoryCell for AgentReasoningTranslationCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        self.lines(width)
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        self.lines(80)
    }

    fn desired_height(&self, width: u16) -> u16 {
        self.lines(width).len() as u16
    }

    fn transcript_lines(&self, width: u16) -> Vec<Line<'static>> {
        self.lines(width)
    }

    fn desired_transcript_height(&self, width: u16) -> u16 {
        self.lines(width).len() as u16
    }
}
