#!/usr/bin/env bash
# Kado 品質ゲート (問314)。
#
# `docs/SPEC.md` §10 と `CONTRIBUTING.md` が掲げるゲートを**1コマンドで**全部走らせる。
# これまでゲートは5つのコマンドを手で叩く運用で、実行を保証するものが規律しか
# 無かった (問309 で CI が権限上有効化できないことを確認済み)。GitHub Actions は
# リポジトリ所有者にしか置けないが、**ローカルでの自動実行は権限内**である。
#
# 使い方:
#   ./scripts/check.sh          全ゲートを実行
#   ./scripts/check.sh --fast   release ビルドを省略 (日常の反復用)
#
# push 前に自動実行するには (推奨・一度だけ):
#   git config core.hooksPath .githooks
#
# 内容は docs/ci.yml のジョブと同一。CI が有効化されれば同じ検査が
# ubuntu/macos/windows でも走る。
set -euo pipefail

cd "$(dirname "$0")/.."

fast=0
[ "${1:-}" = "--fast" ] && fast=1

fail=0
step() {
  local name="$1"; shift
  printf '\033[1m▸ %s\033[0m\n' "$name"
  if "$@"; then
    printf '  \033[32mPASS\033[0m %s\n' "$name"
  else
    printf '  \033[31mFAIL\033[0m %s\n' "$name"
    fail=1
  fi
}

step "format"  cargo fmt --all -- --check
step "clippy"  cargo clippy --all-targets -- -D warnings
step "rustdoc" env RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --quiet
step "tests"   cargo test --all-targets --quiet
if [ "$fast" -eq 0 ]; then
  step "release build" cargo build --release --quiet
fi

# ADR-003 / 問4: コアは std のみ。docs/ci.yml の no-external-deps ジョブと同じ検査。
printf '\033[1m▸ zero external dependencies\033[0m\n'
deps=$(cargo tree --edges normal --prefix none | grep -v '^kado ' | grep -v '^$' | sort -u || true)
if [ -n "$deps" ]; then
  printf '  \033[31mFAIL\033[0m external dependencies detected (ADR-003 forbids them):\n%s\n' "$deps"
  fail=1
else
  printf '  \033[32mPASS\033[0m zero external dependencies\n'
fi

if [ "$fail" -ne 0 ]; then
  printf '\n\033[31m品質ゲート失敗。push しないこと。\033[0m\n'
  exit 1
fi
printf '\n\033[32m全ゲート通過。\033[0m\n'
