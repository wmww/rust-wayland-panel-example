use gtk4::prelude::*;
use gtk4::{glib, Application, ApplicationWindow, Button};
use gtk4_layer_shell;

const APP_ID: &str = "me.phie.phie-shell";

fn activate(app: &Application) {
    let button = Button::builder()
        .label("Press me!")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    button.connect_clicked(|button| {
        button.set_label("Hello World!");
    });

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Phie Shell")
        .child(&button)
        .build();

    gtk4_layer_shell::init_for_window(&window);
    gtk4_layer_shell::set_anchor(&window, gtk4_layer_shell::Edge::Bottom, true);

    window.present();
}

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(activate);
    app.run()
}
