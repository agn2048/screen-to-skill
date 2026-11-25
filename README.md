# ScreenToSkill (macOS)

Record your screen, talk through what you do, and get a `SKILL.md` that Claude Code, Cursor, Codex,
Gemini CLI or any other agent can follow. The LLM reads a few hundred tokens of text instead of video.

Small native app (Tauri v2, 8 MB download), MIT licensed, usable with no account and no AI at all.
Prefer the command line? See the companion `screen-to-skill-cli` repo.

## Requirements

- macOS 12 or newer
- `brew install ffmpeg python` (FFmpeg 5.1 or newer)

Transcription (whisper.cpp) and on-screen text reading (Apple Vision) are bundled.
The first transcription downloads a Whisper model once (about 150 MB, checksum-verified).

## Use

1. Press **Record**. macOS asks for Screen Recording and Microphone access the first time;
   after allowing Screen Recording, quit and reopen the app.
2. Do the task and explain it aloud. Stop from the **REC** item in the menu bar.
3. Press **Create**, review the text, then copy it or save it to `~/.claude/skills/`.

You can also choose or drop an existing video.

| Provider | Where it runs | Notes |
| --- | --- | --- |
| No AI (default) | this Mac | One Markdown prompt to paste into the LLM you already use |
| Ollama | this Mac | `ollama pull qwen3-vl:4b-instruct` |
| MLX | this Mac, Apple Silicon | `pip install mlx-vlm`, then `mlx_vlm.server --port 8080` |
| Groq, Gemini, OpenRouter | cloud, free tiers | Good for Intel Macs; needs a free key |
| Custom | any OpenAI-compatible server | |

Cloud models are retired often. If one stops working, use **List models** in Settings.

## Privacy

- The badge shows **Local Only** or what leaves the Mac. When it says Local Only, the engine runs with
  `--local-only` and refuses to send frames or audio anywhere. Ollama `-cloud` models count as cloud.
- API keys are stored in the macOS Keychain and are never shown to the UI again.
- Free cloud tiers may use your data to improve their products. Check before recording work screens.
- Common key and password patterns are redacted, best effort. **Review before sharing.**
  Skills made from someone else's recording can contain instructions you did not write.
- Recordings stay in `~/Movies/ScreenToSkill`; results in
  `~/Library/Application Support/io.github.screentoskill/skills`.

## Build

Needs Node 20+, [Rust](https://rustup.rs), Xcode Command Line Tools and `brew install cmake`.

```bash
npm install
npm run sidecars        # builds whisper-cli and ocr-helper into src-tauri/binaries
npm run app:dev         # development
npm run app:build       # .app and .dmg in src-tauri/target/release/bundle
```

Universal (Apple Silicon + Intel) build:

```bash
rustup target add x86_64-apple-darwin aarch64-apple-darwin
npm run sidecars -- universal-apple-darwin
npx tauri build --target universal-apple-darwin
```

Builds are ad-hoc signed. On other Macs, the first launch needs right-click > Open (or System Settings >
Privacy & Security > Open Anyway), and macOS may ask for permissions again after each rebuild. To avoid both,
sign and notarize with an Apple Developer account.

## Layout

```
src/                  React UI
src-tauri/src/        recorder, engine runner, Keychain, permissions, tray
engine/               Python engine (standard library only) and OCR helper
templates/            the prompt that shapes SKILL.md
scripts/              sidecar build, engine sync
```

`engine/` and `templates/` are copies from `screen-to-skill-cli`, where the tests live.
Change them there, then run `scripts/sync-engine.sh`.

## License

MIT. Bundles whisper.cpp (MIT, see THIRD_PARTY_LICENSES.txt). FFmpeg is not bundled.
