import { invoke } from "@tauri-apps/api/core";

export interface Settings {
  provider: string;
  models: Record<string, string>;
  base_urls: Record<string, string>;
  transcriber: string;
  whisper_model: string;
  language: string;
  sensitivity: number;
  max_frames: number;
  mic: boolean;
}

export interface Hardware {
  arch: string;
  chip: string;
  ram_gb: number;
  provider: string;
  model: string;
  reason: string;
}

export interface Dependencies {
  python: string | null;
  ffmpeg: string | null;
  whisper: boolean;
  ocr: boolean;
}

export interface Output {
  kind: "skill" | "prompt";
  name: string;
  content: string;
  index: string;
  steps: number;
  folder: string;
}

export interface RecordingEvent {
  state: "recording" | "stopped" | "error";
  path: string | null;
  message: string | null;
  help: "screen" | "microphone" | null;
}

export interface EngineEvent {
  type: "progress" | "warning";
  stage?: string;
  message: string;
  current?: number;
  total?: number;
}

export interface Provider {
  id: string;
  label: string;
  model: string;
  baseUrl: string;
  key: boolean;
  editableUrl: boolean;
  hint: string;
}

export const PROVIDERS: Provider[] = [
  { id: "none", label: "No AI (paste into any LLM)", model: "", baseUrl: "", key: false, editableUrl: false,
    hint: "Combines narration and on-screen text into one Markdown prompt for the LLM you already use. No setup." },
  { id: "ollama", label: "Ollama (local)", model: "qwen3-vl:4b-instruct", baseUrl: "http://localhost:11434/v1", key: false, editableUrl: true,
    hint: "Install from ollama.com, then run: ollama pull <model>" },
  { id: "mlx", label: "MLX (local, Apple Silicon)", model: "mlx-community/Qwen3-VL-4B-Instruct-4bit", baseUrl: "http://localhost:8080/v1", key: false, editableUrl: true,
    hint: "pip install mlx-vlm, then run: mlx_vlm.server --port 8080" },
  { id: "groq", label: "Groq (cloud, free tier)", model: "qwen/qwen3.8-27b", baseUrl: "https://api.groq.com/openai/v1", key: true, editableUrl: false,
    hint: "Get a key at console.groq.com/keys" },
  { id: "gemini", label: "Gemini (cloud, free tier)", model: "gemini-3.8-flash", baseUrl: "https://generativelanguage.googleapis.com/v1beta/openai", key: true, editableUrl: false,
    hint: "Get a key at aistudio.google.com/apikey. Free-tier data may be used by Google to improve its products." },
  { id: "openrouter", label: "OpenRouter (cloud, free models)", model: "google/gemma-4-31b-it:free", baseUrl: "https://openrouter.ai/api/v1", key: true, editableUrl: false,
    hint: "Get a key at openrouter.ai/keys. Free models may log prompts." },
  { id: "custom", label: "Custom (OpenAI-compatible)", model: "", baseUrl: "", key: true, editableUrl: true,
    hint: "Any OpenAI-compatible server that accepts images." },
];

export const provider = (id: string) => PROVIDERS.find((p) => p.id === id) ?? PROVIDERS[0];
export const modelOf = (s: Settings) => s.models[s.provider]?.trim() || provider(s.provider).model;
export const baseUrlOf = (s: Settings) => {
  const p = provider(s.provider);
  return (p.editableUrl && s.base_urls[s.provider]?.trim()) || p.baseUrl;
};

function isLocalUrl(url: string) {
  try {
    const host = new URL(url).hostname;
    return host === "localhost" || host === "[::1]" || /^127(\.\d{1,3}){3}$/.test(host);
  } catch {
    return false;
  }
}

export function leavesDevice(s: Settings): string[] {
  const out: string[] = [];
  if (s.provider !== "none" && !(isLocalUrl(baseUrlOf(s)) && !/[:-]cloud$/i.test(modelOf(s)))) out.push("screen frames");
  if (s.transcriber === "cloud" && s.mic) out.push("audio");
  else if (s.transcriber === "cloud") out.push("any audio in the video");
  return out;
}

export const call = {
  hardware: () => invoke<Hardware>("hardware"),
  dependencies: () => invoke<Dependencies>("check_dependencies"),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  hasKey: (account: string) => invoke<boolean>("has_api_key", { account }),
  setKey: (account: string, key: string) => invoke<void>("set_api_key", { account, key }),
  deleteKey: (account: string) => invoke<void>("delete_api_key", { account }),
  models: (settings: Settings) => invoke<string[]>("list_models", { settings }),
  startRecording: (mic: boolean) => invoke<void>("start_recording", { mic }),
  stopRecording: () => invoke<void>("stop_recording"),
  openPrivacy: (pane: string) => invoke<void>("open_privacy_settings", { pane }),
  convert: (video: string, settings: Settings, localOnly: boolean) =>
    invoke<Output>("convert", { video, settings, localOnly }),
  cancel: () => invoke<void>("cancel_convert"),
  export: (a: { name: string; kind: string; content: string; dir: string | null; overwrite: boolean }) =>
    invoke<string>("export", a),
  copy: (text: string) => invoke<void>("copy_text", { text }),
  reveal: (path: string) => invoke<void>("reveal", { path }),
};
