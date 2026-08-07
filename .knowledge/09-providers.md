# Providers, models, credentials

The layer everyone underestimates. clew's `AGENT.md` documents more hard-won detail here
than anything else in the survey, largely because it supports ~32 providers.

## Capabilities are declared, not assumed

clew's `providers.json`: each provider declares a `capabilities` object —
`chat`, `vision`, `toolCalling`, `streaming` — plus a live `models[]` array, with per-model
`maxContext` as a **static fallback**.

The resolution rule is the interesting part:

> For non-Anthropic providers, the **live `/models` value** (cached by `fetchProviderModels`,
> read synchronously via `getCachedModelContext`) is preferred over `providers.json`'s
> `maxContext`; the static value is the fallback when the live cache is cold.

Why it matters: the context window feeds the auto-compact threshold. A stale static number
means compacting too early (wasting context) or too late (a hard failure mid-turn).

## Model runtime generations (openclaw)

The best answer in the survey to config-change races:

> Gateway startup and config, plugin, or auth publication build **one prepared model runtime
> generation per configured agent**. Each generation owns the discovered auth template, model
> registry, and projected model catalog as **one atomic snapshot**. Agent runs fork mutable
> auth and registry stores from that snapshot; browse, status, cron, doctor, TUI, PDF, and
> image paths read the published catalog instead of repeating filesystem discovery.

And the invariant:

> A failed or stale generation is **never served alongside a newer partial generation**; the
> lifecycle owner must publish a complete replacement first.

Two wins: no repeated filesystem discovery on every surface, and no half-applied config.

## Process-global session state is a trap (clew)

The most specific warning in clew's docs, with a "do not fix this" attached:

- `/model` is **session-scoped** by default — it sets AppState's `mainLoopModelForSession`,
  bridged to `setMainLoopModelOverride()`. Only an explicit "set as default" writes to user
  settings.
- **Do not call `ProviderManager.setSessionModel` / `setSessionProvider`** from a model path:
  it is a process-global singleton and **leaks into subagents and background tasks**.
- Consequently, **mid-retry fallback is same-provider only**. `resolveNextFallback` skips
  entries pinned to another provider because switching providers requires the global setter.
  Cross-provider fallback entries are configurable but only take effect from the next query.
  *"Do not 'fix' this by calling `setSessionProvider` in the retry loop."*

Generalized: **per-session state stored in a process-global is a bug that presents as
mysterious behavior in subagents.** openclaw's generations are the structural fix — fork a
snapshot per run instead of mutating a singleton.

## Fallback chains and routing (clew)

- **Fallback chain** — an ordered list of `{provider?, model, effort?}`. On repeated transient
  capacity errors, `withRetry` consumes the next entry and throws
  `FallbackTriggeredError(model, nextModel, effort)`; the query layer swaps the model, applies
  the entry's effort, advances the cursor, and retries.
- **Task-mode router** — maps a task mode to a model/effort. The mode is *inferred from the
  permission mode* (`plan`→plan, `bypassPermissions`/`auto`→orchestrator,
  `default`/`acceptEdits`→code, `ask`→ask, `dontAsk`→debug) rather than having a separate
  mode UI, explicitly "to keep nothing in sync." An explicit session `/model` override always
  wins, and routed models still pass `isModelAllowed()`.

Deriving one setting from another instead of adding a parallel control is a good pattern —
one less thing to keep consistent.

## Usage and rate limits are provider-shaped

clew normalizes two very different sources into one `Utilization` shape:

- **Anthropic** — an OAuth usage endpoint.
- **Codex/ChatGPT** — rate limits captured **off live `/responses` traffic** via defensive
  header / `rate_limits` parsing, snapshot-only, never probing.

Both feed custom statusline scripts under different keys. The general lesson: usage data is
not a standard API — some providers only tell you in response headers, so the transport layer
has to capture it opportunistically and the model layer has to normalize.

## Provider layer as a library

kimi-cli extracted theirs into **`kosong`**, published separately: "an LLM abstraction layer
… unifies message structures, asynchronous tool orchestration, and pluggable chat providers
so you can build agents with ease and avoid vendor lock-in."

openclaw similarly has `llm-core`, `model-catalog-core`, `ai`, and `normalization-core` as
distinct packages. prime-agent's `packages/ai` holds `api-registry.ts`, `models.ts`,
`models.generated.ts`, `oauth.ts`, `stream.ts`, `cache-pricing.ts`, `bedrock-provider.ts`,
`env-api-keys.ts`, `session-resources.ts`.

Note `cache-pricing.ts` and `models.generated.ts` — prompt-cache pricing is a first-class
concern (see [03-compaction](03-compaction.md)), and the model catalogue is **generated**
rather than hand-maintained.

## Auth surfaces

- OAuth is now normal, not just API keys: prime-agent `/login` with subscription or API-key
  providers; MiMo supports Xiaomi OAuth, Codex/ChatGPT OAuth, xAI OAuth, **and importing
  Claude Code's existing authentication in one step**; grok opens a browser on first launch.
- codex has `keyring-store`, `login`, `aws-auth`, `chatgpt`, `secrets`.
- Secrets get dedicated modules: codex `secrets`, grok `xai-grok-secrets`, hermes
  `credential_pool.py` / `credential_sources.py` / `secret_scope.py` / `redact.py`.

**Importing another agent's credentials is an established, expected move** — which makes
reading clew's credential files a normal design, not a hack.

## For cust

- Reuse clew's credentials read-only: `~/.clew/.credentials.json`, `~/.clew/provider.json`,
  project `.env`. Never write to clew's files. Fail with a message naming the file we looked
  in rather than falling back silently.
- Declare capabilities per model; prefer live `/models` over the static table.
- Publish an **atomic generation**; each run forks from the snapshot. No process-global
  session model or provider — ever.
- Generate the model catalogue rather than hand-maintaining it.
- Capture usage/rate-limit data opportunistically from live traffic; normalize to one shape;
  never probe.
