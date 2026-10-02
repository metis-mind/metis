# metis

A self-evolving agent runtime: a long-lived Rust core (fiber lifecycle containers, context & event system, service registry, hot-reloading loader) extended by Luau plugins. The model may propose new plugins, but never gets runtime eval — installation goes through an approval-gated transaction.

Status: pre-alpha. The architecture is designed in `docs/` ahead of the first line of runtime code.

## Origins

Metis is inspired by [DeepSeek Harness](https://github.com/deepseek-ai/deepseek-harness) and its plugin runtime [Cordis](https://github.com/cordiverse/cordis), together with the paper _A Programming Paradigm for Spatiotemporal Composability_ ([arXiv 2608.25512](https://arxiv.org/abs/2608.25512)). In many ways Metis is a Rust implementation of the Cordis model — fiber lifecycle containers, context scoping, service registry — but it is not a line-by-line port. The Node-specific machinery is dropped, plugins are Luau instead of TypeScript, every plugin gets its own `lua_State`, and several semantics are redesigned around Rust's ownership and actor isolation. The full analysis lives in [`docs/research/cordis-research.md`](docs/research/cordis-research.md).

## Development

    direnv allow   # or: nix develop (entering the shell also arms the git hooks)
    just ci        # full quality gate

See [CONTRIBUTING.md](./CONTRIBUTING.md).

## Documentation

- `docs/decisions/` — architecture decision records (ADRs)
- `docs/design/` — living design documents
- `docs/research/` — point-in-time research reports (Cordis / DeepSeek Harness)

## License

MIT OR Apache-2.0 — see [LICENSE-MIT](./LICENSE-MIT) and [LICENSE-APACHE](./LICENSE-APACHE).
