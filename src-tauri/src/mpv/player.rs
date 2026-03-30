use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use objc2::rc::Retained;
#[allow(deprecated)] // OpenGL is deprecated on macOS but still functional
use objc2_app_kit::{NSOpenGLContext, NSView};
use tauri::{AppHandle, Emitter};

use super::ffi;

/// Serialisable snapshot of mpv state, emitted to the frontend.
#[derive(Clone, serde::Serialize)]
pub struct MpvState {
    pub playing: bool,
    pub time_pos: f64,
    pub duration: f64,
    pub volume: f64,
    pub muted: bool,
    pub idle: bool,
    pub paused_for_cache: bool,
    pub title: String,
}

impl Default for MpvState {
    fn default() -> Self {
        Self {
            playing: false,
            time_pos: 0.0,
            duration: 0.0,
            volume: 100.0,
            muted: false,
            idle: true,
            paused_for_cache: false,
            title: String::new(),
        }
    }
}

// Property‐observation user‐data IDs
const UD_PAUSE: u64 = 1;
const UD_TIME_POS: u64 = 2;
const UD_DURATION: u64 = 3;
const UD_VOLUME: u64 = 4;
const UD_MUTE: u64 = 5;
const UD_IDLE: u64 = 6;
const UD_MEDIA_TITLE: u64 = 7;
const UD_PAUSED_FOR_CACHE: u64 = 8;

/// Wrapper so we can send the raw pointer across threads.
/// mpv's client API is fully thread-safe per the documentation.
struct SendPtr<T> {
    ptr: *mut T,
}
impl<T> Clone for SendPtr<T> {
    fn clone(&self) -> Self { *self }
}
impl<T> Copy for SendPtr<T> {}
unsafe impl<T> Send for SendPtr<T> {}
unsafe impl<T> Sync for SendPtr<T> {}

impl<T> SendPtr<T> {
    fn get(self) -> *mut T {
        self.ptr
    }
}

/// Wrapper for Retained objects that aren't Send (NSView, NSOpenGLContext).
struct SendRetained<T>(Retained<T>);
unsafe impl<T> Send for SendRetained<T> {}
unsafe impl<T> Sync for SendRetained<T> {}

/// Signal used by the mpv update callback to wake the render thread.
struct RenderSignal {
    needs_render: Mutex<bool>,
    condvar: Condvar,
}

/// Wraps a libmpv instance with OpenGL render API, its embedded NSView,
/// and event/render loop threads.
pub struct MpvPlayer {
    ctx: SendPtr<ffi::mpv_handle>,
    _view: SendRetained<NSView>,
    _gl_ctx: SendRetained<NSOpenGLContext>,
    running: Arc<AtomicBool>,
    render_signal: Arc<RenderSignal>,
    event_thread: Option<std::thread::JoinHandle<()>>,
    render_thread: Option<std::thread::JoinHandle<()>>,
    app: AppHandle,
}

// SAFETY: MpvPlayer is guarded by a Mutex in MpvHandle.
// The mpv client API is thread-safe. NSView/NSOpenGLContext are only
// created/destroyed on the main thread; we hold them for ownership.
unsafe impl Send for MpvPlayer {}
unsafe impl Sync for MpvPlayer {}

impl MpvPlayer {
    /// Create + initialise mpv with the OpenGL render API, embed into the given Tauri window.
    ///
    /// The NSView and NSOpenGLContext are dispatched to the main thread for creation.
    pub fn new(window: &tauri::WebviewWindow, app: AppHandle) -> Result<Self, String> {
        let ns_window_addr =
            window.ns_window().map_err(|e| format!("ns_window: {e}"))? as usize;

        // Dispatch GL view creation to the main thread (macOS requirement)
        let (tx, rx) =
            std::sync::mpsc::sync_channel::<Result<super::embed_macos::GlViewData, String>>(1);

        window
            .run_on_main_thread(move || {
                let ns_window: &objc2_app_kit::NSWindow =
                    unsafe { &*(ns_window_addr as *const objc2_app_kit::NSWindow) };
                let result = super::embed_macos::create_gl_view(ns_window);
                let _ = tx.send(result);
            })
            .map_err(|e| format!("run_on_main_thread: {e}"))?;

        let gl_data = rx.recv().map_err(|e| format!("recv: {e}"))??;

        // ── Create mpv ─────────────────────────────────────
        let ctx = unsafe { ffi::mpv_create() };
        if ctx.is_null() {
            return Err("mpv_create returned null".into());
        }

        // Use the render API VO — we provide the OpenGL surface
        unsafe {
            ffi::set_option_str(ctx, "vo", "libmpv")?;
            ffi::set_option_str(ctx, "input-default-bindings", "no")?;
            ffi::set_option_str(ctx, "input-vo-keyboard", "no")?;
            ffi::set_option_str(ctx, "osc", "no")?;
            ffi::set_option_str(ctx, "osd-level", "0")?;
            ffi::set_option_str(ctx, "idle", "yes")?;
            ffi::set_option_str(ctx, "keep-open", "yes")?;
            ffi::set_option_str(ctx, "hwdec", "auto-safe")?;
        }

        unsafe { ffi::check(ffi::mpv_initialize(ctx))? };

        // Observe properties we care about
        unsafe {
            ffi::observe(ctx, UD_PAUSE, "pause", ffi::MPV_FORMAT_FLAG)?;
            ffi::observe(ctx, UD_TIME_POS, "time-pos", ffi::MPV_FORMAT_DOUBLE)?;
            ffi::observe(ctx, UD_DURATION, "duration", ffi::MPV_FORMAT_DOUBLE)?;
            ffi::observe(ctx, UD_VOLUME, "volume", ffi::MPV_FORMAT_DOUBLE)?;
            ffi::observe(ctx, UD_MUTE, "mute", ffi::MPV_FORMAT_FLAG)?;
            ffi::observe(ctx, UD_IDLE, "idle-active", ffi::MPV_FORMAT_FLAG)?;
            ffi::observe(ctx, UD_MEDIA_TITLE, "media-title", ffi::MPV_FORMAT_STRING)?;
            ffi::observe(ctx, UD_PAUSED_FOR_CACHE, "paused-for-cache", ffi::MPV_FORMAT_FLAG)?;
        }

        Ok(Self {
            ctx: SendPtr { ptr: ctx },
            _view: SendRetained(gl_data.view),
            _gl_ctx: SendRetained(gl_data.gl_context),
            running: Arc::new(AtomicBool::new(false)),
            render_signal: Arc::new(RenderSignal {
                needs_render: Mutex::new(false),
                condvar: Condvar::new(),
            }),
            event_thread: None,
            render_thread: None,
            app,
        })
    }

    /// Start the event loop and render loop threads.
    pub fn start(&mut self) {
        if self.running.swap(true, Ordering::SeqCst) {
            return; // already running
        }
        self.spawn_event_loop(self.app.clone());
        self.spawn_render_loop(self.app.clone());
    }

    /// Spin up the background event‐loop that forwards mpv events → frontend.
    fn spawn_event_loop(&mut self, app: AppHandle) {
        let ctx = self.ctx;
        let running = self.running.clone();

        let handle = std::thread::Builder::new()
            .name("mpv-event-loop".into())
            .spawn(move || {
                let ctx_ptr = ctx.get();
                let mut state = MpvState::default();

                while running.load(Ordering::SeqCst) {
                    let ev = unsafe { &*ffi::mpv_wait_event(ctx_ptr, 0.25) };

                    match ev.event_id {
                        ffi::MPV_EVENT_NONE => continue,
                        ffi::MPV_EVENT_SHUTDOWN => {
                            running.store(false, Ordering::SeqCst);
                            let _ = app.emit("mpv-shutdown", ());
                            break;
                        }
                        ffi::MPV_EVENT_FILE_LOADED => {
                            let _ = app.emit("mpv-file-loaded", ());
                        }
                        ffi::MPV_EVENT_END_FILE => {
                            let ef =
                                unsafe { &*(ev.data as *const ffi::mpv_event_end_file) };
                            let reason = match ef.reason {
                                ffi::MPV_END_FILE_REASON_EOF => "eof",
                                ffi::MPV_END_FILE_REASON_STOP => "stop",
                                ffi::MPV_END_FILE_REASON_ERROR => "error",
                                _ => "unknown",
                            };
                            let _ = app.emit("mpv-end-file", reason);
                            state.idle = true;
                            state.playing = false;
                            let _ = app.emit("mpv-state", state.clone());
                        }
                        ffi::MPV_EVENT_PROPERTY_CHANGE => {
                            let prop = unsafe {
                                &*(ev.data as *const ffi::mpv_event_property)
                            };
                            if prop.data.is_null() {
                                continue;
                            }
                            match ev.reply_userdata {
                                UD_PAUSE => {
                                    let v = unsafe { *(prop.data as *const i32) };
                                    state.playing = v == 0;
                                }
                                UD_TIME_POS => {
                                    let v = unsafe { *(prop.data as *const f64) };
                                    if v.is_finite() {
                                        state.time_pos = v;
                                    }
                                }
                                UD_DURATION => {
                                    let v = unsafe { *(prop.data as *const f64) };
                                    if v.is_finite() && v > 0.0 {
                                        state.duration = v;
                                    }
                                }
                                UD_VOLUME => {
                                    let v = unsafe { *(prop.data as *const f64) };
                                    if v.is_finite() {
                                        state.volume = v;
                                    }
                                }
                                UD_MUTE => {
                                    let v = unsafe { *(prop.data as *const i32) };
                                    state.muted = v != 0;
                                }
                                UD_IDLE => {
                                    let v = unsafe { *(prop.data as *const i32) };
                                    state.idle = v != 0;
                                }
                                UD_PAUSED_FOR_CACHE => {
                                    let v = unsafe { *(prop.data as *const i32) };
                                    state.paused_for_cache = v != 0;
                                }
                                UD_MEDIA_TITLE => {
                                    if prop.format == ffi::MPV_FORMAT_STRING {
                                        let cstr_ptr =
                                            unsafe { *(prop.data as *const *const i8) };
                                        if !cstr_ptr.is_null() {
                                            let s = unsafe {
                                                std::ffi::CStr::from_ptr(cstr_ptr)
                                                    .to_string_lossy()
                                                    .into_owned()
                                            };
                                            state.title = s;
                                        }
                                    }
                                }
                                _ => {}
                            }
                            let _ = app.emit("mpv-state", state.clone());
                        }
                        _ => {}
                    }
                }
            })
            .expect("failed to spawn mpv event loop thread");

        self.event_thread = Some(handle);
    }

    /// Spin up the OpenGL render thread.
    ///
    /// This thread owns the GL context and mpv render context. It waits for
    /// mpv's update callback, then renders the current frame into FBO 0
    /// (the NSView's drawable) and swaps buffers.
    fn spawn_render_loop(&mut self, app: AppHandle) {
        let mpv_ctx = self.ctx;
        let running = self.running.clone();
        let signal = self.render_signal.clone();

        // Pass raw pointers to the render thread. The Retained objects in
        // MpvPlayer keep them alive, and Drop joins this thread first.
        let gl_ctx_addr = Retained::as_ptr(&self._gl_ctx.0) as usize;
        let view_addr = Retained::as_ptr(&self._view.0) as usize;

        let handle = std::thread::Builder::new()
            .name("mpv-render".into())
            .spawn(move || {
                let gl_ctx: &NSOpenGLContext =
                    unsafe { &*(gl_ctx_addr as *const NSOpenGLContext) };
                let view: &NSView = unsafe { &*(view_addr as *const NSView) };

                // Make the GL context current on this thread
                gl_ctx.makeCurrentContext();

                // ── Create mpv render context with OpenGL ──────────
                let mut gl_init = ffi::mpv_opengl_init_params {
                    get_proc_address: Some(ffi::gl_get_proc_address),
                    get_proc_address_ctx: std::ptr::null_mut(),
                };
                let api_type = c"opengl";
                let mut params = [
                    ffi::mpv_render_param {
                        type_: ffi::MPV_RENDER_PARAM_API_TYPE,
                        data: api_type.as_ptr() as *mut c_void,
                    },
                    ffi::mpv_render_param {
                        type_: ffi::MPV_RENDER_PARAM_OPENGL_INIT_PARAMS,
                        data: &mut gl_init as *mut _ as *mut c_void,
                    },
                    ffi::mpv_render_param {
                        type_: ffi::MPV_RENDER_PARAM_INVALID,
                        data: std::ptr::null_mut(),
                    },
                ];

                let mut render_ctx: *mut ffi::mpv_render_context = std::ptr::null_mut();
                let err = unsafe {
                    ffi::mpv_render_context_create(
                        &mut render_ctx,
                        mpv_ctx.get(),
                        params.as_mut_ptr(),
                    )
                };
                if err < 0 {
                    eprintln!("mpv: failed to create render context (err={})", err);
                    NSOpenGLContext::clearCurrentContext();
                    return;
                }

                // Set up the update callback — signals this thread when a new frame is ready
                let signal_ptr = Arc::into_raw(signal.clone()) as *mut c_void;
                unsafe {
                    ffi::mpv_render_context_set_update_callback(
                        render_ctx,
                        Some(on_render_update),
                        signal_ptr,
                    );
                }

                // ── Render loop ────────────────────────────────────
                let mut first_frame_emitted = false;
                while running.load(Ordering::SeqCst) {
                    // Wait for the update callback to signal a new frame
                    {
                        let mut needs = signal.needs_render.lock().unwrap();
                        while !*needs && running.load(Ordering::SeqCst) {
                            let result = signal
                                .condvar
                                .wait_timeout(needs, Duration::from_millis(50))
                                .unwrap();
                            needs = result.0;
                        }
                        if !running.load(Ordering::SeqCst) {
                            break;
                        }
                        *needs = false;
                    }

                    // Check if mpv actually wants a frame rendered
                    let flags = unsafe { ffi::mpv_render_context_update(render_ctx) };
                    if flags & ffi::MPV_RENDER_UPDATE_FRAME == 0 {
                        continue;
                    }

                    // Get the view's bounds for the FBO dimensions.
                    // On Retina displays, the backing size is 2x the point size.
                    let bounds = view.bounds();
                    let scale = view
                        .window()
                        .map(|w| w.backingScaleFactor())
                        .unwrap_or(2.0);
                    let w = (bounds.size.width * scale) as i32;
                    let h = (bounds.size.height * scale) as i32;
                    if w <= 0 || h <= 0 {
                        continue;
                    }

                    // Render into FBO 0 (the NSOpenGLContext's view drawable)
                    let mut fbo_data = ffi::mpv_opengl_fbo {
                        fbo: 0,
                        w,
                        h,
                        internal_format: 0,
                    };
                    let mut flip: i32 = 1;
                    let mut render_params = [
                        ffi::mpv_render_param {
                            type_: ffi::MPV_RENDER_PARAM_OPENGL_FBO,
                            data: &mut fbo_data as *mut _ as *mut c_void,
                        },
                        ffi::mpv_render_param {
                            type_: ffi::MPV_RENDER_PARAM_FLIP_Y,
                            data: &mut flip as *mut _ as *mut c_void,
                        },
                        ffi::mpv_render_param {
                            type_: ffi::MPV_RENDER_PARAM_INVALID,
                            data: std::ptr::null_mut(),
                        },
                    ];
                    unsafe {
                        ffi::mpv_render_context_render(
                            render_ctx,
                            render_params.as_mut_ptr(),
                        );
                    }
                    gl_ctx.flushBuffer();
                    unsafe { ffi::mpv_render_context_report_swap(render_ctx) };

                    if !first_frame_emitted {
                        first_frame_emitted = true;
                        let _ = app.emit("mpv-first-frame", ());
                    }
                }

                // ── Cleanup ────────────────────────────────────────
                // Drop the Arc we leaked for the callback
                unsafe { Arc::from_raw(signal_ptr as *const RenderSignal) };
                unsafe { ffi::mpv_render_context_free(render_ctx) };
                NSOpenGLContext::clearCurrentContext();
            })
            .expect("failed to spawn mpv render thread");

        self.render_thread = Some(handle);
    }

    // ── Playback commands ──────────────────────────────────

    pub fn load_file(&self, url: &str) -> Result<(), String> {
        unsafe { ffi::command(self.ctx.ptr, &["loadfile", url, "replace"]) }
    }

    pub fn stop(&self) -> Result<(), String> {
        unsafe { ffi::command(self.ctx.ptr, &["stop"]) }
    }

    pub fn toggle_pause(&self) -> Result<(), String> {
        let paused = unsafe { ffi::get_property_flag(self.ctx.ptr, "pause")? };
        unsafe { ffi::set_property_flag(self.ctx.ptr, "pause", !paused) }
    }

    pub fn set_pause(&self, paused: bool) -> Result<(), String> {
        unsafe { ffi::set_property_flag(self.ctx.ptr, "pause", paused) }
    }

    pub fn seek(&self, seconds: f64, mode: &str) -> Result<(), String> {
        let s = format!("{}", seconds);
        unsafe { ffi::command(self.ctx.ptr, &["seek", &s, mode]) }
    }

    pub fn seek_absolute(&self, seconds: f64) -> Result<(), String> {
        self.seek(seconds, "absolute")
    }

    pub fn set_volume(&self, volume: f64) -> Result<(), String> {
        unsafe {
            ffi::set_property_double(self.ctx.ptr, "volume", volume.clamp(0.0, 150.0))
        }
    }

    pub fn set_mute(&self, muted: bool) -> Result<(), String> {
        unsafe { ffi::set_property_flag(self.ctx.ptr, "mute", muted) }
    }

    pub fn get_state(&self) -> MpvState {
        MpvState {
            playing: unsafe {
                ffi::get_property_flag(self.ctx.ptr, "pause")
                    .map(|p| !p)
                    .unwrap_or(false)
            },
            time_pos: unsafe {
                ffi::get_property_double(self.ctx.ptr, "time-pos").unwrap_or(0.0)
            },
            duration: unsafe {
                ffi::get_property_double(self.ctx.ptr, "duration").unwrap_or(0.0)
            },
            volume: unsafe {
                ffi::get_property_double(self.ctx.ptr, "volume").unwrap_or(100.0)
            },
            muted: unsafe {
                ffi::get_property_flag(self.ctx.ptr, "mute").unwrap_or(false)
            },
            idle: unsafe {
                ffi::get_property_flag(self.ctx.ptr, "idle-active").unwrap_or(true)
            },
            paused_for_cache: unsafe {
                ffi::get_property_flag(self.ctx.ptr, "paused-for-cache").unwrap_or(false)
            },
            title: unsafe {
                ffi::get_property_string(self.ctx.ptr, "media-title").unwrap_or_default()
            },
        }
    }

    /// Get available tracks (audio/sub/video) as a JSON string.
    pub fn get_tracks(&self) -> Result<String, String> {
        let count =
            unsafe { ffi::get_property_double(self.ctx.ptr, "track-list/count")? } as usize;
        let mut tracks = Vec::new();

        for i in 0..count {
            let t_type = unsafe {
                ffi::get_property_string(self.ctx.ptr, &format!("track-list/{}/type", i))
                    .unwrap_or_default()
            };
            let t_id = unsafe {
                ffi::get_property_double(self.ctx.ptr, &format!("track-list/{}/id", i))
                    .unwrap_or(0.0)
            } as i64;
            let t_title = unsafe {
                ffi::get_property_string(self.ctx.ptr, &format!("track-list/{}/title", i))
                    .unwrap_or_default()
            };
            let t_lang = unsafe {
                ffi::get_property_string(self.ctx.ptr, &format!("track-list/{}/lang", i))
                    .unwrap_or_default()
            };
            let t_selected = unsafe {
                ffi::get_property_flag(
                    self.ctx.ptr,
                    &format!("track-list/{}/selected", i),
                )
                .unwrap_or(false)
            };

            tracks.push(serde_json::json!({
                "type": t_type,
                "id": t_id,
                "title": t_title,
                "lang": t_lang,
                "selected": t_selected,
            }));
        }

        Ok(serde_json::to_string(&tracks).unwrap_or_else(|_| "[]".into()))
    }

    pub fn set_track(&self, track_type: &str, id: i64) -> Result<(), String> {
        let prop = match track_type {
            "audio" | "aid" => "aid",
            "sub" | "sid" => "sid",
            "video" | "vid" => "vid",
            _ => return Err(format!("unknown track type: {}", track_type)),
        };
        unsafe { ffi::set_property_str(self.ctx.ptr, prop, &id.to_string()) }
    }
}

impl Drop for MpvPlayer {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        // Wake the render thread so it can exit
        *self.render_signal.needs_render.lock().unwrap() = true;
        self.render_signal.condvar.notify_one();
        // Join render thread first (it frees the render context)
        if let Some(h) = self.render_thread.take() {
            let _ = h.join();
        }
        // Join event thread
        if let Some(h) = self.event_thread.take() {
            let _ = h.join();
        }
        // Now safe to destroy mpv
        if !self.ctx.ptr.is_null() {
            unsafe { ffi::mpv_terminate_destroy(self.ctx.ptr) };
        }
        // Remove the NSView from the window (AppKit requires main thread).
        // The window's contentView retains the subview, so the pointer stays
        // valid until removeFromSuperview is called — safe to fire-and-forget.
        let view_addr = Retained::as_ptr(&self._view.0) as usize;
        if objc2::MainThreadMarker::new().is_some() {
            // Already on the main thread — call directly (avoids deadlock).
            let view: &NSView = unsafe { &*(view_addr as *const NSView) };
            super::embed_macos::remove_mpv_view(view);
        } else {
            // Fire-and-forget: dispatch to main thread without blocking.
            let _ = self.app.run_on_main_thread(move || {
                let view: &NSView = unsafe { &*(view_addr as *const NSView) };
                super::embed_macos::remove_mpv_view(view);
            });
        }
    }
}

/// Called by mpv from an internal thread when a new frame is available.
unsafe extern "C" fn on_render_update(cb_ctx: *mut c_void) {
    let signal = unsafe { &*(cb_ctx as *const RenderSignal) };
    if let Ok(mut needs) = signal.needs_render.lock() {
        *needs = true;
        signal.condvar.notify_one();
    }
}
