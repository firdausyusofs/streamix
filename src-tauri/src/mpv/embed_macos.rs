use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSAutoresizingMaskOptions, NSOpenGLContext, NSOpenGLPFAAccelerated, NSOpenGLPFAAlphaSize,
    NSOpenGLPFAColorSize, NSOpenGLPFADoubleBuffer, NSOpenGLPFAOpenGLProfile,
    NSOpenGLPixelFormat, NSOpenGLPixelFormatAttribute, NSOpenGLProfileVersion3_2Core, NSView,
    NSWindow, NSWindowOrderingMode,
};
use objc2_foundation::NSRect;
use std::ptr::NonNull;

#[allow(deprecated)] // OpenGL is deprecated on macOS but still functional

/// Data returned from GL view creation.
pub struct GlViewData {
    pub view: Retained<NSView>,
    pub gl_context: Retained<NSOpenGLContext>,
}

// SAFETY: We protect access with a Mutex in MpvHandle. The GL context
// is only made current on the render thread; the view is only mutated
// during setup/teardown.
unsafe impl Send for GlViewData {}
unsafe impl Sync for GlViewData {}

/// Create an NSView with an associated NSOpenGLContext, inserted behind the
/// webview so that mpv renders underneath the transparent HTML overlay.
///
/// **Must be called on the main thread** (macOS NSView/NSOpenGLContext requirement).
#[allow(deprecated)]
pub fn create_gl_view(window: &NSWindow) -> Result<GlViewData, String> {
    let mtm = MainThreadMarker::from(window);

    let content_view = window.contentView().expect("window has no contentView");
    let frame: NSRect = content_view.frame();

    // Container view for mpv OpenGL rendering
    let mpv_view = NSView::initWithFrame(mtm.alloc(), frame);
    mpv_view.setAutoresizingMask(
        NSAutoresizingMaskOptions::ViewWidthSizable
            | NSAutoresizingMaskOptions::ViewHeightSizable,
    );
    mpv_view.setWantsLayer(true);

    // Insert behind the webview
    content_view.addSubview_positioned_relativeTo(
        &mpv_view,
        NSWindowOrderingMode::Below,
        None,
    );

    // OpenGL pixel format: double-buffered, accelerated, GL 3.2 Core
    let mut attrs: [NSOpenGLPixelFormatAttribute; 9] = [
        NSOpenGLPFADoubleBuffer,
        NSOpenGLPFAAccelerated,
        NSOpenGLPFAOpenGLProfile,
        NSOpenGLProfileVersion3_2Core,
        NSOpenGLPFAColorSize,
        24,
        NSOpenGLPFAAlphaSize,
        8,
        0, // terminator
    ];

    let attrs_ptr = NonNull::new(attrs.as_mut_ptr())
        .ok_or("null attrs pointer")?;

    let pixel_format = unsafe {
        NSOpenGLPixelFormat::initWithAttributes(mtm.alloc(), attrs_ptr)
    }.ok_or("Failed to create NSOpenGLPixelFormat — OpenGL 3.2 not available")?;

    let gl_context = unsafe {
        NSOpenGLContext::initWithFormat_shareContext(mtm.alloc(), &pixel_format, None)
    }.ok_or("Failed to create NSOpenGLContext")?;

    // Associate GL context with our view so FBO 0 renders to the view's surface
    gl_context.setView(Some(&mpv_view), mtm);
    gl_context.update(mtm);

    Ok(GlViewData {
        view: mpv_view,
        gl_context,
    })
}

/// Remove the mpv view from its superview.
pub fn remove_mpv_view(view: &NSView) {
    view.removeFromSuperview();
}
