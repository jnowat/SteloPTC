#!/usr/bin/env bash
# setup-android.sh — SteloPTC Android build environment setup
# Usage: bash scripts/setup-android.sh [--build] [--release]
#
# Options:
#   --build    Run `cargo tauri android build` (debug APK) after setup
#   --release  Run `cargo tauri android build --release` (signed release APK) after setup
#
# Requirements satisfied by this script:
#   - Rust + Android targets (aarch64, armv7, i686, x86_64)
#   - Java JDK 17
#   - Android SDK (cmdline-tools, platform-tools, build-tools 34.0.0, platform android-34)
#   - Android NDK r27 (27.x)
#   - Tauri CLI (cargo-tauri)
#   - cargo tauri android init

set -euo pipefail

# ── Colours ─────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; CYAN='\033[0;36m'; NC='\033[0m'
info()    { echo -e "${CYAN}[INFO]${NC}  $*"; }
ok()      { echo -e "${GREEN}[OK]${NC}    $*"; }
warn()    { echo -e "${YELLOW}[WARN]${NC}  $*"; }
die()     { echo -e "${RED}[ERROR]${NC} $*" >&2; exit 1; }

BUILD_DEBUG=false
BUILD_RELEASE=false
for arg in "$@"; do
  case "$arg" in
    --build)   BUILD_DEBUG=true ;;
    --release) BUILD_RELEASE=true ;;
    --help|-h)
      echo "Usage: bash scripts/setup-android.sh [--build] [--release]"
      exit 0
      ;;
  esac
done

echo ""
echo "╔══════════════════════════════════════════════╗"
echo "║   SteloPTC — Android Build Environment Setup ║"
echo "╚══════════════════════════════════════════════╝"
echo ""

# ── Detect OS ───────────────────────────────────────────────────────────────
OS="$(uname -s)"
ARCH="$(uname -m)"
info "Detected OS: $OS / $ARCH"

# ── 1. Rust ──────────────────────────────────────────────────────────────────
info "Checking Rust installation..."
if ! command -v rustup &>/dev/null; then
  info "Installing rustup..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
  # shellcheck source=/dev/null
  source "$HOME/.cargo/env"
fi
if ! command -v cargo &>/dev/null; then
  # shellcheck source=/dev/null
  source "$HOME/.cargo/env" 2>/dev/null || true
fi
RUST_VER=$(rustc --version 2>/dev/null || echo "not found")
ok "Rust: $RUST_VER"

# ── 2. Android Rust targets ─────────────────────────────────────────────────
info "Adding Android Rust targets..."
ANDROID_TARGETS=(
  "aarch64-linux-android"
  "armv7-linux-androideabi"
  "i686-linux-android"
  "x86_64-linux-android"
)
for target in "${ANDROID_TARGETS[@]}"; do
  if rustup target list --installed | grep -q "^$target$"; then
    ok "  Target already installed: $target"
  else
    info "  Installing target: $target"
    rustup target add "$target"
    ok "  Installed: $target"
  fi
done

# ── 3. Java JDK 17 ──────────────────────────────────────────────────────────
info "Checking Java JDK 17..."
JAVA_OK=false
if command -v java &>/dev/null; then
  JAVA_VER=$(java -version 2>&1 | head -1)
  if echo "$JAVA_VER" | grep -qE '"17|"21'; then
    ok "Java: $JAVA_VER"
    JAVA_OK=true
  else
    warn "Java found but not JDK 17+: $JAVA_VER"
  fi
fi

if [ "$JAVA_OK" = false ]; then
  info "Installing JDK 17..."
  case "$OS" in
    Linux)
      if command -v apt-get &>/dev/null; then
        sudo apt-get update -qq
        sudo apt-get install -y openjdk-17-jdk
      elif command -v dnf &>/dev/null; then
        sudo dnf install -y java-17-openjdk-devel
      elif command -v pacman &>/dev/null; then
        sudo pacman -Sy --noconfirm jdk17-openjdk
      else
        die "Unsupported Linux package manager. Install JDK 17 manually and re-run."
      fi
      ;;
    Darwin)
      if command -v brew &>/dev/null; then
        brew install --cask temurin@17
      else
        die "Homebrew not found. Install JDK 17 from https://adoptium.net and re-run."
      fi
      ;;
    *)
      die "Unsupported OS: $OS. Install JDK 17 manually and re-run."
      ;;
  esac
  ok "JDK 17 installed."
fi

# Ensure JAVA_HOME is set
if [ -z "${JAVA_HOME:-}" ]; then
  if command -v java &>/dev/null; then
    JAVA_BIN=$(command -v java)
    # Resolve symlinks
    JAVA_BIN=$(readlink -f "$JAVA_BIN" 2>/dev/null || realpath "$JAVA_BIN" 2>/dev/null || echo "$JAVA_BIN")
    export JAVA_HOME="${JAVA_BIN%/bin/java}"
    info "Set JAVA_HOME=$JAVA_HOME"
  fi
fi

# ── 4. Android SDK ──────────────────────────────────────────────────────────
ANDROID_HOME="${ANDROID_HOME:-$HOME/android-sdk}"
info "Android SDK location: $ANDROID_HOME"

SDKMANAGER=""
if [ -n "${ANDROID_HOME}" ] && [ -f "$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager" ]; then
  SDKMANAGER="$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager"
  ok "sdkmanager found: $SDKMANAGER"
elif command -v sdkmanager &>/dev/null; then
  SDKMANAGER=$(command -v sdkmanager)
  ok "sdkmanager found in PATH: $SDKMANAGER"
fi

if [ -z "$SDKMANAGER" ]; then
  info "Downloading Android command-line tools..."
  mkdir -p "$ANDROID_HOME/cmdline-tools"
  case "$OS" in
    Linux)  CMD_TOOLS_URL="https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip" ;;
    Darwin) CMD_TOOLS_URL="https://dl.google.com/android/repository/commandlinetools-mac-11076708_latest.zip" ;;
    *)      die "Unsupported OS for automatic Android SDK download." ;;
  esac
  TMP_ZIP="$(mktemp /tmp/cmdtools.XXXXXX.zip)"
  curl -fsSL "$CMD_TOOLS_URL" -o "$TMP_ZIP"
  unzip -q "$TMP_ZIP" -d "$ANDROID_HOME/cmdline-tools"
  rm "$TMP_ZIP"
  # Google packages it as cmdline-tools/cmdline-tools — rename to 'latest'
  if [ -d "$ANDROID_HOME/cmdline-tools/cmdline-tools" ]; then
    mv "$ANDROID_HOME/cmdline-tools/cmdline-tools" "$ANDROID_HOME/cmdline-tools/latest"
  fi
  SDKMANAGER="$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager"
  ok "cmdline-tools installed."
fi

export ANDROID_HOME
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export PATH="$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools:$ANDROID_HOME/tools/bin:$PATH"

info "Accepting Android SDK licenses..."
yes | "$SDKMANAGER" --licenses >/dev/null 2>&1 || true

info "Installing Android SDK packages (platform-tools, build-tools 34.0.0, android-34)..."
"$SDKMANAGER" \
  "platform-tools" \
  "build-tools;34.0.0" \
  "platforms;android-34"
ok "Android SDK packages installed."

# ── 5. Android NDK r27 ──────────────────────────────────────────────────────
info "Checking Android NDK r27..."
NDK_VERSION="27.2.12479018"
NDK_DIR="$ANDROID_HOME/ndk/$NDK_VERSION"
if [ -d "$NDK_DIR" ]; then
  ok "NDK r27 already installed at $NDK_DIR"
else
  info "Installing NDK r27 ($NDK_VERSION)..."
  "$SDKMANAGER" "ndk;$NDK_VERSION"
  ok "NDK r27 installed."
fi
export ANDROID_NDK_HOME="$NDK_DIR"
export NDK_HOME="$NDK_DIR"

# ── 6. Tauri CLI ─────────────────────────────────────────────────────────────
info "Checking Tauri CLI..."
if cargo tauri --version &>/dev/null; then
  TAURI_CLI_VER=$(cargo tauri --version 2>/dev/null | head -1)
  ok "Tauri CLI: $TAURI_CLI_VER"
else
  info "Installing Tauri CLI..."
  cargo install tauri-cli --version "^2" --locked
  ok "Tauri CLI installed."
fi

# ── 7. cargo tauri android init ─────────────────────────────────────────────
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

info "Running: cargo tauri android init"
if [ -d "src-tauri/gen/android" ]; then
  warn "src-tauri/gen/android already exists — skipping init (delete it to re-init)."
else
  cargo tauri android init
  ok "Android project initialised at src-tauri/gen/android"
fi

# ── 8. Environment summary ───────────────────────────────────────────────────
echo ""
echo "══════════════════════════════════════════════════"
echo "  Environment variables for your shell profile:"
echo "══════════════════════════════════════════════════"
echo "  export ANDROID_HOME=\"$ANDROID_HOME\""
echo "  export ANDROID_SDK_ROOT=\"$ANDROID_HOME\""
echo "  export ANDROID_NDK_HOME=\"$NDK_DIR\""
echo "  export NDK_HOME=\"$NDK_DIR\""
echo "  export PATH=\"\$ANDROID_HOME/cmdline-tools/latest/bin:\$ANDROID_HOME/platform-tools:\$PATH\""
echo "══════════════════════════════════════════════════"
echo ""
ok "All prerequisites are satisfied."

# ── 9. Optional APK build ────────────────────────────────────────────────────
if [ "$BUILD_RELEASE" = true ]; then
  info "Building release APK..."
  cargo tauri android build --release
  APK_DIR="$REPO_ROOT/src-tauri/gen/android/app/build/outputs/apk/universal/release"
  ok "Release APK built. Check: $APK_DIR"
elif [ "$BUILD_DEBUG" = true ]; then
  info "Building debug APK..."
  cargo tauri android build
  APK_DIR="$REPO_ROOT/src-tauri/gen/android/app/build/outputs/apk/universal/debug"
  ok "Debug APK built. Check: $APK_DIR"
else
  echo ""
  info "Setup complete. To build the APK, run:"
  echo "  bash scripts/setup-android.sh --build          # debug APK"
  echo "  bash scripts/setup-android.sh --release        # release APK"
  echo ""
  info "Or run manually after sourcing the env vars above:"
  echo "  cargo tauri android build"
fi
