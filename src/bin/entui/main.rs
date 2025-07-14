use crate::app::App;

pub mod app;
pub mod event;
pub mod ui;
pub mod components;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let app = App::new()?;
    let terminal = ratatui::init();
    let result = app.run(terminal).await;
    ratatui::restore();
    result
}
