use gtk::{ListBox, Stack, prelude::*};
use adw::{ActionRow, NavigationPage, NavigationView, StatusPage};
use gtk::{Overlay, Picture, Box, Button, Label};

use crate::stremio::models::{MetaPreview, StreamResponse};

pub fn build_details_page(movie: &MetaPreview, nav_view: &NavigationView) -> NavigationPage {
    let overlay = Overlay::builder().build();

    let bg_picture = Picture::builder()
        .content_fit(gtk::ContentFit::Cover)
        .can_shrink(true)
        .hexpand(true)
        .vexpand(true)
        .build();

    if !movie.background.is_empty() {
        let bg_picture_clone = bg_picture.clone();
        let bg_url = movie.background.clone();

        gtk::glib::spawn_future_local(async move {
            let (tx, rx) = tokio::sync::oneshot::channel();

            crate::RUNTIME.get().unwrap().spawn(async move {
                let bytes = crate::stremio::cache::fetch_or_cache_image(bg_url).await;
                let _ = tx.send(bytes);
            });

            if let Ok(Some(bytes)) = rx.await {
                let glib_bytes = gtk::glib::Bytes::from(&bytes);
                if let Ok(texture) = gtk::gdk::Texture::from_bytes(&glib_bytes) {
                    bg_picture_clone.set_paintable(Some(&texture));
                }
            }
        });
    }
    overlay.set_child(Some(&bg_picture));

    let gradient_box = Box::builder()
        .hexpand(true)
        .vexpand(true)
        .css_classes(["details-gradient"])
        .build();
    overlay.add_overlay(&gradient_box);

    let back_button = Button::builder()
        .label("← Back") // You can use a specific icon here later!
        .halign(gtk::Align::Start)
        .valign(gtk::Align::Start)
        .margin_bottom(48)
        .css_classes(["custom-back-btn"])
        .build();

    let nav_clone = nav_view.clone();
    back_button.connect_clicked(move |_| {
        nav_clone.pop();
    });

    let left_content_box = Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(16)
        .margin_top(48)
        .margin_bottom(48)
        .margin_start(48)
        .halign(gtk::Align::Start)
        .valign(gtk::Align::Start)
        .hexpand(true)
        .build();

    let logo_picture = Picture::builder()
        .content_fit(gtk::ContentFit::Contain)
        .can_shrink(true)
        .height_request(120)
        .build();

    let logo_clamp = adw::Clamp::builder()
        .maximum_size(350) // The logo will absolutely NEVER exceed 350px wide
        .halign(gtk::Align::Start) // Left-align the clamp container
        .child(&logo_picture)
        .build();

    let title_label = Label::builder().label(&movie.name).css_classes(["title-1"]).halign(gtk::Align::Start).build();
    title_label.set_visible(false);

    if !movie.logo.is_empty() {
        let logo_clone = logo_picture.clone();
        let logo_clamp_clone = logo_clamp.clone();
        let title_label_clone = title_label.clone();
        let logo_url = movie.logo.clone();

        gtk::glib::spawn_future_local(async move {
            let (tx, rx) = tokio::sync::oneshot::channel();

            crate::RUNTIME.get().unwrap().spawn(async move {
                let bytes = crate::stremio::cache::fetch_or_cache_image(logo_url).await;
                let _ = tx.send(bytes);
            });

            let mut loaded_successfully = false;

            if let Ok(Some(bytes)) = rx.await {
                let glib_bytes = gtk::glib::Bytes::from(&bytes);
                if let Ok(texture) = gtk::gdk::Texture::from_bytes(&glib_bytes) {
                    logo_clone.set_paintable(Some(&texture));
                    loaded_successfully = true;
                }
            }

            if !loaded_successfully {
                logo_clamp_clone.set_visible(false);
                title_label_clone.set_visible(true);
            }
        });
    } else {
        logo_clamp.set_visible(false);
        title_label.set_visible(true);
    }

    let genres_str = movie.genres.join(", ");
    let meta_string = format!("{}  •  {}  •  {}", movie.year, movie.runtime, genres_str);
    let meta_label = Label::builder()
        .label(&meta_string)
        .halign(gtk::Align::Start)
        .css_classes(["details-meta"])
        .margin_top(24)
        .build();

    let desc_label = Label::builder()
        .label(&movie.description)
        .wrap(true)
        .max_width_chars(60)
        .halign(gtk::Align::Start)
        .css_classes(["details-desc"])
        .build();

    let cast_str = movie.casts.join(", ");
    let cast_label = Label::builder()
        .label(&format!("Starring: {}", cast_str))
        .wrap(true)
        .max_width_chars(60)
        .halign(gtk::Align::Start)
        .css_classes(["details-cast"])
        .build();

    let trailer_button = Button::builder()
        .label("Play Trailer")
        .css_classes(["trailer-button", "pill"])
        .halign(gtk::Align::Start)
        .margin_top(24)
        .build();

    left_content_box.append(&back_button);

    left_content_box.append(&logo_clamp);
    left_content_box.append(&title_label);

    left_content_box.append(&meta_label);
    left_content_box.append(&desc_label);
    left_content_box.append(&cast_label);
    left_content_box.append(&trailer_button);

    let sidebar_box = Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .width_request(500)
        .css_classes(["sidebar-background"])
        .build();

    let sidebar_stack = Stack::builder()
        .transition_type(gtk::StackTransitionType::Crossfade)
        .transition_duration(300)
        .vexpand(true)
        .build();

    let loading_page = StatusPage::builder()
        .icon_name("network-workgroup-symbolic")
        .title("Finding Streams...")
        .vexpand(true)
        .build();
    let spinner = gtk::Spinner::builder()
        .spinning(true)
        .width_request(32)
        .height_request(32)
        .margin_top(16)
        .build();
    loading_page.set_child(Some(&spinner));

    let empty_page = StatusPage::builder()
        .icon_name("face-sad-symbolic")
        .title("No Streams Found")
        .description("Try installing more addons.")
        .vexpand(true)
        .build();

    let stream_listbox = ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .css_classes(["boxed-list"])
        .build();

    let stream_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(&stream_listbox)
        .build();

    sidebar_stack.add_named(&loading_page, Some("loading"));
    sidebar_stack.add_named(&empty_page, Some("empty"));
    sidebar_stack.add_named(&stream_scroll, Some("list"));
    sidebar_stack.set_visible_child_name("loading");

    sidebar_box.append(&sidebar_stack);

    let main_layout = Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .hexpand(true)
        .vexpand(true)
        .build();

    main_layout.append(&left_content_box);
    main_layout.append(&sidebar_box);

    overlay.add_overlay(&main_layout);

    let (sender, receiver) = std::sync::mpsc::channel::<StreamResponse>();
    let movie_id = movie.id.clone();
    let item_type = movie.item_type.clone();

    crate::RUNTIME.get().unwrap().spawn(async move {
        let config = crate::stremio::store::load_addons();

        for addon in config.addons {
            if addon.manifest.supports_resource("stream", &item_type) {
                if let Ok(response) = crate::stremio::client::fetch_streams(&addon.transport_url, &item_type, &movie_id).await {
                    if !response.streams.is_empty() {
                        println!("Found {} streams for {} from addon {}", response.streams.len(), movie_id, addon.manifest.name);
                        let _ = sender.send(response);
                    } else {
                        eprintln!("No streams found for {} from addon {}", movie_id, addon.manifest.name);
                    }
                }
            }
        }
    });

    let stack_clone = sidebar_stack.clone();
    let stream_listbox_clone = stream_listbox.clone();
    gtk::glib::spawn_future_local(async move {
        let mut total_streams_found = 0;

        while let Some(response) = receiver.recv().ok() {
            total_streams_found += response.streams.len();
            stack_clone.set_visible_child_name("list");

            for stream in response.streams {
                let name = stream.name.unwrap_or_else(|| "Unknown Stream".to_string());
                let title = stream.title.unwrap_or_else(|| "No Title".to_string());

                let row = ActionRow::builder()
                    .title(&name)
                    .subtitle(&title)
                    .title_lines(1)
                    .subtitle_lines(2)
                    .build();

                stream_listbox_clone.append(&row);
            }
        }

        if total_streams_found == 0 {
            stack_clone.set_visible_child_name("empty");
        }
    });

    NavigationPage::builder()
        .title(&movie.name)
        .child(&overlay)
        .build()
}
