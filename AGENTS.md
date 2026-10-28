# cvtl agent notes

Work in a lane (`~/agents/scratch/lane-new <lane>`); never edit `~/agents/scratch/ws`.
Run commands from the lane root (`<lane>/ws`); `<lane>` below is your lane name.

Build and test (quote the verdict line, never infer from a pipe):
- `cargo fmt -p cvtl -- --check` — clean at HEAD, exit 0.
- `~/agents/scratch/cbuild <lane> test -p cvtl --locked` — green, exit 0
  (`tests/composition.rs::identical_suite_through_both_facade_variants`;
  needs disposable Valkey at `VALKEY_URL`, as `.ci/verify.sh` spawns).
- `clippy -p cvtl --locked --all-targets -- -D warnings` is red at HEAD from
  pre-existing drift (`tests/composition.rs:168` unused variable `scope`).
  Fix code, never lint. Full gate (coverage) runs in CI only: `.ci/verify.sh`.

Land: `cargo fmt` in touched repos, rebase on main (never force-push),
`CMTY_PRIORITY=background ~/agents/scratch/cbuild <lane> xtask land --dry`,
then `xtask land`.

Invariants (rule -> enforcing test):
- Every write carries a deadline; handles are scoped to one community/service
  -> `identical_suite_through_both_facade_variants` (explicit deadlines,
  per-keyspace scopes; `docs/CONTRACT.md:13-16,20-24`).
- No fallback: a lost reply is `Unavailable`, the backend never switches
  -> same test (`compare_exchange` → `Err(Unavailable)`; `docs/CONTRACT.md:30-36`).
- Warnings never refuse: Valkey persistence state only surfaces via `warnings()`
  -> same test (`warnings() == health.warnings`; `README.md:11-12`).
- Close propagates to clones; a closed facade never revives
  -> same test (`refusal_isolation`, `reopen` after close → `Err(Closed)`;
  `docs/CONTRACT.md:9-10`).
- `check_current` is atomic and never renews: missing/changed/expired entries
  refuse the whole read
  -> same test via the shared cmmr conformance suite (`docs/CONTRACT.md:38`).
- Invalid limits are refused at construction (`records: 0` → `Err(Invalid)`)
  -> same test (`Volatile::memory`/`valkey` constructors).
- `forbid(unsafe_code)` (`src/lib.rs:3`); first-party git deps follow `main`
  with one locked revision each in `Cargo.lock`.

Pointers: `src/lib.rs`, `tests/composition.rs`, `docs/CONTRACT.md`,
`docs/COVERAGE.md`, `.ci/verify.sh`.
Commits: explicit paths, imperative, author per `~/agents/AGENT.md`; no AI trailers.
`publish=false`; FSL-1.1-ALv2; no registry publication.
