mod config;
mod curl;
mod img_preview;
mod openapi;
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, size},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::io;
use std::time::Duration;

mod app;
mod ui;
mod ai;
mod collections;

use crate::app::App;

#[tokio::main]
async fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App<'_>) -> io::Result<()> {
    loop {
        app.update();
        // Pasamos app como mutable
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(Duration::from_millis(16))? {
            let (width, height) = size()?;
            match event::read()? {
                Event::Key(key) => {
                    if let KeyCode::Char('q') = key.code {
                        if !app.input_mode { return Ok(()); }
                    }
                    app.handle_key(key);
                }
                Event::Mouse(mouse) => {
                    app.handle_mouse(mouse, width, height);
                }
                _ => {}
            }
        }
    }
}
