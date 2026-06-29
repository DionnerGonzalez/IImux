# llmux

> A developer-first CLI toolkit for working with multiple LLM providers.

```
  llmux  v0.1.0
  ─────────────────────────
  ▸ LLM toolkit for developers
```

Written in Rust because life is too short for slow tools.

---

## What it does

`llmux` gives you a single terminal interface to chat with, benchmark, and compare LLM providers — without switching dashboards, writing boilerplate API clients, or waiting for a web UI to load.

**Supported providers**

| Provider  | Auth       | Notes                         |
|-----------|------------|-------------------------------|
| OpenAI    | API key    | GPT-4o, GPT-4.1, GPT-4o-mini |
| Anthropic | API key    | Claude Sonnet, Haiku, Opus    |
| Gemini    | API key    | Gemini Pro/Flash + Gemma      |
| Ollama    | none       | Local models via `ollama serve` |

---

## Installation

```bash
# Clone and build
git clone https://github.com/your-username/llmux
cd llmux
cargo build --release

# Install to PATH
cargo install --path .
```

**Requirements:** Rust 1.78+ (edition 2021), internet connection for remote providers.

---

## Quick start

```bash
# Configure your API keys once
llmux config set openai   sk-...
llmux config set anthropic sk-ant-...
llmux config set gemini   AIza...

# Start chatting
llmux chat --provider gemini
llmux chat --provider anthropic --model claude-haiku-4-5-20251001

# Use a local model (no key needed)
ollama pull llama3.2
llmux chat --provider ollama --model llama3.2
```

---

## Commands

### `chat` — interactive session

```bash
llmux chat [OPTIONS]

Options:
  -p, --provider <NAME>   openai | anthropic | gemini | ollama  [default: gemini]
  -m, --model <MODEL>     override the default model
  -s, --system <PROMPT>   set a system prompt
      --no-save           don't write this session to history
```

**In-session commands:**

| Command    | What it does                         |
|------------|--------------------------------------|
| `/help`    | show available commands              |
| `/clear`   | clear conversation, keep system prompt |
| `/tokens`  | estimate tokens currently in context |
| `/cost`    | estimate cost of this session so far |
| `/quit`    | end the session                      |
| `Ctrl+D`   | also ends the session                |

---

### `bench` — latency & throughput

Sends the same prompt N times and reports P50/P95 latency and tokens/sec.

```bash
llmux bench "Explain ownership in Rust in one sentence" \
    --providers openai,gemini \
    --runs 5
```

Output:
```
  benchmark · 5 runs per provider
  prompt: Explain ownership in Rust in one sentence

  ✓ OpenAI / gpt-4o
    latency:    avg 1423ms  p50 1389ms  p95 1834ms
    throughput: 31 tokens/sec  (~44 tokens/call)

  ✓ Gemini / gemma-4-31b-it
    latency:    avg 892ms   p50 867ms   p95 1102ms
    throughput: 48 tokens/sec  (~43 tokens/call)
```

---

### `compare` — side-by-side

Fires the same prompt at multiple providers in parallel and shows responses as they come in.

```bash
llmux compare "What's the best async runtime for Rust?" \
    --providers openai,anthropic,gemini,ollama
```

---

### `tokens` — count & cost estimate

```bash
# Analyze a string
llmux tokens "This is the text I want to analyze"

# Analyze a file
llmux tokens --file my_prompt.txt --provider anthropic

# Pipe from stdin
cat system_prompt.md | llmux tokens
```

Output:
```
  token analysis
  tokens:  ~342
  chars:   1371
  words:   248
  lines:   14

  estimated cost (as output tokens)
  OpenAI:     $0.003
  Anthropic:  $0.005
  Gemini:     $0.000
```

---

### `config` — manage keys and preferences

```bash
llmux config set <provider> <key>   # save an API key
llmux config unset <provider>       # remove a key
llmux config default <provider>     # set default provider
llmux config list                   # show configured keys (masked)
llmux config path                   # print config file location
```

Config is stored at `~/.config/llmux/config.toml`.

---

### `stats` — usage history

```bash
llmux stats          # show all-time stats
llmux stats --last 5 # show 5 most recent sessions
```

Session history lives in `~/.local/share/llmux/sessions/`.

---

## Architecture

```
src/
├── main.rs                  # CLI entry point (clap)
├── providers/
│   ├── mod.rs               # LlmProvider trait + ProviderKind enum
│   ├── openai.rs            # OpenAI Chat Completions
│   ├── anthropic.rs         # Anthropic Messages API
│   ├── gemini.rs            # Google Gemini / Gemma
│   └── ollama.rs            # Ollama local server
├── commands/
│   ├── chat.rs              # interactive multi-turn chat
│   ├── bench.rs             # latency benchmarking
│   ├── compare.rs           # parallel side-by-side
│   ├── config.rs            # key management
│   ├── tokens.rs            # token counting + cost
│   └── stats.rs             # session history
├── tui/
│   └── spinner.rs           # terminal spinner
└── utils/
    ├── config.rs            # config file I/O (TOML)
    ├── history.rs           # session persistence (JSON)
    └── tokens.rs            # token estimation + cost tables
```

Adding a new provider:
1. Create `src/providers/myprovider.rs` implementing `LlmProvider`
2. Add a variant to `ProviderKind`
3. Add a match arm in `providers::build()`
4. That's it — all commands pick it up automatically

---

## Configuration file

`~/.config/llmux/config.toml`:

```toml
default_provider = "gemini"

[keys]
openai    = "sk-..."
anthropic = "sk-ant-..."
gemini    = "AIza..."

[models]
openai    = "gpt-4o-mini"
anthropic = "claude-haiku-4-5-20251001"
```

You can also pass keys via environment variables:
```bash
LLMUX_OPENAI_KEY=sk-... llmux chat --provider openai
```
*(env var support is on the roadmap)*

---

## Roadmap

- [ ] Streaming responses (token-by-token output)
- [ ] Full ratatui TUI with split-pane compare view
- [ ] Prompt template library (`llmux prompt list`)
- [ ] Export sessions to Markdown
- [ ] Retry with exponential backoff on rate limit errors
- [ ] Token budget enforcement (`--max-tokens`)
- [ ] OpenAI-compatible passthrough server mode

---

## License

MIT — do whatever you want with it.
