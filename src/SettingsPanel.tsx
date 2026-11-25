import { useEffect, useState } from "react";
import { baseUrlOf, call, Dependencies, Hardware, provider, PROVIDERS, Settings } from "./api";

const WHISPER = [
  ["tiny", "Tiny (75 MB, fastest)"],
  ["base", "Base (150 MB, recommended)"],
  ["small", "Small (490 MB, more accurate)"],
  ["large-v3-turbo-q5_0", "Large v3 Turbo (570 MB, best)"],
];

interface Props {
  settings: Settings;
  hw: Hardware | null;
  deps: Dependencies | null;
  onChange: (s: Settings) => void;
}

function KeyField({ account, label }: { account: string; label: string }) {
  const [saved, setSaved] = useState<boolean | null>(null);
  const [draft, setDraft] = useState("");
  const [error, setError] = useState("");
  useEffect(() => {
    let live = true;
    setDraft("");
    setError("");
    setSaved(null);
    call.hasKey(account).then((v) => live && setSaved(v)).catch((e) => live && setError(String(e)));
    return () => { live = false; };
  }, [account]);

  async function act(fn: () => Promise<void>, value: boolean) {
    try {
      await fn();
      setSaved(value);
      setDraft("");
      setError("");
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <label className="field">
      <span>{label}</span>
      {saved ? (
        <div className="row">
          <em className="ok">Saved in Keychain</em>
          <button onClick={() => act(() => call.deleteKey(account), false)}>Remove</button>
        </div>
      ) : (
        <div className="row">
          <input type="password" autoComplete="off" spellCheck={false} value={draft} placeholder="Paste key"
            onChange={(e) => setDraft(e.target.value)} />
          <button disabled={!draft.trim()} onClick={() => act(() => call.setKey(account, draft), true)}>Save</button>
        </div>
      )}
      {error && <small className="bad">{error}</small>}
    </label>
  );
}

export default function SettingsPanel({ settings: s, hw, deps, onChange }: Props) {
  const p = provider(s.provider);
  const [models, setModels] = useState<string[]>([]);
  const [modelError, setModelError] = useState("");
  const set = (patch: Partial<Settings>) => onChange({ ...s, ...patch });
  useEffect(() => { setModels([]); setModelError(""); }, [s.provider]);

  async function loadModels() {
    setModelError("");
    try {
      setModels(await call.models(s));
    } catch (e) {
      setModelError(String(e));
    }
  }

  return (
    <div className="settings">
      <section>
        <h2>This Mac</h2>
        {hw && <p>{hw.chip}, {hw.ram_gb} GB ({hw.arch})</p>}
        {deps && (
          <ul className="checks">
            <li className={deps.ffmpeg ? "ok" : "bad"}>FFmpeg {deps.ffmpeg ? "found" : "missing: brew install ffmpeg"}</li>
            <li className={deps.python ? "ok" : "bad"}>Python 3 {deps.python ? "found" : "missing: brew install python"}</li>
            <li className={deps.whisper ? "ok" : "bad"}>Local transcription {deps.whisper ? "bundled" : "not bundled in this build"}</li>
            <li className={deps.ocr ? "ok" : "bad"}>On-screen text reader {deps.ocr ? "bundled" : "not bundled in this build"}</li>
          </ul>
        )}
        {hw && (
          <div className="tip">
            <span>{hw.reason}</span>
            <button onClick={() => set({
              provider: hw.provider,
              models: hw.model ? { ...s.models, [hw.provider]: hw.model } : s.models,
            })}>Use {provider(hw.provider).label}{hw.model && ` with ${hw.model}`}</button>
          </div>
        )}
      </section>

      <section>
        <h2>AI provider</h2>
        <label className="field">
          <span>Provider</span>
          <select value={s.provider} onChange={(e) => set({ provider: e.target.value })}>
            {PROVIDERS.map((x) => <option key={x.id} value={x.id}>{x.label}</option>)}
          </select>
          <small>{p.hint}</small>
        </label>
        {s.provider !== "none" && (
          <>
            {p.editableUrl && (
              <label className="field">
                <span>Base URL</span>
                <input spellCheck={false} value={s.base_urls[s.provider] ?? ""} placeholder={p.baseUrl || "https://host/v1"}
                  onChange={(e) => set({ base_urls: { ...s.base_urls, [s.provider]: e.target.value } })} />
              </label>
            )}
            {p.key && <KeyField account={s.provider} label={s.provider === "custom" ? "API key (optional)" : "API key"} />}
            <label className="field">
              <span>Model (must accept images)</span>
              <div className="row">
                <input list="models" spellCheck={false} value={s.models[s.provider] ?? ""} placeholder={p.model || "model name"}
                  onChange={(e) => set({ models: { ...s.models, [s.provider]: e.target.value } })} />
                <button disabled={!baseUrlOf(s)} onClick={loadModels}>List models</button>
              </div>
              <datalist id="models">{models.map((m) => <option key={m} value={m} />)}</datalist>
              {models.length > 0 && <small>{models.length} available. Type to filter.</small>}
              {modelError && <small className="bad">{modelError}</small>}
            </label>
          </>
        )}
      </section>

      <section>
        <h2>Narration</h2>
        <label className="field">
          <span>Transcription</span>
          <select value={s.transcriber} onChange={(e) => set({ transcriber: e.target.value })}>
            <option value="auto">On this Mac (Whisper)</option>
            <option value="cloud">Groq cloud (fast on Intel Macs)</option>
            <option value="none">Off</option>
          </select>
        </label>
        {s.transcriber === "auto" && (
          <label className="field">
            <span>Whisper model</span>
            <select value={s.whisper_model} onChange={(e) => set({ whisper_model: e.target.value })}>
              {WHISPER.map(([v, l]) => <option key={v} value={v}>{l}</option>)}
            </select>
            <small>Downloaded once, on first use.</small>
          </label>
        )}
        {s.transcriber === "cloud" && s.provider !== "groq" && <KeyField account="groq" label="Groq API key" />}
        <label className="field">
          <span>Language</span>
          <input spellCheck={false} value={s.language} placeholder="auto, en, hi, de..." onChange={(e) => set({ language: e.target.value.trim() })} />
        </label>
      </section>

      <section>
        <h2>Keyframes</h2>
        <label className="field">
          <span>Detail</span>
          <select value={s.sensitivity} onChange={(e) => set({ sensitivity: Number(e.target.value) })}>
            <option value={0.02}>High: small changes too</option>
            <option value={0.03}>Normal</option>
            <option value={0.1}>Low: big changes only</option>
          </select>
        </label>
        <label className="field">
          <span>Maximum frames</span>
          <select value={s.max_frames} onChange={(e) => set({ max_frames: Number(e.target.value) })}>
            {[15, 40, 80, 150].map((n) => <option key={n} value={n}>{n}</option>)}
          </select>
        </label>
      </section>
    </div>
  );
}
