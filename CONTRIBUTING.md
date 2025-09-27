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

