#![allow(non_camel_case_types, dead_code)]

use std::ffi::{CStr, CString, c_char, c_double, c_int, c_void};
use std::ptr;

// ── Opaque handle ──────────────────────────────────────────────
pub enum mpv_handle {}

// ── Formats ────────────────────────────────────────────────────
pub const MPV_FORMAT_NONE: c_int = 0;
pub const MPV_FORMAT_STRING: c_int = 1;
pub const MPV_FORMAT_OSD_STRING: c_int = 2;
pub const MPV_FORMAT_FLAG: c_int = 3;
pub const MPV_FORMAT_INT64: c_int = 4;
pub const MPV_FORMAT_DOUBLE: c_int = 5;
pub const MPV_FORMAT_NODE: c_int = 6;

// ── Event IDs ──────────────────────────────────────────────────
pub const MPV_EVENT_NONE: c_int = 0;
pub const MPV_EVENT_SHUTDOWN: c_int = 1;
pub const MPV_EVENT_LOG_MESSAGE: c_int = 2;
pub const MPV_EVENT_GET_PROPERTY_REPLY: c_int = 3;
pub const MPV_EVENT_SET_PROPERTY_REPLY: c_int = 4;
pub const MPV_EVENT_COMMAND_REPLY: c_int = 5;
pub const MPV_EVENT_START_FILE: c_int = 6;
pub const MPV_EVENT_END_FILE: c_int = 7;
pub const MPV_EVENT_FILE_LOADED: c_int = 8;
pub const MPV_EVENT_IDLE: c_int = 11;
pub const MPV_EVENT_SEEK: c_int = 20;
pub const MPV_EVENT_PLAYBACK_RESTART: c_int = 21;
pub const MPV_EVENT_PROPERTY_CHANGE: c_int = 22;

// ── End file reasons ───────────────────────────────────────────
pub const MPV_END_FILE_REASON_EOF: c_int = 0;
pub const MPV_END_FILE_REASON_STOP: c_int = 2;
pub const MPV_END_FILE_REASON_QUIT: c_int = 3;
pub const MPV_END_FILE_REASON_ERROR: c_int = 4;

// ── Structs ────────────────────────────────────────────────────
#[repr(C)]
pub struct mpv_event {
    pub event_id: c_int,
    pub error: c_int,
    pub reply_userdata: u64,
    pub data: *mut c_void,
}

#[repr(C)]
pub struct mpv_event_property {
    pub name: *const c_char,
    pub format: c_int,
    pub data: *mut c_void,
}

#[repr(C)]
pub struct mpv_event_end_file {
    pub reason: c_int,
    pub error: c_int,
    pub playlist_entry_id: i64,
    pub playlist_insert_id: i64,
    pub playlist_insert_num_entries: c_int,
}

// ── Extern C bindings ──────────────────────────────────────────
unsafe extern "C" {
    pub fn mpv_create() -> *mut mpv_handle;
    pub fn mpv_initialize(ctx: *mut mpv_handle) -> c_int;
    pub fn mpv_terminate_destroy(ctx: *mut mpv_handle);

    pub fn mpv_set_option(
        ctx: *mut mpv_handle,
        name: *const c_char,
        format: c_int,
        data: *mut c_void,
    ) -> c_int;
    pub fn mpv_set_option_string(
        ctx: *mut mpv_handle,
        name: *const c_char,
        data: *const c_char,
    ) -> c_int;

    pub fn mpv_command(ctx: *mut mpv_handle, args: *const *const c_char) -> c_int;
    pub fn mpv_command_async(
        ctx: *mut mpv_handle,
        reply_userdata: u64,
        args: *const *const c_char,
    ) -> c_int;

    pub fn mpv_set_property(
        ctx: *mut mpv_handle,
        name: *const c_char,
        format: c_int,
        data: *mut c_void,
    ) -> c_int;
    pub fn mpv_set_property_string(
        ctx: *mut mpv_handle,
        name: *const c_char,
        data: *const c_char,
    ) -> c_int;

    pub fn mpv_get_property(
        ctx: *mut mpv_handle,
        name: *const c_char,
        format: c_int,
        data: *mut c_void,
    ) -> c_int;
    pub fn mpv_get_property_string(
        ctx: *mut mpv_handle,
        name: *const c_char,
    ) -> *mut c_char;

    pub fn mpv_observe_property(
        mpv: *mut mpv_handle,
        reply_userdata: u64,
        name: *const c_char,
        format: c_int,
    ) -> c_int;

    pub fn mpv_wait_event(ctx: *mut mpv_handle, timeout: c_double) -> *mut mpv_event;

    pub fn mpv_free(data: *mut c_void);
    pub fn mpv_error_string(error: c_int) -> *const c_char;

    pub fn mpv_set_wakeup_callback(
        ctx: *mut mpv_handle,
        cb: Option<unsafe extern "C" fn(*mut c_void)>,
        d: *mut c_void,
    );
}

// ── Safe helpers ───────────────────────────────────────────────

/// Convert an mpv error code to a Result.
pub fn check(code: c_int) -> Result<(), String> {
    if code >= 0 {
        Ok(())
    } else {
        let msg = unsafe {
            CStr::from_ptr(mpv_error_string(code))
                .to_string_lossy()
                .into_owned()
        };
        Err(format!("mpv error {}: {}", code, msg))
    }
}

/// Set an mpv option as a string (before initialize).
pub unsafe fn set_option_str(ctx: *mut mpv_handle, name: &str, val: &str) -> Result<(), String> {
    let n = CString::new(name).unwrap();
    let v = CString::new(val).unwrap();
    check(unsafe { mpv_set_option_string(ctx, n.as_ptr(), v.as_ptr()) })
}

/// Set an mpv property as a string.
pub unsafe fn set_property_str(ctx: *mut mpv_handle, name: &str, val: &str) -> Result<(), String> {
    let n = CString::new(name).unwrap();
    let v = CString::new(val).unwrap();
    check(unsafe { mpv_set_property_string(ctx, n.as_ptr(), v.as_ptr()) })
}

/// Set an mpv property as f64.
pub unsafe fn set_property_double(ctx: *mut mpv_handle, name: &str, val: f64) -> Result<(), String> {
    let n = CString::new(name).unwrap();
    let mut v = val;
    check(unsafe {
        mpv_set_property(
            ctx,
            n.as_ptr(),
            MPV_FORMAT_DOUBLE,
            &mut v as *mut f64 as *mut c_void,
        )
    })
}

/// Set an mpv property as bool (flag).
pub unsafe fn set_property_flag(ctx: *mut mpv_handle, name: &str, val: bool) -> Result<(), String> {
    let n = CString::new(name).unwrap();
    let mut v: c_int = if val { 1 } else { 0 };
    check(unsafe {
        mpv_set_property(
            ctx,
            n.as_ptr(),
            MPV_FORMAT_FLAG,
            &mut v as *mut c_int as *mut c_void,
        )
    })
}

/// Get a property as f64.
pub unsafe fn get_property_double(ctx: *mut mpv_handle, name: &str) -> Result<f64, String> {
    let n = CString::new(name).unwrap();
    let mut val: f64 = 0.0;
    check(unsafe {
        mpv_get_property(
            ctx,
            n.as_ptr(),
            MPV_FORMAT_DOUBLE,
            &mut val as *mut f64 as *mut c_void,
        )
    })?;
    Ok(val)
}

/// Get a property as bool (flag).
pub unsafe fn get_property_flag(ctx: *mut mpv_handle, name: &str) -> Result<bool, String> {
    let n = CString::new(name).unwrap();
    let mut val: c_int = 0;
    check(unsafe {
        mpv_get_property(
            ctx,
            n.as_ptr(),
            MPV_FORMAT_FLAG,
            &mut val as *mut c_int as *mut c_void,
        )
    })?;
    Ok(val != 0)
}

/// Get a property as string.
pub unsafe fn get_property_string(ctx: *mut mpv_handle, name: &str) -> Result<String, String> {
    let n = CString::new(name).unwrap();
    let raw = unsafe { mpv_get_property_string(ctx, n.as_ptr()) };
    if raw.is_null() {
        return Err(format!("property '{}' unavailable", name));
    }
    let s = unsafe { CStr::from_ptr(raw).to_string_lossy().into_owned() };
    unsafe { mpv_free(raw as *mut c_void) };
    Ok(s)
}

/// Observe a property for MPV_EVENT_PROPERTY_CHANGE events.
pub unsafe fn observe(ctx: *mut mpv_handle, userdata: u64, name: &str, format: c_int) -> Result<(), String> {
    let n = CString::new(name).unwrap();
    check(unsafe { mpv_observe_property(ctx, userdata, n.as_ptr(), format) })
}

/// Send a command to mpv (NULL-terminated array of strings).
pub unsafe fn command(ctx: *mut mpv_handle, args: &[&str]) -> Result<(), String> {
    let cstrs: Vec<CString> = args.iter().map(|a| CString::new(*a).unwrap()).collect();
    let mut ptrs: Vec<*const c_char> = cstrs.iter().map(|c| c.as_ptr()).collect();
    ptrs.push(ptr::null());
    check(unsafe { mpv_command(ctx, ptrs.as_ptr()) })
}

// ── Render API ─────────────────────────────────────────────────

pub enum mpv_render_context {}

pub const MPV_RENDER_PARAM_INVALID: c_int = 0;
pub const MPV_RENDER_PARAM_API_TYPE: c_int = 1;
pub const MPV_RENDER_PARAM_OPENGL_INIT_PARAMS: c_int = 2;
pub const MPV_RENDER_PARAM_OPENGL_FBO: c_int = 3;
pub const MPV_RENDER_PARAM_FLIP_Y: c_int = 4;

pub const MPV_RENDER_UPDATE_FRAME: u64 = 1;

#[repr(C)]
pub struct mpv_render_param {
    pub type_: c_int,
    pub data: *mut c_void,
}

#[repr(C)]
pub struct mpv_opengl_init_params {
    pub get_proc_address:
        Option<unsafe extern "C" fn(ctx: *mut c_void, name: *const c_char) -> *mut c_void>,
    pub get_proc_address_ctx: *mut c_void,
}

#[repr(C)]
pub struct mpv_opengl_fbo {
    pub fbo: c_int,
    pub w: c_int,
    pub h: c_int,
    pub internal_format: c_int,
}

unsafe extern "C" {
    pub fn mpv_render_context_create(
        res: *mut *mut mpv_render_context,
        mpv: *mut mpv_handle,
        params: *mut mpv_render_param,
    ) -> c_int;
    pub fn mpv_render_context_render(
        ctx: *mut mpv_render_context,
        params: *mut mpv_render_param,
    ) -> c_int;
    pub fn mpv_render_context_set_update_callback(
        ctx: *mut mpv_render_context,
        callback: Option<unsafe extern "C" fn(cb_ctx: *mut c_void)>,
        cb_ctx: *mut c_void,
    );
    pub fn mpv_render_context_update(ctx: *mut mpv_render_context) -> u64;
    pub fn mpv_render_context_report_swap(ctx: *mut mpv_render_context);
    pub fn mpv_render_context_free(ctx: *mut mpv_render_context);
}

// dlsym for OpenGL proc address lookup
unsafe extern "C" {
    pub fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

/// RTLD_DEFAULT on macOS = (void*)-2
pub const RTLD_DEFAULT: *mut c_void = -2isize as *mut c_void;

/// OpenGL get_proc_address callback for mpv render API.
pub unsafe extern "C" fn gl_get_proc_address(
    _ctx: *mut c_void,
    name: *const c_char,
) -> *mut c_void {
    unsafe { dlsym(RTLD_DEFAULT, name) }
}
