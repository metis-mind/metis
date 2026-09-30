# metis

A self-evolving agent runtime: a long-lived Rust core (fiber lifecycle containers, context & event system, service registry, hot-reloading loader) extended by Luau plugins. The model may propose new plugins, but never gets runtime eval — installation goes through an approval-gated transaction.

Status: pre-alpha. The architecture is designed in `docs/` ahead of the first line of runtime code.

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
