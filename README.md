# LlamaStudio

A desktop app for managing and running local LLMs (GGUF) with a built-in `llama-server` (llama.cpp) backend.

<!-- screenshot -->

## Features

- **Model library** — scans a folder of `.gguf` models (publisher/family layout) and parses GGUF metadata directly (architecture, parameter count, max context length)
- **Organization** — search, groups, pinning, and drag & drop reordering
- **One-click server launch** — spawns `llama-server` with rich options: GPU offload, context length, eval/physical batches, flash attention, speculative decoding (MTP / draft model), KV cache quantization, reasoning budget & effort, model alias, sleep-when-idle
- **Live log console** — streams server output with log levels, in real time
- **Per-model configurations** — settings are saved per model, with presets, system prompt and sampling options
- **OpenAI-compatible server** — endpoint info and API usage hints in the UI
- **System tray** — minimize-to-tray option and window position/size persistence

## Tech Stack

- [Tauri 2](https://tauri.app/) (Rust backend)
- Vue 3 + TypeScript
- Vite
- [tauri-plugin-store](https://v2.tauri.app/plugin/store/), tauri-plugin-dialog, tauri-plugin-opener

## Getting Started

### Prerequisites

- Rust (stable)
- Node.js + [pnpm](https://pnpm.io/)
- A `llama-server` build from [llama.cpp](https://github.com/ggml-org/llama.cpp) (path configurable in Settings)

### Setup

```bash
pnpm install
pnpm tauri dev
```

On first run, configure the following in the **Settings** view:

- **Models path** — folder containing your models, organized as `<publisher>/<family>/<model>.gguf`
- **llama-server path** — path to the `llama-server` executable
- **Port** — port for the OpenAI-compatible server (default: 8080)

### Building

```bash
pnpm tauri build
```

## Configuration

Application settings are persisted via `tauri-plugin-store` (`config.json`):

| Key | Description | Default |
| --- | --- | --- |
| `modelsPath` | Folder to scan for `.gguf` models | *(none)* |
| `llamaPath` | Path to `llama-server` | *(none)* |
| `port` | Server port | `8080` |
| `minimizeToTray` | Minimize to tray instead of closing | `false` |

Each model additionally stores its own inference configuration (context, offload, batches, speculative decoding, reasoning, cache quantization, etc.) keyed by model path.

## Project Structure

```
src/
  views/        # ModelsView, DeveloperView, SettingsView
  components/   # Sidebar, RightPanel, LoadModelModal
  stores/       # config, groups, selectedModel
src-tauri/
  src/lib.rs    # Model scanning, GGUF parser, llama-server process management
```

## Status

Early-stage project, actively in development.
