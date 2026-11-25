#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
WHISPER_TAG=v1.9.6
MIN=12.0
TRIPLE="${1:-}"
if [ -z "$TRIPLE" ]; then
  [ "$(sysctl -n hw.optional.arm64 2>/dev/null || echo 0)" = 1 ] && TRIPLE=aarch64-apple-darwin || TRIPLE=x86_64-apple-darwin
fi
command -v cmake >/dev/null || { echo "cmake is required: brew install cmake" >&2; exit 1; }
OUT=src-tauri/binaries
mkdir -p "$OUT" .build
SRC=.build/whisper.cpp-$WHISPER_TAG
[ -d "$SRC" ] || git clone --quiet --depth 1 --branch "$WHISPER_TAG" https://github.com/ggml-org/whisper.cpp "$SRC"

build() {
  local arch=arm64 metal=ON simd=""
  if [ "$1" = x86_64-apple-darwin ]; then
    arch=x86_64 metal=OFF
    simd="-DGGML_AVX=OFF -DGGML_AVX2=OFF -DGGML_FMA=OFF -DGGML_F16C=OFF -DGGML_BMI2=OFF"
  fi
  cmake -S "$SRC" -B ".build/whisper-$arch" -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF \
    -DCMAKE_OSX_ARCHITECTURES=$arch -DCMAKE_SYSTEM_PROCESSOR=$arch -DCMAKE_OSX_DEPLOYMENT_TARGET=$MIN \
    -DGGML_METAL=$metal -DGGML_METAL_EMBED_LIBRARY=$metal -DGGML_OPENMP=OFF -DGGML_NATIVE=OFF $simd \
    -DWHISPER_BUILD_TESTS=OFF -DWHISPER_BUILD_SERVER=OFF > ".build/cmake-$arch.log" 2>&1
  cmake --build ".build/whisper-$arch" -j --target whisper-cli >> ".build/cmake-$arch.log" 2>&1
  cp ".build/whisper-$arch/bin/whisper-cli" "$OUT/whisper-cli-$1"
  xcrun swiftc -O -target "$arch-apple-macos$MIN" engine/ocr.swift -o "$OUT/ocr-helper-$1"
  strip -x "$OUT/whisper-cli-$1" "$OUT/ocr-helper-$1" 2>/dev/null
  codesign --force --sign - "$OUT/whisper-cli-$1" "$OUT/ocr-helper-$1"
}

if [ "$TRIPLE" = universal-apple-darwin ]; then
  for t in aarch64-apple-darwin x86_64-apple-darwin; do build $t; done
  for n in whisper-cli ocr-helper; do
    lipo -create "$OUT/$n-aarch64-apple-darwin" "$OUT/$n-x86_64-apple-darwin" -output "$OUT/$n-$TRIPLE"
  done
else
  build "$TRIPLE"
fi
ls -lh "$OUT"
