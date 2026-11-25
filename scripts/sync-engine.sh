#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
SRC="${1:-../screen-to-skill-cli}/screen_to_skill"
cp "$SRC/engine.py" engine/video_to_index.py
cp "$SRC/ocr.swift" engine/ocr.swift
cp "$SRC/skill_prompt.md" templates/skill_prompt.md
echo "Engine synced from $SRC"
