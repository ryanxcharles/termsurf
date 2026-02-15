use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

#[derive(PartialEq)]
enum Mode {
    Browse,
    Control,
}

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

    let mut mode = Mode::Browse;

    // Event loop.
    loop {
        terminal.draw(|frame| ui(frame, &url, &mode))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                // Ctrl+C quits from any mode.
                if key.code == KeyCode::Char('c')
                    && key.modifiers.contains(KeyModifiers::CONTROL)
                {
                    break;
                }

                match mode {
                    Mode::Browse => {
                        if key.code == KeyCode::Esc {
                            mode = Mode::Control;
                        }
                    }
                    Mode::Control => match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Enter => mode = Mode::Browse,
                        _ => {}
                    },
                }
            }
        }
    }

    // Restore terminal.
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

fn ui(frame: &mut Frame, url: &str, mode: &Mode) {
    let layout = Layout::vertical([
        Constraint::Length(3), // URL bar (1 line + top/bottom border)
        Constraint::Min(1),   // Viewport (fill remaining)
        Constraint::Length(1), // Status bar
    ])
    .split(frame.area());

    // URL bar.
    let url_bar = Paragraph::new(url).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" URL ")
            .border_style(Style::default().fg(Color::Gray))
            .title_style(Style::default().fg(Color::Gray)),
    );
    frame.render_widget(url_bar, layout[0]);

    // Viewport.
    let viewport_text = "waiting for browser...";
    let viewport = Paragraph::new(viewport_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Viewport ")
                .border_style(Style::default().fg(Color::Gray))
                .title_style(Style::default().fg(Color::Gray)),
        );
    frame.render_widget(viewport, layout[1]);

    // Status bar.
    let status_layout = Layout::horizontal([
        Constraint::Fill(1),   // Key hints (left)
        Constraint::Length(10), // Mode label (right)
    ])
    .split(layout[2]);

    let (hints, label) = match mode {
        Mode::Browse => ("[esc] control mode", "BROWSE"),
        Mode::Control => ("[q] quit  [enter] browse", "CONTROL"),
    };

    let hints_widget = Paragraph::new(hints)
        .style(Style::default().fg(Color::Gray));
    frame.render_widget(hints_widget, status_layout[0]);

    let label_widget = Paragraph::new(label)
        .alignment(Alignment::Right)
        .style(Style::default().fg(Color::Gray));
    frame.render_widget(label_widget, status_layout[1]);
}
