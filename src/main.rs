mod streamio;
mod ui;

use adw::prelude::*;
use adw::Application;

fn main() {
    let app = Application::builder()
        .application_id("com.fy.streamix")
        .build();

    app.connect_activate(ui::window::build_ui);

    app.run();
}
