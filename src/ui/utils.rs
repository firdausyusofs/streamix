use gtk::prelude::*;

pub fn apply_fallback_icon(picture: &gtk::Picture) {
    if let Some(display) = gtk::gdk::Display::default() {
        let icon_theme = gtk::IconTheme::for_display(&display);

        let paintable = icon_theme.lookup_icon(
            "video-x-generic-symbolic",
            &[] as &[&str], // No fallbacks needed
            64,             // Size
            1,              // Scale
            gtk::TextDirection::Ltr,
            gtk::IconLookupFlags::empty(),
        );

        picture.set_paintable(Some(&paintable));
        picture.set_opacity(0.3);

        picture.set_content_fit(gtk::ContentFit::ScaleDown);
    }
}
