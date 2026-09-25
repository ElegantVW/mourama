#!/usr/bin/env bash
# build.sh — produce release mourama; install into faeOS engine paths
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

cmd="${1:-build}"

build_engine() {
  python3 "$ROOT/tools/make_icons.py"
  cargo build --release --workspace
  BIN="$ROOT/target/release/mourama"
  PLAY="$ROOT/target/release/mourama-play"
  echo "built: $BIN"
  echo "built: $PLAY"
  ls -la "$BIN" "$PLAY"
}

install_engine() {
  build_engine
  BIN="$ROOT/target/release/mourama"
  PLAY="$ROOT/target/release/mourama-play"
  LIB="$HOME/.local/lib/faeos"
  WRAP_SRC="$ROOT/scripts/mourama"
  mkdir -p "$LIB" "$HOME/bin" "$LIB/mourama-web" "$LIB/mourama-icons"
  cp -f "$BIN" "$LIB/mourama"
  cp -f "$PLAY" "$LIB/mourama-play"
  chmod +x "$LIB/mourama" "$LIB/mourama-play"
  cp -a "$ROOT/web/." "$LIB/mourama-web/"
  cp -a "$ROOT/assets/icons/." "$LIB/mourama-icons/"
  mkdir -p "$HOME/.config/systemd/user"
  cp -f "$ROOT/systemd/mourama.service" "$HOME/.config/systemd/user/mourama.service"

  dest="$HOME/bin/mourama"
  cp -f "$WRAP_SRC" "$dest"
  chmod +x "$dest"

  systemctl --user daemon-reload
  echo "installed engine → $LIB/mourama"
  echo "client          → $LIB/mourama-play"
  echo "icons           → $LIB/mourama-icons"
  echo "launcher        → $dest"
  echo "unit            → ~/.config/systemd/user/mourama.service"
  echo "  next:  mourama play"
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
  windows)
    if ! command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
      echo "mourama: mingw is not on PATH yet." >&2
      echo "  next:  sudo pacman -S mingw-w64-gcc" >&2
      exit 1
    fi
    rustup target add x86_64-pc-windows-gnu
    python3 "$ROOT/tools/make_icons.py"
    cargo build --release -p mourama-play --target x86_64-pc-windows-gnu
    mkdir -p "$ROOT/target/mourama-windows" "$HOME/Downloads"
    cp -f "$ROOT/target/x86_64-pc-windows-gnu/release/mourama-play.exe" "$ROOT/target/mourama-windows/mourama-play.exe"
    mkdir -p "$ROOT/target/mourama-windows/icons"
    cp -a "$ROOT/assets/icons/." "$ROOT/target/mourama-windows/icons/"
    (cd "$ROOT/target" && zip -r mourama-windows.zip mourama-windows)
    cp -f "$ROOT/target/mourama-windows.zip" "$HOME/Downloads/mourama-windows.zip"
    echo "zip → $HOME/Downloads/mourama-windows.zip"
    ;;
  *)
    echo "mourama: unknown build target '$cmd'." >&2
    echo "  next:  ./build.sh install   (or keep-awake, apk, windows)" >&2
    exit 2
    ;;
esac
