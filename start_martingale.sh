#!/bin/sh
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
echo "Martingale Roulette Simulator"
echo "Open http://127.0.0.1:3000 after the server reports it is ready."
echo "Press Ctrl+C to stop."
if [ -f ./martingale_sim ]; then
    exec ./martingale_sim
fi
if ! command -v cargo >/dev/null 2>&1; then
    echo "Install Rust from https://rustup.rs or download a prebuilt GitHub Release." >&2
    exit 1
fi
exec cargo run --release --locked
