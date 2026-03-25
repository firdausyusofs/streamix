use std::sync::mpsc;
use std::thread;

use adw::prelude::*;
use adw::{Application, HeaderBar};
use gtk::{Box, FlowBox, ScrolledWindow, glib};

use crate::streamio::client;
use crate::streamio::models::MetaPreview;

pub fn build_ui(app: &Application) {
    let style_manager = adw::StyleManager::default();
    style_manager.set_color_scheme(adw::ColorScheme::ForceDark);

    let header_bar = HeaderBar::builder().build();

    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .build();

    let flow_box = FlowBox::builder()
        .valign(gtk::Align::Start)
        .max_children_per_line(5)
        .build();

    let content_box = Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();

    content_box.append(&header_bar);

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Streamix")
        .default_width(800)
        .default_height(600)
        .content(&content_box)
        .build();

    let (sender, mut receiver) = mpsc::channel::<Vec<MetaPreview>>();

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

    glib::spawn_future_local(async move {
        while let Some(movies) = receiver.recv().ok() {
            println!("Success! Received {} movies in the UI thread.", movies.len());

            break;
        }
    });

    window.present();
}
