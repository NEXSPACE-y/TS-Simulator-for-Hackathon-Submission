#!/usr/bin/env bash
# Vercel のビルド環境に Rust（rust-toolchain.toml の版）と wasm-pack を入れてから npm ci する。
# Vercel のプロジェクト設定で Root Directory を「TSシミュレーター」にしておくこと。
set -euo pipefail

if ! command -v rustup >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none
fi
# shellcheck disable=SC1091
. "$HOME/.cargo/env"
rustup toolchain install

if ! command -v wasm-pack >/dev/null 2>&1; then
  cargo install wasm-pack --locked
fi

npm ci
