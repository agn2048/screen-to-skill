use crate::{keychain, settings::Settings};
use serde::Serialize;
use serde_json::Value;
use std::{
    io::{BufRead, BufReader, Read},
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Mutex,
    thread,
};
use tauri::{path::BaseDirectory, AppHandle, Emitter, Manager};

#[derive(Default)]
pub struct Job(Mutex<JobState>);

#[derive(Default)]
struct JobState {
    running: bool,
    cancelled: bool,
    pid: Option<u32>,
}

#[derive(Serialize)]
pub struct Output {
    kind: &'static str,
    name: String,
    content: String,
    index: String,
    steps: u64,
    folder: String,
}

#[derive(Serialize)]
pub struct Dependencies {
    python: Option<String>,
    ffmpeg: Option<String>,
    whisper: bool,
    ocr: bool,
}

fn first_executable(paths: &[&str]) -> Option<String> {
    paths.iter().find(|p| Path::new(p).exists()).map(|p| p.to_string())
}

fn python() -> Option<String> {
    first_executable(&[
        "/opt/homebrew/bin/python3",
        "/usr/local/bin/python3",
        "/Library/Frameworks/Python.framework/Versions/Current/bin/python3",
    ])
    .or_else(|| {
        let clt = Command::new("/usr/bin/xcode-select").arg("-p").output().ok()?;
        clt.status.success().then(|| "/usr/bin/python3".to_string())
    })
}

fn ffmpeg() -> Option<String> {
    first_executable(&["/opt/homebrew/bin/ffmpeg", "/usr/local/bin/ffmpeg"])
}

fn sidecar(name: &str) -> Option<PathBuf> {
    let p = std::env::current_exe().ok()?.parent()?.join(name);
    p.exists().then_some(p)
}

fn base_command(app: &AppHandle) -> Result<Command, String> {
    let py = python().ok_or("Python 3 was not found. Install it with: brew install python")?;
    let script = app.path().resolve("engine/video_to_index.py", BaseDirectory::Resource).map_err(|e| e.to_string())?;
    let mut cmd = Command::new(py);
    cmd.args(["-I", "-X", "utf8"]).arg(script);
    cmd.env("PATH", "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    Ok(cmd)
}

fn provider_args(cmd: &mut Command, s: &Settings) -> Result<(), String> {
    cmd.args(["--provider", &s.provider]);
    if matches!(s.provider.as_str(), "ollama" | "mlx" | "custom") && !s.base_url().is_empty() {
        cmd.arg(format!("--base-url={}", s.base_url()));
    }
    if !s.model().is_empty() {
        cmd.arg(format!("--model={}", s.model()));
    }
    if let Some(key) = keychain::get(&s.provider)? {
        cmd.env("STS_API_KEY", key);
    } else if matches!(s.provider.as_str(), "groq" | "gemini" | "openrouter") {
        return Err(format!("Add your {} API key in Settings first.", s.provider));
    }
    Ok(())
}

#[tauri::command(async)]
pub fn check_dependencies() -> Dependencies {
    Dependencies {
        python: python(),
        ffmpeg: ffmpeg(),
        whisper: sidecar("whisper-cli").is_some(),
        ocr: sidecar("ocr-helper").is_some(),
    }
}

#[tauri::command(async)]
pub fn list_models(app: AppHandle, settings: Settings) -> Result<Vec<String>, String> {
    let mut cmd = base_command(&app)?;
    cmd.arg("models");
    provider_args(&mut cmd, &settings)?;
    let out = cmd.output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(err.trim().trim_start_matches("error: ").to_string());
    }
    serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub fn convert(app: AppHandle, video: String, settings: Settings, local_only: bool) -> Result<Output, String> {
    let job = app.state::<Job>();
    {
        let mut st = job.0.lock().unwrap();
        if st.running {
            return Err("A conversion is already running.".into());
        }
        *st = JobState { running: true, ..Default::default() };
    }
    let result = run_convert(&app, &video, &settings, local_only);
    let mut st = job.0.lock().unwrap();
    let cancelled = st.cancelled;
    *st = JobState::default();
    if cancelled {
        return Err("Cancelled.".into());
    }
    result
}

fn run_convert(app: &AppHandle, video: &str, s: &Settings, local_only: bool) -> Result<Output, String> {
    let stem = Path::new(video).file_stem().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
    let folder = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("skills")
        .join(format!("{}-{}", chrono::Local::now().format("%Y%m%d-%H%M%S"), stem));

    let mut cmd = base_command(app)?;
    cmd.args(["convert", "--json"]);
    provider_args(&mut cmd, s)?;
    if local_only {
        cmd.arg("--local-only");
    }
    cmd.args(["--transcriber", &s.transcriber, "--whisper-model", &s.whisper_model]);
    cmd.arg(format!("--language={}", if s.language.is_empty() { "auto" } else { &s.language }));
    cmd.arg(format!("--sensitivity={}", s.sensitivity)).arg(format!("--max-frames={}", s.max_frames));
    if s.transcriber == "cloud" {
        let key = keychain::get("groq")?.ok_or("Cloud transcription needs a Groq API key. Add it in Settings.")?;
        cmd.env("STS_STT_API_KEY", key);
    }
    if let Some(p) = sidecar("whisper-cli") {
        cmd.arg("--whisper-cli").arg(p);
    }
    if let Some(p) = sidecar("ocr-helper") {
        cmd.arg("--ocr-helper").arg(p);
    }
    let template = app.path().resolve("templates/skill_prompt.md", BaseDirectory::Resource).map_err(|e| e.to_string())?;
    cmd.arg("--prompt-template").arg(template).arg("-o").arg(&folder).arg("--").arg(video);

    let mut child = cmd.process_group(0).spawn().map_err(|e| format!("Cannot start the engine: {e}"))?;
    {
        let job = app.state::<Job>();
        let mut st = job.0.lock().unwrap();
        st.pid = Some(child.id());
        if st.cancelled {
            terminate(child.id());
        }
    }
    let mut stderr = child.stderr.take().unwrap();
    let errors = thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stderr.read_to_end(&mut buf);
        String::from_utf8_lossy(&buf).into_owned()
    });
    let (mut result, mut error) = (None, None);
    for line in BufReader::new(child.stdout.take().unwrap()).lines().map_while(Result::ok) {
        let Ok(v) = serde_json::from_str::<Value>(&line) else { continue };
        match v["type"].as_str() {
            Some("result") => result = Some(v),
            Some("error") => error = v["message"].as_str().map(String::from),
            Some(_) => {
                let _ = app.emit("engine", v);
            }
            None => {}
        }
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    let errors = errors.join().unwrap_or_default();
    if let Some(e) = error {
        return Err(e);
    }
    let Some(r) = result.filter(|_| status.success()) else {
        let tail: Vec<_> = errors.trim().lines().rev().take(3).collect();
        return Err(format!("The engine stopped unexpectedly. {}", tail.into_iter().rev().collect::<Vec<_>>().join(" ")));
    };
    let (kind, path) = match r["skill"].as_str() {
        Some(p) => ("skill", p),
        None => ("prompt", r["prompt"].as_str().unwrap_or_default()),
    };
    let read = |p: &str| std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}"));
    Ok(Output {
        kind,
        name: r["name"].as_str().unwrap_or("recorded-workflow").to_string(),
        content: read(path)?,
        index: read(r["index"].as_str().unwrap_or_default())?,
        steps: r["steps"].as_u64().unwrap_or(0),
        folder: folder.to_string_lossy().into_owned(),
    })
}

fn terminate(pid: u32) {
    unsafe { libc::kill(-(pid as libc::pid_t), libc::SIGTERM) };
}

pub fn cancel(app: &AppHandle) {
    let job = app.state::<Job>();
    let mut st = job.0.lock().unwrap();
    if st.running {
        st.cancelled = true;
        if let Some(pid) = st.pid {
            terminate(pid);
        }
    }
}

#[tauri::command]
pub fn cancel_convert(app: AppHandle) {
    cancel(&app);
}
