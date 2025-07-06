# Contributing to minikv

Thanks for contributing. This guide is aligned with the v1.0.0 GA workflow.

## Ways to Contribute

- Report bugs with clear reproduction steps.
- Propose roadmap or UX improvements.
- Submit code, tests, docs, and runbook improvements.
- Improve observability, release engineering, and operator workflows.

Issues: https://github.com/whispem/minikv/issues

## Local Setup

```bash
git clone https://github.com/whispem/minikv
cd minikv
make build
```

Recommended checks before opening a PR:

```bash
make test
make fmt
make clippy
make release-preflight
```

## Branch and Commit Workflow

1. Create a branch.

```bash
git checkout -b feat/short-description
```

2. Implement changes with tests.
3. Run formatting/lint/tests.
4. Update docs if behavior or APIs changed.
5. Commit with a clear message.

Examples:

- `feat(timeseries): add tag-filtered query validation`
- `fix(vector): persist index atomically`
- `docs(release): update preflight instructions`

## Pull Request Checklist

- Feature behavior is tested (unit and/or integration).
- `cargo fmt --all` is clean.
- `cargo clippy --all-targets --all-features -- -D warnings` passes.
- Relevant docs updated (`README.md`, `CHANGELOG.md`, runbooks).
- Any API/endpoint changes are documented with examples.

## Testing Expectations

Minimum for most PRs:

```bash
cargo test --lib
```

For API, storage, replication, or release-impacting changes, run:

```bash
make test
make release-preflight
```

For release-critical PRs, also run:

```bash
make release-preflight-full
