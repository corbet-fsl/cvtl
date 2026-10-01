# cvtl — Volatile

Thin server-side composition of [cmmr](https://github.com/corbet-foss/cmmr)
and [cvlk](https://github.com/corbet-foss/cvlk). Select `Volatile::memory` or
`Volatile::valkey` at startup, supplying a trusted scope, explicit limits and a
clock. Valkey also receives the service's shared network connection. Every
operation delegates to the selected child; errors never trigger fallback.

The API is cmmr's `Store`: atomic record/index CAS, reads, generation deletion,
bounded pages, explicit maintenance and close. Every write requires a deadline.
State is derived from the child. Clones share the same child. Inspect `warnings()`
when selecting Valkey; persistence enabled/unknown is a warning, not refusal.

Candidates surveyed: the existing cmmr/cvlk and cthl ports. Their implementations
already own storage and quota behavior; this facade adds no storage engine,
policy or third-party cache. For distributed throttling, wire the Valkey variant's
shared child to the re-exported `ThrottleStore` and existing `cthl::Throttle`;
for memory services, supply one shared `cthl::MemoryStore`.

[Implemented contract](docs/CONTRACT.md). CI runs the same cmmr conformance suite
through both variants against memory and disposable Valkey. FSL-1.1-ALv2.

## Scope

Select and route the actual Memory or Valkey child at startup. Re-export the shared port and child readiness/warnings without a second store, fallback, identity, quota policy or member lifecycle.

## Continuous verification

Dependency updates follow main and are tested against one CI-resolved lockfile.
Line and branch coverage target 100%; failures remain blocking. See
[the coverage contract](docs/COVERAGE.md) for measurement and exclusions.
