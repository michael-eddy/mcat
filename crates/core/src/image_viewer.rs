use crossterm::{
    cursor::MoveTo,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::Print,
    terminal::{Clear, ClearType, disable_raw_mode, enable_raw_mode},
};
use std::{
    io::{self, Write},
    time::Duration,
};

pub struct ImageViewerState {
    pub x: i32,
    pub y: i32,
    pub zoom: usize,
}

pub fn show_help_prompt(
    out: &mut impl Write,
    term_width: u16,
    term_height: u16,
    state: &ImageViewerState,
) -> io::Result<()> {
    let help_text = "[Arrow/hjkl] Move  [+/-] Zoom  [0] Reset  [q/ESC] Quit";
    let status_text = format!(
        "Position: ({}, {}) | Zoom: {}x",
        state.x, state.y, state.zoom
    );

    // Calculate positions (bottom of screen)
    let separator_line = 2; // Lines reserved for status/help
    let status_line = term_height.saturating_sub(separator_line);
    let help_line = term_height.saturating_sub(1);

    // Center the text horizontally
    let help_pos = term_width.saturating_sub(help_text.len() as u16) / 2;
    let status_pos = term_width.saturating_sub(status_text.len() as u16) / 2;

    // Add separator line
    execute!(
        out,
        MoveTo(0, status_line.saturating_sub(1)),
        Print(format!("{:━^width$}", "", width = term_width as usize)),
    )?;

    // Write status text
    execute!(out, MoveTo(status_pos, status_line), Print(&status_text),)?;

    // Write help text
    execute!(out, MoveTo(help_pos, help_line), Print(&help_text),)?;

    Ok(())
}

pub fn clear_screen(stdout: &mut impl std::io::Write) -> std::io::Result<()> {
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;
    Ok(())
}

pub fn run_interactive_viewer(
    mut callback: impl FnMut(&ImageViewerState) -> bool,
) -> std::io::Result<()> {
    enable_raw_mode()?;

    let mut state = ImageViewerState {
        x: 0,
        y: 0,
        zoom: 1,
    };

    // Initial callback
    let mut should_quit = callback(&state);
    let mut last_callback_time = std::time::Instant::now();
    let callback_throttle = std::time::Duration::from_millis(50);

    while !should_quit {
        if event::poll(Duration::from_millis(16))? {
            // ~60fps
            if let Event::Key(key) = event::read()? {
                let mut clicked_correct_key = false;
                match key {
                    // Quit (q or ESC)
                    KeyEvent {
                        code: KeyCode::Char('q'),
                        modifiers: KeyModifiers::NONE,
                        ..
                    }
                    | KeyEvent {
                        code: KeyCode::Esc, ..
                    } => break,

                    // Movement (arrow keys or hjkl)
                    KeyEvent {
                        code: KeyCode::Left,
                        ..
                    }
                    | KeyEvent {
                        code: KeyCode::Char('h'),
                        modifiers: KeyModifiers::NONE,
                        ..
                    } => {
                        clicked_correct_key = true;
                        state.x -= 1
                    }
                    KeyEvent {
                        code: KeyCode::Right,
                        ..
                    }
                    | KeyEvent {
                        code: KeyCode::Char('l'),
                        modifiers: KeyModifiers::NONE,
                        ..
                    } => {
                        clicked_correct_key = true;
                        state.x += 1
                    }
                    KeyEvent {
                        code: KeyCode::Up, ..
                    }
                    | KeyEvent {
                        code: KeyCode::Char('k'),
                        modifiers: KeyModifiers::NONE,
                        ..
                    } => {
                        clicked_correct_key = true;
                        state.y -= 1
                    }
                    KeyEvent {
                        code: KeyCode::Down,
                        ..
                    }
                    | KeyEvent {
                        code: KeyCode::Char('j'),
                        modifiers: KeyModifiers::NONE,
                        ..
                    } => {
                        clicked_correct_key = true;
                        state.y += 1
                    }

                    // Zoom (+, - or =)
                    KeyEvent {
                        code: KeyCode::Char('+'),
                        modifiers: KeyModifiers::NONE,
                        ..
                    }
                    | KeyEvent {
                        code: KeyCode::Char('='),
                        modifiers: KeyModifiers::NONE,
                        ..
                    } => {
                        clicked_correct_key = true;
                        state.zoom += 1
                    }
                    KeyEvent {
                        code: KeyCode::Char('-'),
                        modifiers: KeyModifiers::NONE,
                        ..
                    } => {
                        if state.zoom > 1 {
                            clicked_correct_key = true;
                            state.zoom -= 1
                        }
                    }

                    // Reset (0)
                    KeyEvent {
                        code: KeyCode::Char('0'),
                        modifiers: KeyModifiers::NONE,
                        ..
                    } => {
                        clicked_correct_key = true;
                        state.x = 0;
                        state.y = 0;
                        state.zoom = 1;
                    }

                    _ => {}
                }

                // Callback after each key press, but throttled
                if clicked_correct_key {
                    let now = std::time::Instant::now();
                    if now.duration_since(last_callback_time) >= callback_throttle {
                        should_quit = callback(&state);
                        last_callback_time = now;
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    Ok(())
}
