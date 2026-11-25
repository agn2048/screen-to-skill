import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { ask, open } from "@tauri-apps/plugin-dialog";
import { useEffect, useRef, useState } from "react";
import {
  baseUrlOf, call, Dependencies, EngineEvent, Hardware, leavesDevice, Output, provider, RecordingEvent, Settings,
} from "./api";
import SettingsPanel from "./SettingsPanel";

const STAGES: [string, string][] = [
  ["frames", "Extract frames"],
  ["transcribe", "Transcribe narration"],
  ["ocr", "Read on-screen text"],
  ["analyze", "Analyze frames"],
  ["synthesize", "Write SKILL.md"],
];
const VIDEO = /\.(mov|mp4|m4v|mkv|webm)$/i;
const fileName = (p: string) => p.split("/").pop() ?? p;
const clock = (n: number) => `${String(Math.floor(n / 60)).padStart(2, "0")}:${String(n % 60).padStart(2, "0")}`;

export default function App() {
  const [tab, setTab] = useState<"home" | "settings">("home");
  const [settings, setSettings] = useState<Settings | null>(null);
  const [hw, setHw] = useState<Hardware | null>(null);
  const [deps, setDeps] = useState<Dependencies | null>(null);
  const [recording, setRecording] = useState(false);
  const [seconds, setSeconds] = useState(0);
  const [video, setVideo] = useState("");
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<EngineEvent | null>(null);
  const [warnings, setWarnings] = useState<string[]>([]);
  const [error, setError] = useState("");
  const [help, setHelp] = useState<RecordingEvent["help"]>(null);
  const [output, setOutput] = useState<Output | null>(null);
  const [text, setText] = useState("");
  const [view, setView] = useState<"result" | "index">("result");
  const [toast, setToast] = useState("");
  const busyRef = useRef(false);
  busyRef.current = busy || recording;

  function fail(e: unknown) {
    setError(String(e));
    setHelp(null);
  }

  useEffect(() => {
    call.getSettings().then(setSettings).catch(fail);
    call.hardware().then(setHw).catch(fail);
    const subs = [
      listen<RecordingEvent>("recording", ({ payload: e }) => {
        setRecording(e.state === "recording");
        setError(e.state === "error" ? e.message ?? "Recording failed." : "");
        setHelp(e.help);
        if (e.state === "recording") setSeconds(0);
        if (e.state === "stopped" && e.path) {
          pick(e.path);
          setTab("home");
        }
      }),
      listen<EngineEvent>("engine", ({ payload: e }) => {
        if (e.type === "warning") setWarnings((w) => (w.includes(e.message) ? w : [...w, e.message]));
        else setProgress(e);
      }),
      getCurrentWebview().onDragDropEvent(({ payload: e }) => {
        if (e.type !== "drop" || busyRef.current) return;
        const p = e.paths.find((x) => VIDEO.test(x));
        if (p) pick(p);
      }),
    ];
    return () => subs.forEach((s) => void s.then((off) => off()));
  }, []);

  useEffect(() => { call.dependencies().then(setDeps).catch(fail); }, [tab]);

  useEffect(() => {
    if (!recording) return;
    const t = setInterval(() => setSeconds((n) => n + 1), 1000);
    return () => clearInterval(t);
  }, [recording]);

  useEffect(() => {
    if (!toast) return;
    const t = setTimeout(() => setToast(""), 4000);
    return () => clearTimeout(t);
  }, [toast]);

  if (!settings) return <main className="empty">{error || "Loading..."}</main>;

  const sent = leavesDevice(settings);
  const local = sent.length === 0;
  const p = provider(settings.provider);
  const stages = STAGES.filter(([id]) => settings.provider !== "none" || !["analyze", "synthesize"].includes(id));
  const stageIndex = stages.findIndex(([id]) => id === progress?.stage);

  function update(s: Settings) {
    setSettings(s);
    call.saveSettings(s).catch(fail);
  }

  function pick(path: string) {
    setVideo(path);
    setOutput(null);
    setError("");
    setWarnings([]);
  }

  async function choose() {
    const f = await open({ filters: [{ name: "Video", extensions: ["mov", "mp4", "m4v", "mkv", "webm"] }] });
    if (typeof f === "string") pick(f);
  }

  async function convert() {
    setBusy(true);
    setError("");
    setHelp(null);
    setWarnings([]);
    setProgress(null);
    setOutput(null);
    try {
      const out = await call.convert(video, settings!, local);
      setOutput(out);
      setText(out.content);
      setView("result");
    } catch (e) {
      if (String(e) === "Cancelled.") setToast("Cancelled");
      else fail(e);
    } finally {
      setBusy(false);
    }
  }

  async function save(dir: string | null) {
    if (!output) return;
    const edited = output.kind === "skill" ? /^---\n(?:.*\n)*?name:\s*["']?([a-z0-9-]+)["']?\s*\n/.exec(text)?.[1] : null;
    const args = { name: edited ?? output.name, kind: output.kind, content: text, dir };
    try {
      let path: string;
      try {
        path = await call.export({ ...args, overwrite: false });
      } catch (e) {
        if (e !== "EXISTS") throw e;
        if (!(await ask(`"${args.name}" already exists there. Replace it?`, { title: "ScreenToSkill", kind: "warning" }))) return;
        path = await call.export({ ...args, overwrite: true });
      }
      setToast(`Saved to ${path}`);
    } catch (e) {
      fail(e);
    }
  }

  async function saveTo() {
    const dir = await open({ directory: true });
    if (typeof dir === "string") await save(dir);
  }

  const missing = deps && (!deps.ffmpeg ? "FFmpeg is missing. In Terminal, run: brew install ffmpeg"
    : !deps.python ? "Python 3 is missing. In Terminal, run: brew install python" : "");

  return (
    <>
      <header>
        <nav>
          <button className={tab === "home" ? "on" : ""} onClick={() => setTab("home")}>Create</button>
          <button className={tab === "settings" ? "on" : ""} onClick={() => setTab("settings")} disabled={busy}>Settings</button>
        </nav>
        <span className={`badge ${local ? "local" : "cloud"}`}
          title={local ? "Nothing leaves this Mac. The engine refuses remote connections for your data."
            : `Sent off this Mac: ${sent.join(" and ")}`}>
          {local ? "Local Only" : `Cloud API: ${sent.join(" + ")}`}
        </span>
      </header>

      <main>
        {error && (
          <div className="alert" role="alert">
            <span>{error}</span>
            {help && <button onClick={() => call.openPrivacy(help)}>Open System Settings</button>}
            <button onClick={() => setError("")}>Dismiss</button>
          </div>
        )}
        {tab === "settings" ? (
          <SettingsPanel settings={settings} hw={hw} deps={deps} onChange={update} />
        ) : (
          <>
            {missing && <div className="alert"><span>{missing}</span></div>}
            <section className="capture">
              {recording ? (
                <button className="record live" onClick={() => call.stopRecording()}><i />Stop ({clock(seconds)})</button>
              ) : (
                <button className="record" disabled={busy} onClick={() => call.startRecording(settings.mic).catch(fail)}><i />Record</button>
              )}
              <label className="check">
                <input type="checkbox" checked={settings.mic} disabled={recording}
                  onChange={(e) => update({ ...settings, mic: e.target.checked })} />
                Record narration
              </label>
              <span className="grow" />
              <button disabled={busy || recording} onClick={choose}>Choose video...</button>
            </section>
            {!video && !recording && (
              <p className="muted">
                Press Record and talk through what you do. The window hides; stop from the REC item in the menu bar.
                You can also drop a video here.
              </p>
            )}

            {video && !recording && (
              <section className="job">
                <div className="row">
                  <strong title={video}>{fileName(video)}</strong>
                  <button className="link" onClick={() => call.reveal(video).catch(fail)}>Show in Finder</button>
                  <span className="grow" />
                  {busy ? (
                    <button onClick={() => call.cancel()}>Cancel</button>
                  ) : (
                    <button className="primary" disabled={!!missing} onClick={convert}>
                      {output ? "Convert again" : settings.provider === "none" ? "Create Markdown" : "Create SKILL.md"}
                    </button>
                  )}
                </div>
                <p className="muted">
                  {local ? "Processed on this Mac." : `Sends ${sent.join(" and ")} to ${[
                    sent.includes("screen frames") ? (p.id === "custom" ? baseUrlOf(settings) : p.label.split(" (")[0]) : "",
                    settings.transcriber === "cloud" ? "Groq" : "",
                  ].filter((x, i, a) => x && a.indexOf(x) === i).join(" and ")}.`}
                </p>
                {busy && (
                  <ol className="stages">
                    {stages.map(([id, label], i) => (
                      <li key={id} className={i < stageIndex ? "done" : i === stageIndex ? "now" : ""}>
                        <span>{i === stageIndex ? progress?.message : label}</span>
                        {i === stageIndex && (progress?.total
                          ? <progress value={progress.current} max={progress.total} /> : <progress />)}
                      </li>
                    ))}
                  </ol>
                )}
                {warnings.map((w) => <p key={w} className="warn">{w}</p>)}
              </section>
            )}

            {output && (
              <section className="result">
                <div className="row">
                  <nav>
                    <button className={view === "result" ? "on" : ""} onClick={() => setView("result")}>
                      {output.kind === "skill" ? "SKILL.md" : "Prompt for any LLM"}
                    </button>
                    <button className={view === "index" ? "on" : ""} onClick={() => setView("index")}>Workflow index ({output.steps} steps)</button>
                  </nav>
                  <span className="grow" />
                  <button className="primary" onClick={() => call.copy(view === "index" ? output.index : text).then(() => setToast("Copied")).catch(fail)}>Copy</button>
                  {output.kind === "skill" && <button onClick={() => save(null)}>Save to ~/.claude/skills</button>}
                  <button onClick={saveTo}>Save to folder...</button>
                </div>
                <textarea spellCheck={false} aria-label="Result" readOnly={view === "index"}
                  value={view === "index" ? output.index : text} onChange={(e) => setText(e.target.value)} />
                <p className="muted">Review before sharing: recordings can capture private information.</p>
              </section>
            )}
          </>
        )}
      </main>
      {toast && <div className="toast" role="status">{toast}</div>}
    </>
  );
}
