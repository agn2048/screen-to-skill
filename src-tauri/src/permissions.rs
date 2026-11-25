use block2::RcBlock;
use objc2::{class, msg_send, runtime::Bool};
use objc2_foundation::NSString;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}

#[link(name = "AVFoundation", kind = "framework")]
extern "C" {}

pub fn ensure_screen() -> bool {
    unsafe { CGPreflightScreenCaptureAccess() || CGRequestScreenCaptureAccess() }
}

pub fn ensure_microphone() -> bool {
    let audio = NSString::from_str("soun");
    let status: isize =
        unsafe { msg_send![class!(AVCaptureDevice), authorizationStatusForMediaType: &*audio] };
    match status {
        3 => true,
        0 => {
            let (tx, rx) = std::sync::mpsc::channel();
            let done = RcBlock::new(move |ok: Bool| {
                let _ = tx.send(ok.as_bool());
            });
            let () = unsafe {
                msg_send![class!(AVCaptureDevice), requestAccessForMediaType: &*audio, completionHandler: &*done]
            };
            rx.recv().unwrap_or(false)
        }
        _ => false,
    }
}

#[tauri::command]
pub fn open_privacy_settings(pane: String) -> Result<(), String> {
    let anchor = match pane.as_str() {
        "screen" => "Privacy_ScreenCapture",
        "microphone" => "Privacy_Microphone",
        _ => return Err("unknown pane".into()),
    };
    std::process::Command::new("/usr/bin/open")
        .arg(format!("x-apple.systempreferences:com.apple.preference.security?{anchor}"))
        .status()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
