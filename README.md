<p align="center"><code>npm i -g @cometix/codex</code></p>
<p align="center"><strong>@cometix/codex</strong> — A customized fork of <a href="https://github.com/openai/codex">OpenAI Codex CLI</a>, the local coding agent that runs in your terminal.</p>
<p align="center">
  <img src="https://github.com/openai/codex/blob/main/.github/codex-cli-splash.png" alt="Codex CLI splash" width="80%" />
</p>

---

## Install

```shell
npm install -g @cometix/codex
```

Then run `codex` to get started.

## Fork Customizations

This fork includes the following enhancements over the upstream [openai/codex](https://github.com/openai/codex):

- **StatusLine (CxLine)** — Bottom status bar displaying model name, reasoning effort, usage percentage, and rate limit reset time
- **Reasoning Translation** — Real-time translation of agent reasoning content via `/translate` command, supporting 15+ LLM providers (OpenAI, Anthropic, DeepSeek, Moonshot, Qwen, Groq, Gemini, etc.)
- **Personality for all models** — The Personality feature is enabled for every model, not limited to select ones
- **CJK block cursor fix** — Correct block cursor width for CJK characters on Windows Terminal
- **Update detection via npm** — Version checking uses the `@cometix/codex` npm registry instead of GitHub releases

## Upstream Sync

This fork is regularly synced with the upstream OpenAI Codex repository. Current base: **0.99.0**.

## Using Codex with your ChatGPT plan

Run `codex` and select **Sign in with ChatGPT**. We recommend signing into your ChatGPT account to use Codex as part of your Plus, Pro, Team, Edu, or Enterprise plan. [Learn more about what's included in your ChatGPT plan](https://help.openai.com/en/articles/11369540-codex-in-chatgpt).

You can also use Codex with an API key, but this requires [additional setup](https://developers.openai.com/codex/auth#sign-in-with-an-api-key).

## Docs

- [**Codex Documentation**](https://developers.openai.com/codex)
- [**Upstream Repository**](https://github.com/openai/codex)

This repository is licensed under the [Apache-2.0 License](LICENSE).
