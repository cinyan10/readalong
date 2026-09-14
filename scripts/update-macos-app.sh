#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
app_name="Readalong.app"
built_app="$repo_root/src-tauri/target/release/bundle/macos/$app_name"
installed_app="${READALONG_APP_PATH:-/Applications/$app_name}"
skip_pull=false
relaunch=false

usage() {
  cat <<'EOF'
Usage: bun run app:update -- [--local] [--open]

Builds Readalong and safely replaces the installed macOS app.

  --local    Build the current checkout without pulling from Git first.
  --open     Launch Readalong after installing it.

Set READALONG_APP_PATH to install somewhere other than /Applications/Readalong.app.
EOF
}

for argument in "$@"; do
  case "$argument" in
    --local)
      skip_pull=true
      ;;
    --open)
      relaunch=true
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $argument" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "This command only installs the macOS app bundle." >&2
  exit 1
fi

cd "$repo_root"

if [[ "$skip_pull" == false ]]; then
  if ! git diff --quiet || ! git diff --cached --quiet; then
    echo "Note: local changes are present; Git will keep them while fast-forwarding."
  fi
  git pull --ff-only
fi

bun install --frozen-lockfile
bun run tauri:build

if [[ ! -d "$built_app" ]]; then
  echo "Build completed without producing $built_app" >&2
  exit 1
fi

install_dir="$(dirname "$installed_app")"
staged_app="$install_dir/.${app_name}.installing.$$"
backup_app="$install_dir/.${app_name}.backup.$$"

cleanup() {
  if [[ -e "$staged_app" ]]; then
    rm -rf "$staged_app"
  fi
}
trap cleanup EXIT

mkdir -p "$install_dir"
ditto "$built_app" "$staged_app"

osascript -e 'tell application id "com.cinyan10.readalong" to quit' >/dev/null 2>&1 || true

if [[ -e "$installed_app" ]]; then
  mv "$installed_app" "$backup_app"
fi

if ! mv "$staged_app" "$installed_app"; then
  if [[ -e "$backup_app" ]]; then
    mv "$backup_app" "$installed_app"
  fi
  echo "Installation failed; the previous app was restored." >&2
  exit 1
fi

if [[ -e "$backup_app" ]]; then
  rm -rf "$backup_app"
fi

if [[ "$relaunch" == true ]]; then
  open "$installed_app"
fi

echo "Installed the latest Readalong build at $installed_app"
