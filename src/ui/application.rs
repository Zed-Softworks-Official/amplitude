use gtk4 as gtk;
use gtk::prelude::*;
use gtk::{glib, Application, ApplicationWindow};

pub struct AmplitudeApplication {
    gtk_app: Application,
    // Audio Manager
}

impl AmplitudeApplication {
    pub fn new() -> Result<Self, ()> {
        let gtk_app = Application::builder()
            .application_id("dev.zedsoftworks.amplitude")
            .build();

        // TODO: Create Audio Manager

        Ok(Self {
            gtk_app
        })
    }

    pub fn run(&self) -> glib::ExitCode {
        // TODO: Audio Manager Stuff

        self.gtk_app.connect_activate(move |app| {
            let window = ApplicationWindow::builder()
                .application(app)
                .title("Amplitude")
                .default_width(1280)
                .default_height(720)
                .build();

            window.present();
        });

        self.gtk_app.run()
    }
}
