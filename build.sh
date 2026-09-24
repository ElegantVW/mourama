#!/usr/bin/env bash
# build.sh — produce release mourama; install into faeOS engine paths
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

cmd="${1:-build}"

build_engine() {
  cargo build --release
  BIN="$ROOT/target/release/mourama"
  echo "built: $BIN"
  ls -la "$BIN"
}

install_engine() {
  build_engine
  BIN="$ROOT/target/release/mourama"
  LIB="$HOME/.local/lib/faeos"
  WRAP_SRC="$ROOT/scripts/mourama"
  mkdir -p "$LIB" "$HOME/bin" "$LIB/mourama-web"
  cp -f "$BIN" "$LIB/mourama"
  chmod +x "$LIB/mourama"
  cp -a "$ROOT/web/." "$LIB/mourama-web/"
  mkdir -p "$HOME/.config/systemd/user"
  cp -f "$ROOT/systemd/mourama.service" "$HOME/.config/systemd/user/mourama.service"

  dest="$HOME/bin/mourama"
  cp -f "$WRAP_SRC" "$dest"
  chmod +x "$dest"

  systemctl --user daemon-reload
  echo "installed engine → $LIB/mourama"
  echo "web             → $LIB/mourama-web"
  echo "launcher        → $dest"
  echo "unit            → ~/.config/systemd/user/mourama.service"
  echo "  next:  mourama invite && systemctl --user enable --now mourama"
}

case "$cmd" in
  build|"")
    build_engine
    ;;
  install)
    install_engine
    ;;
  keep-awake)
    exec sudo "$ROOT/scripts/keep-awake.sh"
    ;;
  apk)
    if [[ ! -x "$ROOT/android/gradlew" ]] && ! command -v gradle >/dev/null 2>&1; then
      echo "mourama: gradle is not on PATH yet." >&2
      echo "  next:  sudo pacman -S jdk17-openjdk gradle android-tools" >&2
      exit 1
    fi
    if [[ -z "${ANDROID_HOME:-}${ANDROID_SDK_ROOT:-}" && ! -d "$HOME/Android/Sdk" ]]; then
      echo "mourama: Android SDK is not on this box yet." >&2
      echo "  next:  install cmdline-tools, then sdkmanager platform-33 build-tools" >&2
      exit 1
    fi
    export ANDROID_HOME="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}}"
    export ANDROID_SDK_ROOT="$ANDROID_HOME"
    (cd "$ROOT/android" && ${GRADLE:-gradle} assembleDebug)
    mkdir -p "$ROOT/target" "$HOME/Downloads"
    apk="$(find "$ROOT/android" -name 'app-debug.apk' | head -1)"
    cp -f "$apk" "$ROOT/target/mourama.apk"
    cp -f "$apk" "$HOME/Downloads/mourama.apk"
    echo "apk → $ROOT/target/mourama.apk"
    echo "apk → $HOME/Downloads/mourama.apk"
    ;;
  *)
    echo "mourama: unknown build target '$cmd'." >&2
    echo "  next:  ./build.sh install   (or keep-awake, apk)" >&2
    exit 2
    ;;
esac
