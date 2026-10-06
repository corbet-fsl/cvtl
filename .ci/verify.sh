#!/usr/bin/env bash
set -euo pipefail
exec 2>&1
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
export CARGO_TERM_COLOR="${CARGO_TERM_COLOR:-always}"

valkey_owned_ready() {
  [ -n "${VALKEY_PID:-}" ] || { echo "cvtl fixture: no tracked server child" >&2; return 1; }
  kill -0 "$VALKEY_PID" 2>/dev/null || { echo "cvtl fixture: owned server child $VALKEY_PID exited" >&2; return 1; }
  _vor_id="$(valkey-cli -p "$VALKEY_PORT" INFO server 2>/dev/null | awk -F: '$1=="process_id"{print $2}' | tr -d '\r' || true)"
  [ -n "$_vor_id" ] || { echo "cvtl fixture: no INFO process_id on port $VALKEY_PORT" >&2; return 1; }
  [ "$_vor_id" = "$VALKEY_PID" ] || { echo "cvtl fixture: INFO process_id '$_vor_id' != owned child $VALKEY_PID" >&2; return 1; }
}
start_valkey() {
  VALKEY_TMP="$(mktemp -d)"
  VALKEY_PORT="$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])')"
  printf 'port %s\nbind 127.0.0.1\nprotected-mode yes\ndir %s\ndbfilename dump.rdb\nsave ""\nappendonly no\n' "$VALKEY_PORT" "$VALKEY_TMP" > "$VALKEY_TMP/valkey.conf"
  valkey-server "$VALKEY_TMP/valkey.conf" --daemonize no --logfile "$VALKEY_TMP/valkey.log" \
    0</dev/null >>"$VALKEY_TMP/valkey.log" 2>&1 8>&- 9>&- &
  VALKEY_PID="$!"
  for _ in $(seq 1 50); do
    kill -0 "$VALKEY_PID" 2>/dev/null || { echo "cvtl fixture: owned server child $VALKEY_PID exited during readiness" >&2; return 1; }
    if valkey-cli -p "$VALKEY_PORT" ping >/dev/null 2>&1; then break; fi
    sleep 0.2
  done
  valkey_owned_ready || { echo "cvtl fixture: refusing ping from unowned server" >&2; return 1; }
  valkey-cli -p "$VALKEY_PORT" ping
  export VALKEY_URL="redis://127.0.0.1:${VALKEY_PORT}/"
}
stop_valkey() {
  _sv_pid="${VALKEY_PID:-}"
  _sv_tmp="${VALKEY_TMP:-}"
  VALKEY_PID=""; VALKEY_TMP=""; VALKEY_PORT=""; VALKEY_URL=""
  if [ -n "$_sv_pid" ]; then kill "$_sv_pid" 2>/dev/null || true; wait "$_sv_pid" 2>/dev/null || true; fi
  if [ -n "$_sv_tmp" ] && [ -d "$_sv_tmp" ]; then rm -rf "$_sv_tmp"; fi
}
trap stop_valkey EXIT
start_valkey

rustc --version
cargo metadata --locked --all-features --format-version 1 > dependency-metadata.json
python3 .github/check-first-party.py
echo '+STAGE: fmt'
cargo fmt --all
git diff --exit-code -- '*.rs' > format.patch
echo '+STAGE: clippy'
cargo clippy --locked --all-targets -- -D warnings
echo '+STAGE: test'
cargo test --locked -- --nocapture
stop_valkey
start_valkey

python3 .github/check-first-party.py

rustup run nightly rustc --version
NIGHTLY_RUSTC="$(rustup which --toolchain nightly rustc)"
NIGHTLY_CARGO="$(rustup which --toolchain nightly cargo)"
NIGHTLY_BIN="$(dirname "$NIGHTLY_RUSTC")"
export RUSTC="$NIGHTLY_RUSTC"
export CARGO="$NIGHTLY_CARGO"
export PATH="$NIGHTLY_BIN:$PATH"
export RUSTUP_TOOLCHAIN=nightly
"$NIGHTLY_CARGO" llvm-cov --version
# Bug 2 isolation: coverage data must not reuse the shared cargo target
# across jobs (stale checkout paths fail the next gate); scope the
# coverage target to this job only. Native gates keep the shared warm
# target; outer traps and services are untouched.
(
coverage_target="$(mktemp -d)"
case "$coverage_target" in ""|"$PWD"/*) echo 'unsafe coverage target' >&2; exit 2;; esac
trap 'rm -rf -- "$coverage_target"' EXIT
export CARGO_TARGET_DIR="$coverage_target"
"$NIGHTLY_CARGO" llvm-cov --locked --workspace --all-features --branch --no-cfg-coverage --no-cfg-coverage-nightly --ignore-filename-regex '/tests/' --json --output-path coverage.json
"$NIGHTLY_CARGO" llvm-cov report --locked --branch --ignore-filename-regex '/tests/' --lcov --output-path coverage.lcov
"$NIGHTLY_CARGO" llvm-cov report --locked --branch --ignore-filename-regex '/tests/' --text > coverage.txt
)
python3 .github/check-coverage.py coverage.lcov coverage.json coverage.txt
