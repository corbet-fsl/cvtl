#!/usr/bin/env bash
set -euo pipefail
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
export CARGO_TERM_COLOR="${CARGO_TERM_COLOR:-always}"

VALKEY_TMP="$(mktemp -d)"
VALKEY_PORT="$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])')"
printf 'port %s\nbind 127.0.0.1\nprotected-mode yes\ndir %s\ndbfilename dump.rdb\nsave ""\nappendonly no\n' "$VALKEY_PORT" "$VALKEY_TMP" > "$VALKEY_TMP/valkey.conf"
valkey-server "$VALKEY_TMP/valkey.conf" --daemonize no --logfile "$VALKEY_TMP/valkey.log" &
VALKEY_PID="$!"
cleanup_valkey() { kill "$VALKEY_PID" 2>/dev/null || true; wait "$VALKEY_PID" 2>/dev/null || true; rm -rf "$VALKEY_TMP"; }
trap cleanup_valkey EXIT
for _ in $(seq 1 50); do
  if valkey-cli -p "$VALKEY_PORT" ping >/dev/null 2>&1; then break; fi
  sleep 0.2
done
valkey-cli -p "$VALKEY_PORT" ping
export VALKEY_URL="redis://127.0.0.1:${VALKEY_PORT}/"

rustc --version
bash .ci/update-our-crates.sh
cargo metadata --locked --all-features --format-version 1 > dependency-metadata.json
python3 .github/check-first-party.py
cargo fmt --all
git diff --exit-code -- '*.rs' > format.patch
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked -- --nocapture

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
"$NIGHTLY_CARGO" llvm-cov --locked --workspace --all-features --branch --no-cfg-coverage --no-cfg-coverage-nightly --ignore-filename-regex '/tests/' --json --output-path coverage.json
"$NIGHTLY_CARGO" llvm-cov report --locked --branch --ignore-filename-regex '/tests/' --lcov --output-path coverage.lcov
"$NIGHTLY_CARGO" llvm-cov report --locked --branch --ignore-filename-regex '/tests/' --text > coverage.txt
python3 .github/check-coverage.py coverage.lcov coverage.json coverage.txt
