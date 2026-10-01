# Agent instructions

Write comments and documentation in English.

## Product boundary

- cvtl belongs to cfrm's volatile storage composition. Follow docs/CONTRACT.md.
- Survey maintained libraries before adding mechanisms; record the choice in README.md.
- Every write requires bounded expiry. No SQL, files, private plaintext, request logs or credentials read from the environment by the library.
- Reuse the shared cmmr contract and conformance suite; facades only wire children.
- Preserve the repository's license and do not add GPL-only or AGPL-only dependencies.

## Quality boundary

- No workstation Cargo commands. GitHub Actions runs formatting, Clippy and tests.
- Use real disposable Valkey in CI; tests need no external accounts.
- First-party Git dependencies follow branch main; preserve exactly one resolved revision per crate in Cargo.lock.
- Commit explicit paths in small imperative English steps; no AI attribution.
- Pull with rebase before every push to main. Never force-push, deploy or publish packages.
