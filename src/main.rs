mod stremio;
mod ui;

use std::sync::OnceLock;

use adw::prelude::*;
use adw::Application;
use tokio::runtime::Runtime;

pub static RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn main() {
    RUNTIME
        .set(Runtime::new().expect("Failed to create Tokio runtime"))
        .unwrap();

    RUNTIME.get().unwrap().spawn(async {
        crate::stremio::server::start_server().await;
    });

    let app = Application::builder()
        .application_id("com.fy.streamix")
        .build();

    app.connect_activate(ui::window::build_ui);

    app.run();
}
