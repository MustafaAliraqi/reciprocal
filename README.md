# Reciprocal

**Reciprocal is a local workspace for coordinating two different kinds of language agents: LLMs Agents and Diffusion Agents.**

Most agent tools assume that every language model works in roughly the same way. Reciprocal is built around a different idea: autoregressive language models and language-diffusion models have different generation processes, interfaces, and execution characteristics. They should be represented as distinct agent families, while still being able to work on the same task.

Reciprocal gives both families a shared place to receive work and pass results between one another. The goal is not to decide which architecture is better. The goal is to make it possible to compose them without hiding their differences.

## The two agent families

### LLMs Agent

An LLMs Agent represents an autoregressive language model or an agent runtime built around one. Autoregressive generation produces a sequence token by token, conditioning each next token on the preceding context.

Possible connection styles include:

- Hosted model APIs and compatible gateways.
- OpenRouter and other model-routing services.
- Local or self-hosted inference endpoints.
- Agent runtimes and command-line tools, such as Codex CLI.

These are connection options, not a claim that every named service or runtime is already implemented. In this version, the available adapters are Codex CLI and an OpenAI-compatible chat-completions API. OpenRouter can be used through that API adapter.

### Diffusion Agent

A Diffusion Agent represents a language model whose generation process is based on iterative refinement or denoising rather than ordinary left-to-right next-token generation.

This is a separate family in Reciprocal. Diffusion agents have their own registration and selection path; an LLMs Agent connector must not silently appear as a Diffusion Agent connector, or the other way around.

The current API adapter sends chat-completions requests. That transport format alone does **not** make a model a diffusion model. The selected model must genuinely use a language-diffusion generation method. Native diffusion runtimes and provider-specific diffusion APIs require dedicated adapters and are not yet implemented in this release.

## Why keep the families separate?

The separation is functional, not cosmetic:

- Each provider is registered with an explicit architecture family.
- Task configuration selects an LLMs Agent and a Diffusion Agent independently.
- The backend checks the family again when a task runs; a provider cannot be assigned to the other family just by changing a UI selection.
- Runtime/connector type and model architecture are treated as different concepts. For example, an API gateway is a way to reach a model; it does not determine the model's generation architecture.

For generic compatible endpoints, the user currently declares the model family. Reciprocal does not infer architecture from a vendor name, and it does not yet maintain a verified catalogue of every provider and model.

## How agents can work together

A task can be sent through one of the following workflows:

| Workflow | What happens |
| --- | --- |
| LLMs Agent | One autoregressive agent handles the task. |
| Diffusion Agent | One language-diffusion agent handles the task. |
| LLMs → Diffusion | The LLMs Agent produces an output that is passed to the Diffusion Agent. |
| Diffusion → LLMs | The Diffusion Agent produces an output that is passed to the LLMs Agent. |
| Parallel | Both agents receive the task independently; both outputs are returned. |
| Hierarchical | The LLMs Agent plans, the Diffusion Agent works on the task, and the LLMs Agent verifies the result. |

These are execution arrangements, not rankings. Reciprocal does not score one architecture against the other or assume that one family must always plan while the other must always generate. The hierarchical arrangement is simply one available workflow.

## Connections available in this version

| Connector | Agent family | Status |
| --- | --- | --- |
| Codex CLI | LLMs Agent | Implemented; requires the local `codex` command and an authenticated session. |
| OpenAI-compatible chat-completions API | LLMs Agent or Diffusion Agent | Implemented as a transport adapter; the selected model's actual architecture must match the chosen family. |
| OpenRouter | Through the OpenAI-compatible adapter | Usable where the selected model and endpoint support the expected chat-completions interface. |

The connector list is intentionally extensible. Future adapters can include additional hosted APIs, local model servers, agent CLIs, and native language-diffusion runtimes. A connector should only be offered under an agent family when its capabilities and the selected model support that family.

## The application

Reciprocal runs locally. It starts a small HTTP server bound to the loopback interface and opens the GUI in the default browser. The GUI assets are embedded in the application binary, so a separate web server or static web directory is not needed.

- No remote Reciprocal server is required.
- Provider configuration is stored in the operating system's application configuration directory.
- API keys are stored using the operating system credential store through the `keyring` library, rather than being written into the provider configuration file.
- The local web interface is served on `127.0.0.1`; it is not intended to be exposed as a public network service.

## Platforms and builds

The project is configured to build native release binaries for:

- Windows x86-64
- Linux x86-64
- macOS on the architecture provided by the GitHub Actions macOS runner

Every push to `main` runs formatting, tests, and Clippy on Linux, then builds release binaries on Windows, Linux, and macOS. Each successful platform build is uploaded as a separate GitHub Actions artifact.

A successful CI build confirms compilation for that runner. It does not replace testing the application on every supported desktop environment.

### Build locally

Install a stable Rust toolchain, then run:

```sh
cargo run
```

To build an optimized executable:

```sh
cargo build --release
```

The binary is written to `target/release/`:

- Windows: `reciprocal.exe`
- Linux and macOS: `reciprocal`

On first launch, Reciprocal opens its local GUI in the default browser. If the browser does not open automatically, use the local address printed by the application.

## Development

The main code is organized around:

- `domain.rs`: agent families, connector types, workflows, and task data.
- `providers.rs`: provider configuration, credential storage, and connector invocation.
- `orchestrator.rs`: execution of the selected workflow.
- `lib.rs` and `main.rs`: local HTTP API, embedded GUI, and application startup.
- `static/`: browser-based interface.

The HTTP API currently exposes:

- `GET /api/health`
- `GET /api/providers`
- `POST /api/providers`
- `DELETE /api/providers/:id`
- `POST /api/tasks`

## Current scope and next work

This is an early implementation of the orchestration workspace. The current release has a working local GUI, provider configuration, task execution paths, and a Windows release build. The CI workflow is being expanded to build Linux and macOS binaries as well.

Important limitations:

1. The generic API adapter currently speaks OpenAI-compatible `/chat/completions`. It is not a universal adapter for every hosted API.
2. Native language-diffusion inference APIs and local diffusion runtimes still need dedicated adapters. A compatible HTTP endpoint is not proof that the underlying model is diffusion-based.
3. Codex CLI must be installed and authenticated separately on the host machine.
4. Provider architecture is declared by the user for generic endpoints. Automatic model capability discovery and a verified provider/model registry are not implemented yet.
5. Task history, multi-user accounts, and cloud synchronization are outside the current scope.

Contributions that add a provider adapter should document its supported agent family, request/response protocol, authentication method, and whether it has been tested against a real model.

## License

MIT
