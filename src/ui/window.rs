use std::sync::mpsc;
use std::thread;

use adw::prelude::*;
use adw::{Application, HeaderBar, NavigationView, NavigationPage, ToolbarView};
use gtk::{Box, CssProvider, FlowBox, GestureClick, Label, Picture, ScrolledWindow, StyleContext, gio, glib};

use crate::streamio::client;
use crate::streamio::models::MetaPreview;

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
            /* A subtle inner shadow to make the image pop */
            box-shadow: inset 0 0 0 1px alpha(white, 0.1);
        }

        /* Style the title text */
        .movie-title {
            font-weight: bold;
            font-size: 14px;
            margin-top: 4px;
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

fn build_details_page(movie: &MetaPreview) -> NavigationPage {
    let toolbar_view = ToolbarView::builder().build();
    let header_bar = HeaderBar::builder()
        .show_start_title_buttons(false)
        .show_end_title_buttons(false)
        .build();
    toolbar_view.add_top_bar(&header_bar);

    let content_box = Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(24)
        .margin_top(48)
        .margin_bottom(48)
        .halign(gtk::Align::Center)
        .build();

    let picture = Picture::builder()
        .height_request(360) // Bigger poster!
        .width_request(240)
        .content_fit(gtk::ContentFit::Cover)
        .can_shrink(true)
        .css_classes(["details-poster"])
        .build();

    let title = Label::builder()
        .label(&movie.name)
        .css_classes(["title-1"]) // Built-in Adwaita class for large titles
        .build();

    content_box.append(&picture);
    content_box.append(&title);

    toolbar_view.set_content(Some(&content_box));

    NavigationPage::builder()
        .title(&movie.name)
        .child(&toolbar_view)
        .build()
}

pub fn build_ui(app: &Application) {
    let style_manager = adw::StyleManager::default();
    style_manager.set_color_scheme(adw::ColorScheme::ForceDark);
    load_css();

    let nav_view = NavigationView::builder().build();

    let catalog_toolbar = ToolbarView::builder().build();
    let catalog_header = HeaderBar::builder()
        .show_start_title_buttons(false)
        .show_end_title_buttons(false)
        .build();
    catalog_toolbar.add_top_bar(&catalog_header);

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
    catalog_toolbar.set_content(Some(&scrolled_window));

    let catalog_page = NavigationPage::builder()
        .title("Movies")
        .child(&catalog_toolbar)
        .build();
    nav_view.add(&catalog_page);

    let content_box = Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    let app_header = HeaderBar::builder().build();

    content_box.append(&app_header);
    content_box.append(&nav_view);

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Streamix")
        .default_width(800)
        .default_height(600)
        .content(&content_box)
        .build();

    let (sender, receiver) = mpsc::channel::<Vec<MetaPreview>>();

    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let manifest_url = "https://v3-cinemeta.strem.io/manifest.json";

            match client::fetch_catalog(manifest_url, "movie", "top").await {
                Ok(manifest) => {
                    if let Err(e) = sender.send(manifest.metas) {
                        eprintln!("Error sending catalog data: {}", e);
                    }
                }
                Err(e) => eprintln!("Error fetching manifest: {}", e),
            }
        });
    });

    let nav_view_clone = nav_view.clone();

    glib::spawn_future_local(async move {
        while let Some(movies) = receiver.recv().ok() {
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
                    .width_request(160)
                    .height_request(450)
                    .content_fit(gtk::ContentFit::Cover)
                    .valign(gtk::Align::Fill)
                    .halign(gtk::Align::Fill)
                    .css_classes(["card-poster"])
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
                    let details_page = build_details_page(&movie_clone);

                    nav_view_for_click.push(&details_page);
                });
                movie_card.add_controller(click_gesture);

                movie_card.append(&picture);
                movie_card.append(&title);
                flow_box.append(&movie_card);

                let pic_clone = picture.clone();

                glib::spawn_future_local(async move {
                    let file = gio::File::for_uri(&movie.poster);

                    if let Ok((bytes, _)) = file.load_bytes_future().await {
                        if let Ok(texture) = gtk::gdk::Texture::from_bytes(&bytes) {
                            pic_clone.set_paintable(Some(&texture));
                        }
                    }
                });
            }
            break;
        }
    });

    window.present();
}
