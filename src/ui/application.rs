use spdlog::prelude::*;

use gtk4 as gtk;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow};

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::audio::{AudioEvent, AudioManager, NodeManager, NodeType};

pub struct AmplitudeApplication {
    gtk_app: Application,
    audio_manager: Arc<RwLock<AudioManager>>
}

impl AmplitudeApplication {
    pub async fn new() -> anyhow::Result<Self> {
        let gtk_app = Application::builder()
            .application_id("dev.zedsoftworks.amplitude")
            .build();

        let audio_manager = Arc::new(RwLock::new(AudioManager::new().await?));

        Ok(Self {
            gtk_app,
            audio_manager
        })
    }

    pub fn run(&self) -> anyhow::Result<()> {
        let audio_manager = self.audio_manager.clone();

        self.gtk_app.connect_activate(move |app| {
            let am = audio_manager.clone();
            build_ui(app, am);
        });

        let am = self.audio_manager.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let mut manager = am.write().await;
                if let Err(e) = manager.run().await {
                    error!("Audio Manager error: {}", e);
                }
            });
        });

        self.gtk_app.run();
        Ok(())
    }
}

fn build_ui(app: &Application, audio_manager: Arc<RwLock<AudioManager>>) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Amplitude")
        .default_width(1280)
        .default_height(720)
        .build();

    window.present();
}
