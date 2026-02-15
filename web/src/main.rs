use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

fn main() -> io::Result<()> {
    let url = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: web <url>");
        std::process::exit(1);
    });

    // Enter raw mode and alternate screen.
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    // Event loop.
    loop {
        terminal.draw(|frame| ui(frame, &url))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q')
                    || (key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL))
                {
                    break;
                }
            }
        }
    }

    // Restore terminal.
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

fn ui(frame: &mut Frame, url: &str) {
    let layout = Layout::vertical([
        Constraint::Length(3), // URL bar (1 line + top/bottom border)
        Constraint::Min(1),   // Viewport (fill remaining)
        Constraint::Length(1), // Status bar
    ])
    .split(frame.area());

    // URL bar.
    let url_bar = Paragraph::new(url)
        .block(Block::default().borders(Borders::ALL).title(" URL "));
    frame.render_widget(url_bar, layout[0]);

    // Viewport.
    let viewport_text = "waiting for browser...";
    let viewport = Paragraph::new(viewport_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Viewport "),
        );
    frame.render_widget(viewport, layout[1]);

    // Status bar.
    let status = Paragraph::new("[q] quit")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(status, layout[2]);
}
