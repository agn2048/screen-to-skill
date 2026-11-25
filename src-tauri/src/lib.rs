mod engine;
mod export;
mod hardware;
mod keychain;
mod permissions;
mod recorder;
mod settings;

use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{TrayIcon, TrayIconBuilder},
    AppHandle, Manager, RunEvent, WindowEvent, Wry,
};

struct Tray {
    icon: TrayIcon,
    toggle: MenuItem<Wry>,
}

pub(crate) fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

pub(crate) fn update_tray(app: &AppHandle, recording: bool) {
    let app2 = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(t) = app2.try_state::<Tray>() {
            let _ = t.toggle.set_text(if recording { "Stop Recording" } else { "Start Recording" });
            let _ = t.icon.set_title(recording.then_some("REC"));
        }
    });
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let toggle = MenuItem::with_id(app, "toggle", "Start Recording", true, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", "Open ScreenToSkill", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle, &show, &quit])?;
    let icon = TrayIconBuilder::new()
        .icon(Image::from_bytes(include_bytes!("../icons/tray.png"))?)
        .icon_as_template(true)
        .tooltip("ScreenToSkill")
        .menu(&menu)
        .on_menu_event(|app, e| match e.id.as_ref() {
            "toggle" => {
                if app.state::<recorder::Recorder>().is_recording() {
                    recorder::stop(app);
                } else {
                    let app = app.clone();
                    std::thread::spawn(move || recorder::start(&app, settings::load(&app).mic));
                }
            }
            "show" => show_main(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    app.manage(Tray { icon, toggle });
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(recorder::Recorder::default())
        .manage(engine::Job::default())
        .invoke_handler(tauri::generate_handler![
            hardware::hardware,
            keychain::set_api_key,
            keychain::has_api_key,
            keychain::delete_api_key,
            permissions::open_privacy_settings,
            recorder::start_recording,
            recorder::stop_recording,
            engine::check_dependencies,
            engine::list_models,
            engine::convert,
            engine::cancel_convert,
            export::export,
            export::copy_text,
            export::reveal,
            settings::get_settings,
            settings::save_settings,
        ])
        .setup(|app| Ok(build_tray(app.handle())?))
        .on_window_event(|w, e| {
            if let WindowEvent::CloseRequested { api, .. } = e {
                api.prevent_close();
                let _ = w.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building ScreenToSkill")
        .run(|app, e| match e {
            RunEvent::Reopen { .. } => show_main(app),
            RunEvent::Exit => {
                recorder::finish(app);
                engine::cancel(app);
            }
            _ => {}
        });
}
