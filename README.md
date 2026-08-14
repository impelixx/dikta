# Dikta

[![CI](https://github.com/impelixx/dikta/actions/workflows/ci.yml/badge.svg)](https://github.com/impelixx/dikta/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg?logo=rust)](src-tauri)
[![TypeScript](https://img.shields.io/badge/typescript-5.6-3178c6.svg?logo=typescript&logoColor=white)](src)
[![Tauri](https://img.shields.io/badge/tauri-2-24c8db.svg?logo=tauri&logoColor=white)](https://tauri.app)

*[Русская версия](README.ru.md)*

Push-to-talk dictation for Russian — speech recognition running entirely on
your machine, no cloud, no subscription. Press the hotkey, speak, and the text
is already typed into whatever field had focus, in any application.

Under the hood: local ASR models ([GigaAM](https://github.com/salute-developers/GigaAM)
by Sber, plus Whisper by OpenAI and NeMo/Zipformer community exports) served
through a hybrid backend — [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx)
(ONNX Runtime) for CTC/Transducer/Whisper-ONNX models, and
[whisper.cpp](https://github.com/ggerganov/whisper.cpp) (via `whisper-rs`,
GGML models, GPU-accelerated through Metal on macOS) for Whisper — both
directly inside the Rust process, no Python runtime involved. Desktop shell
built with [Tauri 2](https://tauri.app/) and a thin TypeScript frontend, no
framework.

## Contents

- [Features](#features)
- [Install](#install)
- [Usage](#usage)
- [Settings](#settings)
- [macOS permissions](#macos-permissions)
- [Architecture](#architecture)
- [Development](#development)
- [FAQ](#faq)
- [License](#license)

## Features

| | |
|---|---|
| **Push-to-talk and toggle hotkeys** | Separate bindings for each mode, captured globally in any application regardless of focus |
| **Silence-based auto-stop** | Adjustable sensitivity (from "whisper-quiet" to "construction site") and pause duration |
| **Live captions** | See what you've said while you're still talking — the growing buffer is re-decoded roughly every 900ms |
| **Floating overlay** | Recording/recognition status on top of any app — no need to switch to the Dikta window |
| **Auto-paste** | Types into the focused field via the Accessibility API, with a guaranteed clipboard fallback |
| **Noise suppression** | Optional RNNoise-based denoising (`nnnoiseless`, pure Rust) runs on the buffer before recognition — on by default, toggle it in Settings |
| **Hybrid ASR backend** | [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) (CTC/Transducer/Whisper-ONNX) and [whisper.cpp](https://github.com/ggerganov/whisper.cpp) (GPU-accelerated via Metal on macOS) side by side, pick per model |
| **11 models in the catalog** | GigaAM v3 (CTC/Transducer, ±punctuation), Whisper tiny/base/small/medium via whisper.cpp, plus NeMo Conformer (English), Fast Conformer (10 European languages) and Zipformer (Russian) — filterable by language and engine, switch on the fly from the tray, with real measured speed |
| **Bring your own model** | Any sherpa-onnx-compatible model by Hugging Face repo id |
| **Microphone selection** | Switch the input device without restarting the app |
| **Three themes** | Cream/terracotta, mint, lavender |
| **History and stats** | Search/copy/delete past transcriptions, daily activity charts |
| **Tray-only** | Doesn't sit in the Dock/taskbar; closing the window hides it instead of quitting |

## Install

**From a release.** Grab the installer for your OS from
[Releases](https://github.com/impelixx/dikta/releases). Builds aren't signed
with an Apple Developer certificate — on first launch macOS will block it;
open it via System Settings → Privacy & Security → **Open Anyway**.

**From source:**

```bash
git clone https://github.com/impelixx/dikta.git
cd dikta
npm install
npm run tauri dev      # development, with hot-reload
# or
npm run tauri build    # production installer build
```

You'll need to download a model once, right inside the app — on first launch
there isn't one yet, and the overlay will point you to Settings → "Recognition
model" → "Download" (from ~80MB for the smallest Whisper tiny model up to
~1.5GB for Whisper medium; the Russian GigaAM/Zipformer models sit around
85–280MB).

## Usage

1. **Press the hotkey** in any application — `⌘⇧D` by default for push-to-talk
   (hold while speaking) or `⌘⇧Space` for toggle (press to start, press again
   to stop). Both are rebindable in Settings by clicking the button and typing
   the combination you want.
2. **Speak.** The floating overlay near the bottom of the screen shows the
   text live as you go — this isn't "real" realtime streaming (there's no
   dedicated online model), it's an honest compromise: the growing buffer is
   fully re-decoded roughly every 900ms, and since the model decodes faster
   than realtime (RTF ≈ 0.08, i.e. ~12x realtime for the default model), the
   lag isn't noticeable at typical dictation lengths.
3. **Release/stop.** With auto-stop on, you don't even need to press again —
   recording stops itself after the configured pause. The recognized text is
   typed automatically wherever the cursor was. If auto-paste isn't available
   (no permission, or the context doesn't support it), the text is guaranteed
   to land in the clipboard — paste manually with `⌘V`/`Ctrl+V`.

Everything you've dictated goes into history (searchable, copyable, deletable)
and stats (time, words, sessions, a daily activity chart).

## Settings

- **Hotkeys** — push-to-talk and toggle, set by pressing the combination you want
- **Auto-stop sensitivity** — how quiet it needs to get before stopping
- **Pause before auto-stop** — how long to wait in silence (0.5–4s), shorter for commands, longer for connected speech
- **Auto-stop in push-to-talk / in toggle** — toggled independently
- **Auto-paste** — on/off; when off, text goes straight to the clipboard
- **Noise suppression** — on/off; RNNoise-based denoising applied before recognition (on by default)
- **Input device** — pick your microphone
- **Theme** — cream/terracotta, mint, lavender
- **Recognition model** — the built-in catalog (filterable by language and engine via dropdowns), plus your own Hugging Face models

All settings live in SQLite in the app's standard data directory
(`~/Library/Application Support/dikta` on macOS), alongside the history.

## macOS permissions

- **Microphone** — required, recording won't start without it
- **Accessibility** — needed for auto-paste via synthetic keyboard events.
  Without it the text still reliably lands in the clipboard — just paste it
  manually. One notable wrinkle: on macOS, attempting auto-paste without the
  permission may not surface as an explicit error (CGEventPost simply doesn't
  reach the target app), so Dikta checks `AXIsProcessTrusted()` up front
  instead of trusting the return value blindly.

Both permissions live under System Settings → Privacy & Security.

## Architecture

```
src/                   main window frontend (TS, no framework)
src/overlay.ts          floating overlay frontend
src-tauri/src/
  lib.rs                 entry point, tray, windows, command routing
  hotkeys.rs             push-to-talk/toggle, live captions, VAD watcher
  audio.rs               cpal wrapper, resampling, buffer snapshots, RNNoise denoising
  asr.rs                 hybrid recognizer: sherpa-rs-sys (CTC, Transducer, Whisper-ONNX)
                          and whisper-rs/whisper.cpp (GPU via Metal on macOS)
  models.rs              model catalog (sherpa-onnx + whisper.cpp entries), downloads, custom HF models
  db.rs                  SQLite: history, stats, settings
  paste.rs               auto-paste + unconditional clipboard fallback
  vad.rs                 silence detector based on RMS energy
  settings.rs             settings struct and (de)serialization
  commands.rs             Tauri commands invoked from the frontend
site/                  promo page (static, published to GitHub Pages)
```

Data flow while dictating: hotkey → `AudioEngine::start` (the cpal stream is
already open for the app's entire lifetime; start/stop just toggles whether
samples get pushed into the buffer) → in parallel, a VAD watcher (auto-stop),
a level watcher (waveform in the UI), and a partial-transcript watcher (live
captions) all run → on completion, the buffer is optionally denoised
(RNNoise via `nnnoiseless`) → `Recognizer::decode`, routed to sherpa-onnx
(offline CTC/Transducer/Whisper-ONNX) or whisper.cpp (GPU-accelerated via
Metal on macOS) depending on the selected model's engine →
`paste::insert_text` (Accessibility + unconditional clipboard write) → the
entry is saved to SQLite → events fan out to every window (`main` and
`overlay`) through the Tauri event system.

More detail in [CONTRIBUTING.md](CONTRIBUTING.md).

## Development

```bash
npm install
npm run tauri dev
```

Before opening a PR:

```bash
cd src-tauri && cargo build && cargo test --lib
cd .. && npm run build
```

CI (`.github/workflows/ci.yml`) runs the same checks on push and PR. Releases
(`.github/workflows/release.yml`) build on tag `vX.Y.Z` for macOS (universal,
packaged as a `.dmg` with an `/Applications` shortcut), Windows, and Linux
(requires `libxdo-dev` for clipboard/paste emulation via `enigo`) via
`tauri-action`, publishing a draft GitHub Release. More detail in
[CONTRIBUTING.md](CONTRIBUTING.md).

## FAQ

**Why isn't there live streaming recognition like Siri?**
That would need a dedicated class of streaming/online models — the models
available here are exported as offline models. Instead, Dikta uses an honest
compromise: since decoding runs several times faster than realtime, a
periodic re-decode of the growing buffer (every ~900ms) reads as live
captions without needing a different architecture.

**Can I use a language other than Russian?**
The default catalog includes Whisper (multilingual, ~99 languages, via
whisper.cpp) alongside GigaAM (Russian-only, but noticeably more accurate on
Russian), an English-only NeMo Conformer, a Fast Conformer covering 10
European languages (Belarusian, German, English, Spanish, French, Croatian,
Italian, Polish, Russian, Ukrainian), and a Russian-only Zipformer. Filter
the catalog by language or engine right from the dropdowns in Settings, or
add any other sherpa-onnx-compatible offline CTC/Transducer model via a
Hugging Face repo id.

**The app doesn't show up in the Dock — how do I open it?**
It's intentionally tray-only (menu bar icon on macOS / tray on Windows).
Click the icon, or use "Open Dikta" from its menu.

**Push-to-talk doesn't respond to the hotkey.**
Check that the chosen combination physically exists on your keyboard (`F13`,
for instance, is missing on most laptop keyboards) and isn't already claimed
by another app or the OS.

## License

Code is [MIT](LICENSE). Bundled/downloadable ASR models carry their own
licenses independent of the app's — GigaAM in particular is licensed
non-commercially (GigaAM License NC, included as a PDF inside its download).
Models are fetched at runtime, not distributed with the code.
