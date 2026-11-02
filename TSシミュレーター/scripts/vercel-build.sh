#!/usr/bin/env bash
# Vercel：WASMのビルドを含めて Web版をビルドする（npm run build）
set -euo pipefail
# shellcheck disable=SC1091
. "$HOME/.cargo/env"
npm run build
