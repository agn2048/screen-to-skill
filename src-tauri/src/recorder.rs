use crate::permissions;
use serde::Serialize;
use std::{
    io::Read,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};

struct Active {
    child: Child,
    path: PathBuf,
    stopping: bool,
}

#[derive(Default)]
pub struct Recorder(Mutex<Option<Active>>);

#[derive(Clone, Serialize)]
pub struct RecordingEvent {
    state: &'static str,
    path: Option<String>,
    message: Option<String>,
    help: Option<&'static str>,
}

fn event(state: &'static str) -> RecordingEvent {
    RecordingEvent { state, path: None, message: None, help: None }
}

fn notify(app: &AppHandle, e: RecordingEvent) {
    crate::update_tray(app, e.state == "recording");
    if e.state == "recording" {
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.hide();
        }
    } else {
        crate::show_main(app);
    }
    let _ = app.emit("recording", e);
}

fn fail(app: &AppHandle, message: &str, help: Option<&'static str>) {
    notify(app, RecordingEvent { message: Some(message.into()), help, ..event("error") });
}

impl Recorder {
    pub fn is_recording(&self) -> bool {
        self.0.lock().unwrap().is_some()
    }
}

pub fn start(app: &AppHandle, mic: bool) {
    let state = app.state::<Recorder>();
    if state.is_recording() {
        return;
    }
    if !permissions::ensure_screen() {
        return fail(app, "Allow ScreenToSkill under Screen Recording, then quit and reopen the app.", Some("screen"));
    }
    if mic && !permissions::ensure_microphone() {
        return fail(app, "Allow ScreenToSkill under Microphone, or turn narration off.", Some("microphone"));
    }
    let dir = match app.path().video_dir() {
        Ok(d) => d.join("ScreenToSkill"),
        Err(e) => return fail(app, &e.to_string(), None),
    };
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return fail(app, &format!("Cannot create {}: {e}", dir.display()), None);
    }
    let path = dir.join(chrono::Local::now().format("recording-%Y%m%d-%H%M%S.mov").to_string());

    let mut cmd = Command::new("/usr/sbin/screencapture");
    cmd.args(["-v", "-x", "-k"]);
    if mic {
        cmd.arg("-g");
    }
    {
        let mut slot = state.0.lock().unwrap();
        if slot.is_some() {
            return;
        }
        match cmd.arg(&path).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::piped()).spawn() {
            Ok(child) => *slot = Some(Active { child, path, stopping: false }),
            Err(e) => {
                drop(slot);
                return fail(app, &format!("Cannot start screencapture: {e}"), None);
            }
        }
    }
    notify(app, event("recording"));
    let app = app.clone();
    thread::spawn(move || watch(app));
}

fn watch(app: AppHandle) {
    let state = app.state::<Recorder>();
    let mut active = loop {
        thread::sleep(Duration::from_millis(200));
        let mut slot = state.0.lock().unwrap();
        match slot.as_mut().map(|a| a.child.try_wait()) {
            None => return,
            Some(Ok(None)) => continue,
            Some(_) => break slot.take().unwrap(),
        }
    };
    let ok = std::fs::metadata(&active.path).map(|m| m.len() > 0).unwrap_or(false);
    if ok {
        let path = Some(active.path.to_string_lossy().into_owned());
        let message = (!active.stopping).then(|| "macOS ended the recording.".to_string());
        notify(&app, RecordingEvent { path, message, ..event("stopped") });
    } else {
        let mut err = String::new();
        if let Some(mut s) = active.child.stderr.take() {
            let _ = s.read_to_string(&mut err);
        }
        let detail = err.lines().filter(|l| !l.starts_with("type any character")).collect::<Vec<_>>().join(" ");
        fail(&app, &format!("No recording was saved. Check the Screen Recording permission, then quit and reopen the app. {detail}"), Some("screen"));
    }
}

fn interrupt(a: &Active) {
    unsafe { libc::kill(a.child.id() as libc::pid_t, libc::SIGINT) };
}

pub fn stop(app: &AppHandle) {
    if let Some(a) = app.state::<Recorder>().0.lock().unwrap().as_mut() {
        a.stopping = true;
        interrupt(a);
    }
}

pub fn finish(app: &AppHandle) {
    let taken = app.state::<Recorder>().0.lock().unwrap().take();
    if let Some(mut a) = taken {
        interrupt(&a);
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline && matches!(a.child.try_wait(), Ok(None)) {
            thread::sleep(Duration::from_millis(100));
        }
        let _ = a.child.kill();
    }
}

#[tauri::command(async)]
pub fn start_recording(app: AppHandle, mic: bool) {
    start(&app, mic);
}

#[tauri::command]
pub fn stop_recording(app: AppHandle) {
    stop(&app);
}
