\
#!/usr/bin/env bash
set -euo pipefail
# professional test + doc helper for ratcrate project v1.0
ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
echo "Project root: $ROOT_DIR"

# 1) Toolchain checks
echo "Checking Rust toolchain..."
if ! command -v rustc >/dev/null 2>&1; then
  echo "ERROR: rustc not found. Install Rust from https://rustup.rs" >&2
  exit 2
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo "ERROR: cargo not found. Install Rust from https://rustup.rs" >&2
  exit 2
fi
echo "rustc: $(rustc --version)"
echo "cargo: $(cargo --version)"

# 2) Format & lint
echo "-> Running cargo fmt (safe)..."
cargo fmt --all -- --check || { echo "cargo fmt suggested changes. Running formatter..."; cargo fmt --all; }

echo "-> Running cargo clippy (best-effort)..."
if command -v cargo-clippy >/dev/null 2>&1 || true; then
  # run clippy but do not fail the whole script on warnings unless explicitly desired
  cargo clippy --all-targets -- -D warnings || echo "clippy reported issues (non-fatal)"
else
  cargo clippy --all-targets -- -D warnings || echo "clippy not available or reported warnings"
fi

# 3) Run tests
echo "-> Running cargo test --verbose"
cargo test --verbose

# 4) Build docs
echo "-> Generating documentation (cargo doc --no-deps)"
cargo doc --no-deps --document-private-items

echo "Docs built to: target/doc"
echo "To open docs locally:"
if command -v xdg-open >/dev/null 2>&1; then
  echo "  xdg-open target/doc/<your_crate>/index.html (Linux)"
elif command -v open >/dev/null 2>&1; then
  echo "  open target/doc/<your_crate>/index.html (macOS)"
fi

echo "Helper script complete."
