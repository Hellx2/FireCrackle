//! Thin Tauri shim over `studio_core::workspace`: one open video at a time, every command a one-liner.
//! Anything that can block (importing, decoding) runs on a blocking thread so the window stays responsive.

use std::path::Path;
use std::sync::{Arc, Mutex};
use studio_core::config::{SettingInfo, Settings};
use studio_core::import::Tools;
use studio_core::lane::Bucket;
use studio_core::phase::Flag;
use studio_core::workspace::{PhaseChart, UiAction, View, Workspace};
use tauri::ipc::Response;
use tauri::State;

#[derive(Default)]
struct AppState {
    ws: Arc<Mutex<Option<Workspace>>>,
}

fn err<E: std::fmt::Debug>(e: E) -> String {
    format!("{e:?}")
}

/// Run `f` on the open workspace on a blocking thread.
async fn run<T: Send + 'static>(
    ws: Arc<Mutex<Option<Workspace>>>,
    f: impl FnOnce(&mut Workspace) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = ws.lock().map_err(|_| "internal error: the workspace lock was poisoned".to_string())?;
        let w = guard.as_mut().ok_or_else(|| "no video is open".to_string())?;
        f(w)
    })
    .await
    .map_err(err)?
}

/// Import (or load) the project for a video and make it the open workspace.
#[tauri::command]
async fn open_video(state: State<'_, AppState>, path: String) -> Result<View, String> {
    let ws = state.ws.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut w = Workspace::open(&Tools::default(), Path::new(&path), &Settings::default()).map_err(err)?;
        let view = w.view();
        let mut guard = ws.lock().map_err(|_| "internal error: the workspace lock was poisoned".to_string())?;
        if let Some(old) = guard.take() {
            old.wait_idle(); // let the previous video's prefetch finish before it is dropped
        }
        *guard = Some(w);
        Ok(view)
    })
    .await
    .map_err(err)?
}

#[tauri::command]
async fn view(state: State<'_, AppState>) -> Result<View, String> {
    run(state.ws.clone(), |w| Ok(w.view())).await
}

#[tauri::command]
async fn act(state: State<'_, AppState>, action: UiAction) -> Result<View, String> {
    run(state.ws.clone(), move |w| w.apply(action).map_err(err)).await
}

/// The JPEG bytes of one frame, sent raw (no JSON).
#[tauri::command]
async fn frame(state: State<'_, AppState>, frame: u32) -> Result<Response, String> {
    run(state.ws.clone(), move |w| w.frame_jpeg(frame).map(|b| Response::new((*b).clone())).map_err(err)).await
}

#[tauri::command]
async fn timeline(state: State<'_, AppState>, start: u32, end: u32, buckets: usize) -> Result<Vec<Bucket>, String> {
    run(state.ws.clone(), move |w| Ok(w.timeline(start, end, buckets))).await
}

#[tauri::command]
async fn flags(state: State<'_, AppState>) -> Result<Vec<Flag>, String> {
    run(state.ws.clone(), |w| Ok(w.flags())).await
}

#[tauri::command]
async fn phase_chart(state: State<'_, AppState>, max_points: usize) -> Result<PhaseChart, String> {
    run(state.ws.clone(), move |w| Ok(w.phase_chart(max_points))).await
}

#[tauri::command]
async fn settings_describe(state: State<'_, AppState>) -> Result<Vec<SettingInfo>, String> {
    run(state.ws.clone(), |w| Ok(w.describe_settings())).await
}

#[tauri::command]
async fn settings_set(state: State<'_, AppState>, key: String, value: f64) -> Result<View, String> {
    run(state.ws.clone(), move |w| w.set_setting(&key, value).map_err(err)).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run_app() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            open_video,
            view,
            act,
            frame,
            timeline,
            flags,
            phase_chart,
            settings_describe,
            settings_set
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
