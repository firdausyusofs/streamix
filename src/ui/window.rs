use adw::{Application, HeaderBar, NavigationPage, NavigationView, ToolbarView};
use gtk::prelude::*;
use gtk::{Box, Button, CssProvider};

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

        .sidebar-background {
            background-color: alpha(black, 0.85);
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

pub fn build_ui(app: &Application) {
    let style_manager = adw::StyleManager::default();
    style_manager.set_color_scheme(adw::ColorScheme::ForceDark);
    load_css();

    let nav_view = NavigationView::builder().build();
    let catalog_toolbar = ToolbarView::builder().build();

    let main_stack = crate::ui::catalog::build_catalog_stack(&nav_view);
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
        crate::ui::addons::show_addons_window(&window_clone);
    });

    window.present();
}
