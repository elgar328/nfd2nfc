pub mod app;
pub mod component;
pub mod dir_browser;
pub mod shortcuts;
pub mod styles;
pub mod tabs;
pub mod tick_timer;
pub mod toast;

use std::io;
use std::time::Duration;

use crossterm::event::{DisableMouseCapture, EnableMouseCapture, EventStream};
use crossterm::execute;
use futures::StreamExt;
use ratatui::DefaultTerminal;

use crate::daemon_controller;
use app::App;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Ensure launchd plist is installed before starting TUI
    daemon_controller::install_plist_if_missing()
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    // Setup terminal
    let mut terminal = ratatui::init();
    execute!(io::stdout(), EnableMouseCapture)?;

    // Create app state
    let mut app = App::new();

    // Run the main loop
    let result = tokio::runtime::Runtime::new()?.block_on(run_app(&mut terminal, &mut app));

    // Restore terminal
    execute!(io::stdout(), DisableMouseCapture)?;
    ratatui::restore();

    result
}

async fn run_app(
    terminal: &mut DefaultTerminal,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut event_stream = EventStream::new();
    let mut tick_interval = tokio::time::interval(Duration::from_millis(33));

    loop {
        // Force full redraw when tab switched (clears rendering artifacts)
        if app.force_redraw {
            terminal.clear()?;
            app.force_redraw = false;
        }

        // Draw UI
        terminal.draw(|f| {
            app::render::draw(f, app);
        })?;

        tokio::select! {
            // Handle keyboard/mouse events
            maybe_event = event_stream.next() => {
                app::events::handle_event(app, maybe_event)?;
            }
            // Periodic tick for updates
            _ = tick_interval.tick() => {
                app.tick();
            }
        }

        // Drain any queued events immediately to prevent scroll lag in Terminal.app
        while crossterm::event::poll(Duration::ZERO)? {
            let event = crossterm::event::read()?;
            app::events::handle_event(app, Some(Ok(event)))?;
        }

        if !app.running {
            break;
        }
    }

    Ok(())
}
