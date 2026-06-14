#!/usr/bin/env bash
#
# build.sh — cross-compile the recommender into an Android artifact.
#
# Produces, for the android repo to consume:
#   build/recommender/recommender.aar     ← native libs (jni/<abi>/librecommender_ffi.so)
#   build/recommender/kotlin/             ← generated UniFFI Kotlin bindings (source)
#
# The android app applies the AAR (for the .so per ABI) and adds the generated
# Kotlin under build/recommender/kotlin/ as source (UniFFI's standard Android
# integration: native libs shipped, bindings compiled by the consuming app, which
# already provides the JNA dependency). The AAR path above is THE artifact path
# the orchestrator/android rely on.
#
# Scope: ALWAYS `-p recommender-ffi` (ADR 0005) — never the whole workspace; the
# backend crates (axum/sqlx/tokio) do not cross-compile to Android.
#
# Idempotent: re-running rebuilds cleanly. Fails loudly if the Android toolchain
# (cargo-ndk / NDK) is missing — the orchestrator owns that toolchain.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_ROOT"

FFI_CRATE="recommender-ffi"
LIB_NAME="librecommender_ffi.so"
OUT_DIR="$REPO_ROOT/build/recommender"
JNILIBS_DIR="$OUT_DIR/jniLibs"
KOTLIN_DIR="$OUT_DIR/kotlin"
AAR_PATH="$OUT_DIR/recommender.aar"

ANDROID_ABIS=(
  "aarch64-linux-android"
  "armv7-linux-androideabi"
  "i686-linux-android"
  "x86_64-linux-android"
)

die() {
  echo "ERROR: $*" >&2
  exit 1
}

# --- preconditions (fail loudly; the orchestrator provides the toolchain) -------
command -v cargo >/dev/null 2>&1 || die "cargo not found."

command -v cargo-ndk >/dev/null 2>&1 || die \
  "cargo-ndk not found. Install it with:  cargo install cargo-ndk
   and add the Android Rust targets:  rustup target add ${ANDROID_ABIS[*]}"

if [[ -z "${ANDROID_NDK_HOME:-}" && -z "${ANDROID_NDK_ROOT:-}" && -z "${NDK_HOME:-}" ]]; then
  die "Android NDK not found. Set ANDROID_NDK_HOME (or ANDROID_NDK_ROOT) to your NDK path."
fi

command -v zip >/dev/null 2>&1 || die "zip not found (needed to assemble the .aar)."

# --- clean staging (idempotent) -------------------------------------------------
rm -rf "$OUT_DIR"
mkdir -p "$JNILIBS_DIR" "$KOTLIN_DIR"

# --- 1. Android native libs: all ABIs in one shot, SCOPED to the ffi crate ------
echo ">> Building $FFI_CRATE for: ${ANDROID_ABIS[*]}"
ndk_targets=()
for abi in "${ANDROID_ABIS[@]}"; do
  ndk_targets+=("-t" "$abi")
done
cargo ndk "${ndk_targets[@]}" -o "$JNILIBS_DIR" build --release -p "$FFI_CRATE"

# --- 2. Generate the Kotlin bindings from one built library ---------------------
AARCH64_SO="$REPO_ROOT/target/aarch64-linux-android/release/$LIB_NAME"
[[ -f "$AARCH64_SO" ]] || die "expected built library missing: $AARCH64_SO"

echo ">> Generating Kotlin bindings"
cargo run -q --bin uniffi-bindgen -p "$FFI_CRATE" -- generate \
  --library "$AARCH64_SO" \
  --language kotlin \
  --out-dir "$KOTLIN_DIR"

# --- 3. Assemble the AAR (native libs + manifest) -------------------------------
# cargo-ndk writes jniLibs/<android-abi>/*.so; the AAR wants jni/<android-abi>/*.so.
echo ">> Assembling $AAR_PATH"
AAR_STAGE="$OUT_DIR/aar"
rm -rf "$AAR_STAGE"
mkdir -p "$AAR_STAGE/jni"
cp -R "$JNILIBS_DIR/." "$AAR_STAGE/jni/"

cat > "$AAR_STAGE/AndroidManifest.xml" <<'XML'
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="com.library.recommender" />
XML

# An AAR is a zip; a native-only AAR carries an (empty) classes.jar. Use the JDK's
# jar if present, otherwise a minimal empty zip.
if command -v jar >/dev/null 2>&1; then
  ( cd "$AAR_STAGE" && mkdir -p .empty && jar cf classes.jar -C .empty . && rm -rf .empty )
else
  ( cd "$AAR_STAGE" && : > .keep && zip -q classes.jar .keep && rm -f .keep )
fi

( cd "$AAR_STAGE" && zip -qr "$AAR_PATH" AndroidManifest.xml classes.jar jni )
rm -rf "$AAR_STAGE"

echo ""
echo "Done."
echo "  AAR (native libs):   $AAR_PATH"
echo "  Kotlin bindings:     $KOTLIN_DIR  (add as source in the android module)"
