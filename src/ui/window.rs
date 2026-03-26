use std::sync::mpsc;
use std::thread;

use adw::{ActionRow, ApplicationWindow, EntryRow, PreferencesGroup, PreferencesPage, PreferencesWindow, StatusPage, prelude::*};
use adw::{Application, HeaderBar, NavigationView, NavigationPage, ToolbarView};
use gtk::{Box, Button, CssProvider, FlowBox, GestureClick, Label, Overlay, Picture, ScrolledWindow, Stack, StyleContext, gio, glib};

use crate::stremio::client;
use crate::stremio::models::MetaPreview;

fn load_css() {
    let css_data = "
        /* The main container for each movie */
        .movie-card {
            background-color: alpha(currentColor, 0.03); /* Subtle dark background */
            border-radius: 12px;
            padding: 8px;
            transition: all 0.2s cubic-bezier(0.25, 0.46, 0.45, 0.94); /* Smooth animation */
        }

        /* Make the card pop up and cast a shadow when hovered! */
        .movie-card:hover {
            transform: scale(1.05); /* Enlarge slightly */
            background-color: alpha(currentColor, 0.08); /* Brighten background */
        }

        /* Round the corners of the movie poster */
        .card-poster {
            border-radius: 8px;
            box-shadow: inset 0 0 0 1px alpha(white, 0.1);
            /* NEW: A subtle placeholder background before the image loads */
            background-color: alpha(white, 0.05);
        }

        /* Style the title text */
        .movie-title {
            font-weight: bold;
            font-size: 14px;
            margin-top: 4px;
        }

        /* Details page styles */
        .trailer-button {
            background-color: alpha(white, 0.1);
            color: white;
            font-weight: bold;
            border-radius: 24px;
            padding: 12px 32px;
            border: 1px solid alpha(white, 0.2);
            transition: all 0.2s ease;
        }
        .trailer-button:hover {
            background-color: alpha(white, 0.2);
        }

        .details-gradient {
            background-image: linear-gradient(to right, rgba(0,0,0, 0.9) 0%, rgba(0,0,0, 0.6) 40%, transparent 100%);
        }
        .details-meta {
            color: #aaaaaa;
            font-size: 14px;
            font-weight: bold;
        }
        .details-desc {
            font-size: 16px;
            line-height: 1.4;
            color: #eeeeee;
        }
        .details-cast {
            font-size: 14px;
            color: #888888;
        }

        .custom-back-btn {
            background-color: alpha(black, 0.5);
            color: white;
            border-radius: 50px;
            padding: 12px 20px;
            font-weight: bold;
            font-size: 14px;
            border: 1px solid alpha(white, 0.2);
            transition: all 0.2s ease;
        }
        .custom-back-btn:hover {
            background-color: alpha(white, 0.2);
            transform: scale(1.05);
        }
    ";

    let provider = CssProvider::new();
    provider.load_from_data(css_data);

    gtk::style_context_add_provider_for_display(
        &gtk::gdk::Display::default().expect("Failed to get default display"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn build_details_page(movie: &MetaPreview, nav_view: &NavigationView) -> NavigationPage {
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

        glib::spawn_future_local(async move {
            let (tx, rx) = tokio::sync::oneshot::channel();

            crate::RUNTIME.get().unwrap().spawn(async move {
                let bytes = crate::stremio::cache::fetch_or_cache_image(bg_url).await;
                let _ = tx.send(bytes);
            });

            if let Ok(Some(bytes)) = rx.await {
                let glib_bytes = glib::Bytes::from(&bytes);
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

    let content_box = Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(16)
        .margin_top(48)
        .margin_bottom(48)
        .margin_start(48)
        .halign(gtk::Align::Start)
        .valign(gtk::Align::Start)
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

        glib::spawn_future_local(async move {
            let (tx, rx) = tokio::sync::oneshot::channel();

            crate::RUNTIME.get().unwrap().spawn(async move {
                let bytes = crate::stremio::cache::fetch_or_cache_image(logo_url).await;
                let _ = tx.send(bytes);
            });

            let mut loaded_successfully = false;

            if let Ok(Some(bytes)) = rx.await {
                let glib_bytes = glib::Bytes::from(&bytes);
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

    content_box.append(&back_button);

    content_box.append(&logo_clamp);
    content_box.append(&title_label);

    content_box.append(&meta_label);
    content_box.append(&desc_label);
    content_box.append(&cast_label);
    content_box.append(&trailer_button);

    overlay.add_overlay(&content_box);

    NavigationPage::builder()
        .title(&movie.name)
        .child(&overlay)
        .build()
}

fn show_addons_window(parent: &ApplicationWindow) {
    let pref_window = PreferencesWindow::builder()
        .transient_for(parent)
        .modal(true)
        .default_width(600)
        .default_height(400)
        .title("Manage Add-ons")
        .build();

    let page = PreferencesPage::builder()
        .build();

    let add_group = PreferencesGroup::builder()
        .title("Add New Add-on")
        .description("Enter the manifest URL of a Stremio add-on to install it.")
        .build();

    let url_entry = EntryRow::builder()
        .title("Manifest URL")
        .build();

    let add_button = Button::builder()
        .label("Install")
        .valign(gtk::Align::Start)
        .css_classes(["suggested-action", "pill"])
        .build();

    url_entry.add_suffix(&add_button);
    add_group.add(&url_entry);
    page.add(&add_group);

    let installed_group = PreferencesGroup::builder()
        .title("Installed Add-ons")
        .build();

    let config = crate::stremio::store::load_addons();

    for addon in config.addons {
        let addon_row = ActionRow::builder()
            .title(&addon.manifest.name)
            .subtitle(&format!("Version: {}", addon.manifest.version))
            .build();

        if !addon.manifest.logo.is_empty() {
            let logo_picture = Picture::builder()
                .content_fit(gtk::ContentFit::Contain)
                .can_shrink(true)
                .width_request(48)
                .height_request(48)
                .margin_end(12)
                .build();

            let logo_clamp = adw::Clamp::builder()
                .maximum_size(48) // The logo will absolutely NEVER exceed 350px wide
                .halign(gtk::Align::Center) // Left-align the clamp container
                .child(&logo_picture)
                .build();

            let logo_url = addon.manifest.logo.clone();
            let logo_clone = logo_picture.clone();

            glib::spawn_future_local(async move {
                let file = gio::File::for_uri(&logo_url);

                if let Ok((bytes, _)) = file.load_bytes_future().await {
                    if let Ok(texture) = gtk::gdk::Texture::from_bytes(&bytes) {
                        logo_clone.set_paintable(Some(&texture));
                    }
                }
            });

            addon_row.add_prefix(&logo_clamp);
        }

        let remove_btn = Button::builder()
            .label("Remove")
            .valign(gtk::Align::Center)
            .css_classes(["destructive-action", "pill"])
            .build();

        addon_row.add_suffix(&remove_btn);
        installed_group.add(&addon_row);
    }

    page.add(&installed_group);

    pref_window.add(&page);

    let window_clone = pref_window.clone();

    add_button.connect_clicked(move |_| {
        let new_url = url_entry.text().to_string();

        if !new_url.is_empty() {
            println!("Fetching manifest from URL: {}", new_url);

            let url_clone = new_url.clone();
            let window_for_async = window_clone.clone();

            glib::spawn_future_local(async move {
                let (sender, receiver) = tokio::sync::oneshot::channel();
                let url_for_tokio = url_clone.clone();

                crate::RUNTIME.get().unwrap().spawn(async move {
                    let result = crate::stremio::client::fetch_manifest(&url_for_tokio).await;
                    let _ = sender.send(result);
                });

                if let Ok(Ok(manifest)) = receiver.await {
                    let mut current_config = crate::stremio::store::load_addons();

                    if !current_config.addons.iter().any(|a| a.transport_url == url_clone) {
                        let installed_addon = crate::stremio::store::InstalledAddon {
                            transport_url: url_clone.clone(),
                            manifest,
                        };

                        current_config.addons.push(installed_addon);
                        crate::stremio::store::save_addons(&current_config);

                        println!("✅ Successfully installed and saved: {}", url_clone);

                        window_for_async.close();
                    } else {
                        println!("⚠️ Add-on already installed: {}", url_clone);
                    }
                } else {
                    println!("❌ Failed to fetch manifest from URL: {}", url_clone);
                }
            });
        }
    });

    pref_window.present();
}

fn apply_fallback_icon(picture: &Picture) {
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

pub fn build_ui(app: &Application) {
    let style_manager = adw::StyleManager::default();
    style_manager.set_color_scheme(adw::ColorScheme::ForceDark);
    load_css();

    let nav_view = NavigationView::builder().build();

    let catalog_toolbar = ToolbarView::builder().build();

    let loading_page = StatusPage::builder()
        .icon_name("network-workgroup-symbolic")
        .title("Loading Catalog")
        .description("Fetching movies...")
        .build();

    let spinner = gtk::Spinner::builder()
        .spinning(true)
        .width_request(32)
        .height_request(32)
        .margin_top(16)
        .build();

    loading_page.set_child(Some(&spinner));

    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .build();

    let flow_box = FlowBox::builder()
        .valign(gtk::Align::Start)
        .halign(gtk::Align::Center)
        .max_children_per_line(8)
        .min_children_per_line(2)
        .selection_mode(gtk::SelectionMode::None)
        .margin_top(24)
        .margin_bottom(24)
        .margin_start(24)
        .margin_end(24)
        .column_spacing(16)
        .row_spacing(24)
        .build();

    scrolled_window.set_child(Some(&flow_box));

    let main_stack = Stack::builder()
        .transition_type(gtk::StackTransitionType::Crossfade)
        .transition_duration(300)
        .build();

    main_stack.add_named(&loading_page, Some("loading"));
    main_stack.add_named(&scrolled_window, Some("catalog"));

    main_stack.set_visible_child_name("loading");

    catalog_toolbar.set_content(Some(&main_stack));

    let catalog_page = NavigationPage::builder()
        .title("Movies")
        .child(&catalog_toolbar)
        .build();
    nav_view.add(&catalog_page);

    let content_box = Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    let app_header = HeaderBar::builder().build();

    let addons_btn = Button::builder()
        .icon_name("preferences-system-symbolic")
        .tooltip_text("manage addons")
        .build();

    app_header.pack_end(&addons_btn);

    content_box.append(&app_header);
    content_box.append(&nav_view);

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Streamix")
        .default_width(800)
        .default_height(600)
        .content(&content_box)
        .build();

    let window_clone = window.clone();
    addons_btn.connect_clicked(move |_| {
        show_addons_window(&window_clone);
    });

    let (sender, receiver) = mpsc::channel::<Vec<MetaPreview>>();

    crate::RUNTIME.get().unwrap().spawn(async move {
        let config = crate::stremio::store::init_addons().await;

        for addon in config.addons {
            if addon.manifest.supports_resource("catalog", "movie") {
                println!("Addon supports movie catalogs. Fetching top movies...");

                if let Some(catalog) = addon.manifest.catalogs.iter().find(|c| c.item_type == "movie") {
                    if let Ok(catalog_response) = client::fetch_catalog(
                        &addon.transport_url,
                        &catalog.item_type,
                        &catalog.id
                    ).await {
                        if let Err(e) = sender.send(catalog_response.metas) {
                            eprintln!("Error sending catalog data: {}", e);
                        }
                    } else {
                        eprintln!("Error fetching catalog from addon: {}", addon.manifest.name);
                    }
                }
            }
        }
    });

    let nav_view_clone = nav_view.clone();
    let stack_clone = main_stack.clone();

    glib::spawn_future_local(async move {
        while let Some(movies) = receiver.recv().ok() {
            stack_clone.set_visible_child_name("catalog");

            for movie in movies {
                let movie_card = Box::builder()
                    .orientation(gtk::Orientation::Vertical)
                    .spacing(8)
                    .width_request(160)
                    .valign(gtk::Align::Start)
                    .halign(gtk::Align::Center)
                    .css_classes(["movie-card"])
                    .build();

                let picture = Picture::builder()
                    .width_request(260)
                    .height_request(390)
                    .content_fit(gtk::ContentFit::Cover)
                    .valign(gtk::Align::Fill)
                    .halign(gtk::Align::Fill)
                    .css_classes(["card-poster"])
                    .build();

                let picture_clamp = adw::Clamp::builder()
                    .maximum_size(260) // The poster will absolutely NEVER exceed 160px wide
                    .child(&picture)
                    .build();

                let title = Label::builder()
                    .label(&movie.name)
                    .wrap(true)
                    .max_width_chars(15)
                    .ellipsize(gtk::pango::EllipsizeMode::End)
                    .lines(2)
                    .css_classes(["movie-title"])
                    .build();

                let click_gesture = GestureClick::new();
                let nav_view_for_click = nav_view_clone.clone();
                let movie_clone = movie.clone();

                click_gesture.connect_released(move |_, _, _, _| {
                    let details_page = build_details_page(&movie_clone, &nav_view_for_click);

                    nav_view_for_click.push(&details_page);
                });
                movie_card.add_controller(click_gesture);

                movie_card.append(&picture_clamp);
                movie_card.append(&title);
                flow_box.append(&movie_card);

                let pic_clone = picture.clone();

                if !movie.poster.is_empty() {
                    let poster_url = movie.poster.clone();

                    glib::spawn_future_local(async move {
                        let (tx, rx) = tokio::sync::oneshot::channel();

                        crate::RUNTIME.get().unwrap().spawn(async move {
                            let result = crate::stremio::cache::fetch_or_cache_image(poster_url).await;
                            let _ = tx.send(result);
                        });

                        let mut loaded_successfully = false;

                        if let Ok(Some(bytes)) = rx.await {
                            let glib_bytes = glib::Bytes::from(&bytes);
                            if let Ok(texture) = gtk::gdk::Texture::from_bytes(&glib_bytes) {
                                pic_clone.set_paintable(Some(&texture));
                                loaded_successfully = true;
                            }
                        }

                        if !loaded_successfully {
                            apply_fallback_icon(&pic_clone);
                        }
                    });
                } else {
                    apply_fallback_icon(&pic_clone);
                }
            }
            break;
        }
    });

    window.present();
}
