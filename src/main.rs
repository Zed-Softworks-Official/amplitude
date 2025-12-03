use gtk4 as gtk;
use gtk::prelude::*;
use gtk::{glib, Application, ApplicationWindow, Button};

fn main() -> glib::ExitCode {
    let app = Application::builder()
        .application_id("dev.zedsoftworks.amplitude")
        .build();

    app.connect_activate(|app| {
        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(1280)
            .default_height(720)
            .title("Amplitude")
            .build();

        let button = Button::with_label("Click Me!");
        button.connect_clicked(|_| {
            eprintln!("Clicked!");
        });
        window.set_child(Some(&button));

        window.present();
    });

    app.run()
}
