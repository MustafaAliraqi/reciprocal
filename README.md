# Reciprocal

**Reciprocal is a local-first agent workspace for coordinating autoregressive LLM agents and language-diffusion agents without treating them as the same architecture.**

It combines provider configuration, multi-agent workflows, a bounded workspace tool runtime, and a browser-based local GUI in one native Rust application.

## Agent families stay separate

- **LLMs Agent (autoregressive):** token-by-token language models and runtimes built around them.
- **Diffusion Agent:** language models that generate through iterative denoising or refinement.

The architecture family is an explicit part of every provider configuration and is checked again by the backend when a task runs. A connector's HTTP protocol does not determine the model architecture. For generic endpoints, users declare the model family; Reciprocal does not guess it from a vendor or model name.

## What is implemented

### Provider connections

- OpenAI-compatible `/chat/completions` endpoints.
- OpenRouter and other compatible gateways through that adapter, when the chosen model supports the expected API.
- Codex CLI as an external LLMs Agent runtime. Codex must be installed and authenticated separately; it runs its own agent loop and does not use Reciprocal's workspace tools.
- Provider capability flags are explicit. Workspace tool calling is opt-in and currently limited to OpenAI-compatible providers registered as LLMs Agents.

The OpenAI-compatible adapter is a transport implementation, not a universal provider SDK.

### Workflows

- LLMs Agent only.
- Diffusion Agent only.
- LLMs → Diffusion.
- Diffusion → LLMs.
- Parallel execution.
- Hierarchical execution: LLMs planning → Diffusion work → LLMs verification.

These workflows compose model calls; they do not rank architectures or assume one family is universally better.

### Agent tool runtime

For an eligible LLMs Agent provider, Reciprocal can send function schemas and run a bounded tool-calling loop:

`model → tool call → tool result → model → final response`

The runtime currently exposes:

| Tool | Access |
| --- | --- |
| `workspace_list` | Read |
| `workspace_read` | Read |
| `workspace_search` | Read |
| `git_status` | Read-only |
| `git_diff` | Read-only |
| `workspace_write` | Write; disabled unless the user explicitly opts in for that task |

The loop is capped at eight model/tool rounds to prevent unbounded execution. Tool calls and their results are returned with the task result and shown in the GUI.

Diffusion providers do not receive these function schemas. Codex CLI is also not routed through this runtime because it has its own tool execution system.

### Workspace boundaries and permissions

- The user selects a local workspace root.
- File tools accept relative paths only and reject parent traversal and paths resolving outside the workspace.
- File reads are limited to 2 MiB.
- Search is bounded by file and result limits and skips common generated/vendor directories.
- Writes are disabled by default. A per-task GUI consent checkbox explicitly authorizes the agent to write within the selected workspace for that task.
- Git integration is read-only: status and unstaged diff only.
- Reciprocal does not expose a general-purpose shell or arbitrary terminal command tool.

The workspace APIs are local application capabilities; the agent only receives them when its provider has the tool capability enabled.

## What is not implemented

These are deliberately not represented as existing features:

- Native language-diffusion inference adapters. The current chat-completions transport does not turn a model into a diffusion model.
- Automatic architecture discovery or a verified provider/model catalogue.
- Reciprocal-managed tools for Codex CLI.
- General terminal execution, browser automation, plugin marketplace, vector database/RAG, or benchmark suite.
- Durable task sessions/resume, background task queue, live token streaming, and cooperative cancellation.

The last group is future runtime work. Current task execution is a synchronous request/response operation; task tool events are returned when the request completes.

## Run locally

Install a stable Rust toolchain, then:

```sh
cargo run
```

For an optimized binary:

```sh
cargo build --release
```

The application binds to an available loopback address (`127.0.0.1`) and opens the GUI in the default browser. If the browser does not open, use the local URL printed in the terminal.

The GUI assets are embedded in the executable. No separate frontend server is required.

## Configuration and credentials

- Provider configuration is stored in the operating system's application configuration directory.
- API keys are stored through the operating system credential store using the `keyring` crate, not in `providers.json`.
- Workspace selection is local to the running application instance.
- The HTTP server binds to loopback and is not designed to be exposed to a network.

Existing provider configuration files remain readable: the new tool capability flag defaults to disabled when absent.

## Build targets and CI

GitHub Actions runs formatting, tests, and Clippy on Linux, and builds release binaries on:

- Windows x86-64
- Linux x86-64
- macOS (the architecture supplied by the hosted macOS runner)

Successful builds are uploaded as workflow artifacts. A CI build confirms compilation on its runner; it is not a substitute for interactive testing on every desktop environment.

## Project layout

- `src/domain.rs` — architecture families, workflows, provider and task types.
- `src/providers.rs` — provider configuration, credential access, API calls, and bounded agent tool loop.
- `src/orchestrator.rs` — multi-agent workflow execution.
- `src/workspace.rs` — workspace boundary checks, file operations, and read-only Git inspection.
- `src/lib.rs`, `src/main.rs` — local HTTP API, embedded assets, and startup.
- `static/` — GUI.

## HTTP API

- `GET /api/health`
- `GET /api/providers`
- `POST /api/providers`
- `DELETE /api/providers/:id`
- `POST /api/tasks`
- `GET /api/workspace`
- `POST /api/workspace`
- `GET /api/workspace/files`
- `GET /api/workspace/read`
- `GET /api/workspace/search`

## License

MIT
