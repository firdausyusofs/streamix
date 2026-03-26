use adw::{ActionRow, prelude::*};
use adw::{ApplicationWindow, EntryRow, PreferencesGroup, PreferencesPage, PreferencesWindow};
use gtk::{Button, Picture};

pub fn show_addons_window(parent: &ApplicationWindow) {
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

            gtk::glib::spawn_future_local(async move {
                let file = gtk::gio::File::for_uri(&logo_url);

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

            gtk::glib::spawn_future_local(async move {
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
