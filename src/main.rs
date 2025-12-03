mod ui;
mod audio;

use ui::AmplitudeApplication;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = AmplitudeApplication::new().await?;
    app.run()
}
