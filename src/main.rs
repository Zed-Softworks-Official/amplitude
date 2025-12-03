mod ui;

use gtk4::glib;
use ui::AmplitudeApplication;

fn main() -> glib::ExitCode {
    spdlog::default_logger().set_level_filter(spdlog::LevelFilter::All);

    let app = AmplitudeApplication::new();
    app.expect("An Error Occured").run()
}

