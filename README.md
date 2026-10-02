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

### Purpose

Storage that forgets: one expiring storage port for the forum, where every write requires an expiry.

### Owns

- Selecting either the Valkey-backed child (cvlk) or the in-memory child (cmmr) at service startup and routing a single scoped storage port to it, using the supplied connection, scope, limits, and clock.
- Serving as the forum's only storage path, with one connection for the service.

### Never

- Adds durable database engines, credential stores, domain admission decisions, an eviction policy that revives expired presence, or more than one storage connection.
- Falls back to durable storage when a backend refuses an operation.
- Owns domain queries or record lifetimes; those stay with the presence, lookup, and room libraries.

### States

Derived from the selected child: Closed, Ready (naming the active backend), or Unavailable.

### Test obligations

- The same presence, lookup, and throttle suite runs unchanged against both backends.
- A write without an expiry is impossible at the port.
- A backend refusal reaches the caller without fallback to another store.

## Continuous verification

Dependency updates follow main and are tested against one CI-resolved lockfile.
Line and branch coverage target 100%; failures remain blocking. See
[the coverage contract](docs/COVERAGE.md) for measurement and exclusions.
