# Agent Reasoning Translation (TUI)

This fork adds an optional external translation hook that can translate the TUI-only “Agent Reasoning / Thinking” blocks into Chinese.

Key properties:

- Translation is a **display-only** feature in the TUI.
- The translated text is **not** fed back into the model context/history.
- Translation is performed by running a **user-supplied external command**.

## Enable

Add to `~/.codex/config.toml`:

```toml
[plugins.translation.agent_reasoning]
command = ["python", "C:/path/to/translate-agent-reasoning.py"]

# Optional:
timeout_ms = 2000        # per translation command invocation
ui_max_wait_ms = 5000    # keep translated block adjacent to reasoning block
```

Legacy config (deprecated, still supported):

```toml
[translation.agent_reasoning]
command = ["python", "C:/path/to/translate-agent-reasoning.py"]
```

Do not set both legacy and new config in the same scope (global or the same profile).

## Wire Protocol (stdin/stdout JSON)

Your command reads a single JSON object from `stdin`:

```json
{
  "schema_version": 1,
  "kind": "agent_reasoning_title" | "agent_reasoning_body",
  "format": "plain" | "markdown",
  "source_language": "en",
  "target_language": "zh-CN",
  "text": "..."
}
```

It must write a single JSON object to `stdout`:

```json
{ "schema_version": 1, "text": "..." }
```

Notes:

- `kind=agent_reasoning_body` will be rendered as Markdown in the TUI.
- If your command exits non-zero, the TUI will show a small error block under the reasoning block.

## Example Script

See `codex-rs/scripts/translate-agent-reasoning-example.py`.

