#!/usr/bin/env bash
# Keep a live Lunchbox running while you edit code.
#
# The app is only rebuilt and swapped when the new binary is ready: the old
# instance keeps serving until then, so it is down for roughly a second per
# change instead of for the whole build. A failed build leaves the running app
# alone.
#
#     ./dev.sh &          # watch app and vendored Rust code, then restart
#
# Runs the debug binary: the first link takes a few minutes, later one-file Rust
# rebuilds take about twenty-five seconds.
set -euo pipefail
cd "$(dirname "$0")"

# Re-exec inside the dev shell when watchexec is not on PATH, so the script
# works whether or not the caller already ran `nix develop`.
if ! command -v watchexec >/dev/null 2>&1; then
  exec nix develop --command bash "$0" "$@"
fi

app_pid=""
ocr_feature_args=()
if [[ "$(uname -s)" == Linux && "$(uname -m)" == x86_64 ]]; then
  ocr_feature_args=(--features rocm-ocr)
fi

stop_app() {
  if [[ -n "$app_pid" ]] && kill -0 "$app_pid" 2>/dev/null; then
    kill "$app_pid" 2>/dev/null || true
    # Wait for a clean exit so the next launch can take the instance lock, but
    # never hang: the window should be gone for about a second at most.
    for _ in $(seq 1 60); do
      kill -0 "$app_pid" 2>/dev/null || break
      sleep 0.05
    done
    kill -9 "$app_pid" 2>/dev/null || true
  fi
  app_pid=""
}

start_app() {
  # Qt/QML diagnostics go to the same log so a broken binding is visible.
  QT_LOGGING_TO_CONSOLE=1 target/debug/lunchbox >>"${LUNCHBOX_DEV_LOG:-/tmp/lunchbox-dev.log}" 2>&1 &
  app_pid=$!
}

# Returns non-zero when the build fails.
build() {
  cargo build -p lunchbox-app --bin lunchbox "${ocr_feature_args[@]}"
}

# Swaps the app for the freshly built binary.
swap() {
  stop_app
  start_app
}

trap stop_app EXIT INT TERM

echo "[dev] first build; the app starts as soon as it is ready"
if build; then
  start_app
  echo "[dev] running target/debug/lunchbox (pid $app_pid)"
else
  echo "[dev] build failed; fix the error and save to retry" >&2
fi

# watchexec restarts this build command on every change; --shell=none means the
# marker line is the only thing we have to trust.
watchexec --restart --shell=none \
  --watch crates \
  --watch vendor \
  --watch Cargo.toml \
  --watch Cargo.lock \
  --exts rs,qml,json,toml,lock,h,cpp,slang,slangp \
  -- bash -c 'ocr_feature_args=(); if [[ "$(uname -s)" == Linux && "$(uname -m)" == x86_64 ]]; then ocr_feature_args=(--features rocm-ocr); fi; if cargo build -p lunchbox-app --bin lunchbox "${ocr_feature_args[@]}"; then echo LUNCHBOX_DEV_BUILT; fi' \
  | while IFS= read -r line; do
      printf '[dev] %s\n' "$line"
      if [[ "$line" == LUNCHBOX_DEV_BUILT ]]; then
        swap
        echo "[dev] restarted (pid $app_pid)"
      fi
    done
