# Reciprocal

**A local-first agent orchestration workspace for autoregressive (AR) and language-diffusion models.**

Reciprocal runs real model requests through configured provider adapters. It is not a simulator or benchmarking suite.

## Windows application

The Windows release is a standalone executable. On launch it binds a local-only HTTP server to an available loopback port and opens the workspace in the default browser. The browser is the GUI; no remote server is required.

## Provider separation

Providers are registered under one architecture family:

- **Autoregressive (AR):** sequential next-token generation agents. Codex CLI is available only in this family.
- **Language diffusion:** models that generate/refine text through diffusion-style iterative denoising. They are registered separately and never appear in AR selectors.

The application rejects a Codex CLI registration as a diffusion provider and checks the selected provider family again when a task is executed. For generic OpenAI-compatible endpoints, the user must classify the specific model correctly; the application does not guess architecture from a vendor name.

## Supported workflows

- AR agent
- Language-diffusion agent
- AR → diffusion handoff
- Diffusion → AR handoff
- Parallel AR + diffusion execution

Each stage receives the task and, for sequential workflows, the preceding stage's output. Parallel execution returns both agent outputs without pretending to rank or compare them.

## Connections

- **OpenAI-compatible API:** configure endpoint, model ID, and API key. This can connect to OpenRouter and compatible providers. Diffusion providers must expose a compatible chat-completions interface; other diffusion APIs need a dedicated adapter.
- **Codex CLI:** uses the locally installed and authenticated `codex` command. AR family only.

API keys are stored through the operating system credential manager, not written to the provider JSON file. Provider metadata is stored in the user's application configuration directory.

## Build from source

```bash
cargo run
```

The application opens the browser automatically.

## Windows release build

```powershell
cargo build --release
```

The executable is at `target/release/reciprocal.exe`. The `static/` directory must be available beside the executable in this initial source build.

## API

- `GET /api/health`
- `GET /api/providers`
- `POST /api/providers`
- `DELETE /api/providers/:id`
- `POST /api/tasks`

## Current limitations

- Diffusion adapters currently require an OpenAI-compatible chat-completions endpoint. A provider-specific adapter interface is the next extension for native diffusion APIs.
- Codex CLI must already be installed and authenticated on the machine.
- The first version does not include long-term task history, team accounts, or cloud synchronization.

MIT License.
