# Reciprocal

A Rust research workbench for comparing orchestration strategies across autoregressive (AR) and language-diffusion models.

## Research question

When, and under what controlled conditions, does coordinating AR and diffusion models improve task quality enough to justify additional latency and cost?

Reciprocal deliberately does **not** hard-code roles such as “AR reasons, diffusion rewrites.” It compares six execution strategies:

- `ar_only`
- `diffusion_only`
- `ar_then_diffusion`
- `diffusion_then_ar`
- `parallel`
- `hierarchical`

## Important status note

The first version is an orchestration simulator and API scaffold. Its outputs are labelled simulated and must not be interpreted as empirical model performance. Provider adapters, reproducible benchmark suites, and durable experiment storage are planned extensions.

## Run

Requires stable Rust and Cargo.

```bash
cargo run
```

Open http://127.0.0.1:3000

## API

- `GET /api/health`
- `GET /api/strategies`
- `GET /api/runs`
- `POST /api/runs`

Example:

```bash
curl -X POST http://127.0.0.1:3000/api/runs \
  -H 'content-type: application/json' \
  -d '{"task":"Compare two system designs","strategy":"parallel","repetitions":1}'
```

## Experimental discipline

For valid comparisons, hold constant or report the task set, prompt, model/version, decoding settings, hardware, evaluator rubric, and repetition count.

Report at least:

- task quality and the evaluator/rubric used;
- end-to-end latency;
- model and orchestration cost;
- correction/rework cycles;
- interaction gain against the strongest single-family baseline, together with added cost and latency.

Do not claim a hybrid strategy is better from quality alone if its resource use is materially higher.

## License

MIT
