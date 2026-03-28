use std::sync::mpsc;
use gtk::prelude::*;
use adw::{NavigationView, StatusPage};
use gtk::{Box, FlowBox, GestureClick, Label, Picture, ScrolledWindow, Stack};

use crate::stremio::{client, models::MetaPreview};

pub fn build_catalog_stack(nav_view: &NavigationView) -> Stack {
    let (sender, receiver) = mpsc::channel::<Vec<MetaPreview>>();

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

    gtk::glib::spawn_future_local(async move {
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
                    let details_page = crate::ui::details::build_details_page(&movie_clone, &nav_view_for_click);

                    nav_view_for_click.push(&details_page);
                });
                movie_card.add_controller(click_gesture);

                movie_card.append(&picture_clamp);
                movie_card.append(&title);
                flow_box.append(&movie_card);

                if !movie.poster.is_empty() {
                    let pic_clone = picture.clone();
                    let poster_url = movie.poster.clone();

                    gtk::glib::spawn_future_local(async move {
                        let (tx, rx) = tokio::sync::oneshot::channel();

                        crate::RUNTIME.get().unwrap().spawn(async move {
                            let result = crate::stremio::cache::fetch_or_cache_image(poster_url).await;
                            let _ = tx.send(result);
                        });

                        let mut loaded_successfully = false;

                        if let Ok(Some(bytes)) = rx.await {
                            let glib_bytes = gtk::glib::Bytes::from(&bytes);
                            if let Ok(texture) = gtk::gdk::Texture::from_bytes(&glib_bytes) {
                                pic_clone.set_paintable(Some(&texture));
                                loaded_successfully = true;
                            }
                        }

                        if !loaded_successfully {
                            crate::ui::utils::apply_fallback_icon(&pic_clone);
                        }
                    });
                } else {
                    crate::ui::utils::apply_fallback_icon(&picture);
                }
            }
            break;
        }
    });

    main_stack
}
