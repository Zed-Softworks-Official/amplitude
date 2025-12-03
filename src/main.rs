mod ui;

use ui::AmplitudeApplication;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = AmplitudeApplication::new();
    app.expect("An Error Occured").run();

    Ok(())
}
