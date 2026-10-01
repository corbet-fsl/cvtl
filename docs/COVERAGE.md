# Coverage contract

CI targets 100% of reachable production lines and branches. Stable Rust runs
the real native server checks. Nightly Rust is used only for LLVM branch
instrumentation, which currently requires it. Both jobs use the same resolved
Cargo.lock snapshot; all build and test commands after resolution use --locked.

The coverage job executes real tests with cargo-llvm-cov and retains the raw JSON
even when the gate fails. The checker compares integer covered/total counts for
both metrics; rounded percentages cannot pass. An empty report cannot pass.
The report excludes integration-test harness files under tests/, not production
code. No production source exclusions are currently approved.

A failing gate is missing evidence, not permission to lower the threshold or
change domain behavior. Add meaningful failure and round-trip tests. Document
any genuinely unreachable defensive branch precisely before excluding it. This server adapter makes no browser coverage claim.

First-party dependencies follow main. Their resolved full revisions remain in
Cargo.lock, with exactly one source per first-party crate. Dependabot maintains
committed snapshots; CI refreshes once per run and retains the tested snapshot.
Auto-merge requires protected main and successful substantive checks on the
exact current Dependabot head. It never executes PR code with write permissions.

The strict gate reads every DA and BRDA production source record emitted by
upstream LLVM LCOV from the same test execution. Each must have positive hits;
there are no production exclusions. Raw JSON and the original JSON diagnostic
checker remain available. This is source coverage, not coverage of every generic
instantiation; LLVM summary counters can retain an uncovered instantiation even
when emitted source records have both outcomes. An empty line report refuses.
A facade with no instrumentable branches still requires every emitted line.

The shared policy checker also requires the same raw JSON file inventory,
complete branch-location inventory and matching upstream summary metadata.
Truncated, duplicate or malformed LCOV records fail before coverage is counted.
