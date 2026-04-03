use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

use super::player::{MpvPlayer, MpvState};

/// Shared mpv player, initialised lazily on first play.
pub struct MpvHandle(pub Mutex<Option<MpvPlayer>>);

/// Initialise the mpv player and start the event/render loops.
fn ensure_player(handle: &MpvHandle, app: &AppHandle) -> Result<(), String> {
    let mut lock = handle.0.lock().map_err(|e| e.to_string())?;
    if lock.is_some() {
        return Ok(());
    }

    let window = app
        .get_webview_window("main")
        .ok_or("main window not found")?;

    let mut player = MpvPlayer::new(&window, app.clone())?;
    player.start();
    *lock = Some(player);
    Ok(())
}

// ── Tauri commands ─────────────────────────────────────────────

#[tauri::command]
pub fn mpv_play(
    url: String,
    handle: State<'_, MpvHandle>,
    app: AppHandle,
) -> Result<(), String> {
    ensure_player(&handle, &app)?;
    let lock = handle.0.lock().map_err(|e| e.to_string())?;
    lock.as_ref().unwrap().load_file(&url)
}

#[tauri::command]
pub fn mpv_stop(handle: State<'_, MpvHandle>) -> Result<(), String> {
    let mut lock = handle.0.lock().map_err(|e| e.to_string())?;
    // Drop the player — triggers MpvPlayer::Drop which joins threads,
    // frees the mpv context, and removes the NSView from the window.
    drop(lock.take());
    Ok(())
}

#[tauri::command]
pub fn mpv_toggle_pause(handle: State<'_, MpvHandle>) -> Result<(), String> {
    let lock = handle.0.lock().map_err(|e| e.to_string())?;
    lock.as_ref().ok_or("mpv not initialised")?.toggle_pause()
}

#[tauri::command]
pub fn mpv_set_pause(paused: bool, handle: State<'_, MpvHandle>) -> Result<(), String> {
    let lock = handle.0.lock().map_err(|e| e.to_string())?;
    lock.as_ref().ok_or("mpv not initialised")?.set_pause(paused)
}

#[tauri::command]
pub fn mpv_seek(seconds: f64, handle: State<'_, MpvHandle>) -> Result<(), String> {
    let lock = handle.0.lock().map_err(|e| e.to_string())?;
    lock.as_ref().ok_or("mpv not initialised")?.seek_absolute(seconds)
}

#[tauri::command]
pub fn mpv_set_volume(volume: f64, handle: State<'_, MpvHandle>) -> Result<(), String> {
    let lock = handle.0.lock().map_err(|e| e.to_string())?;
    lock.as_ref().ok_or("mpv not initialised")?.set_volume(volume)
}

#[tauri::command]
pub fn mpv_set_mute(muted: bool, handle: State<'_, MpvHandle>) -> Result<(), String> {
    let lock = handle.0.lock().map_err(|e| e.to_string())?;
    lock.as_ref().ok_or("mpv not initialised")?.set_mute(muted)
}

#[tauri::command]
pub fn mpv_get_state(handle: State<'_, MpvHandle>) -> Result<MpvState, String> {
    let lock = handle.0.lock().map_err(|e| e.to_string())?;
    Ok(lock.as_ref().map(|p| p.get_state()).unwrap_or_default())
}

#[tauri::command]
pub fn mpv_get_tracks(handle: State<'_, MpvHandle>) -> Result<String, String> {
    let lock = handle.0.lock().map_err(|e| e.to_string())?;
    lock.as_ref().ok_or("mpv not initialised")?.get_tracks()
}

#[tauri::command]
pub fn mpv_set_track(track_type: String, id: i64, handle: State<'_, MpvHandle>) -> Result<(), String> {
    let lock = handle.0.lock().map_err(|e| e.to_string())?;
    lock.as_ref().ok_or("mpv not initialised")?.set_track(&track_type, id)
}

#[tauri::command]
pub fn mpv_update_context(handle: State<'_, MpvHandle>) -> Result<(), String> {
    let lock = handle.0.lock().map_err(|e| e.to_string())?;
    if let Some(player) = lock.as_ref() {
        player.update_gl_context();
    }
    Ok(())
}
