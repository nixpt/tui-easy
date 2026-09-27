//! Terminal initialisation and restoration.
//!
//! [`init()`] enters raw mode, switches to the alternate screen, enables
//! mouse capture, and installs a panic hook that guarantees terminal
//! restoration on crash. [`restore()`] reverses everything on clean exit.
//!
//! Consumers should not need to call these directly — [`crate::run_app()`]
//! handles them automatically.

use ratatui::crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io::{self, Stdout};

/// Type alias for the standard terminal
pub type ExoTerminal = Terminal<CrosstermBackend<Stdout>>;

/// Initialize a crossterm terminal with alternate screen and mouse capture.
///
/// Sets up a panic hook that restores the terminal on crash.
pub fn init() -> io::Result<ExoTerminal> {
    install_panic_hook();
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

/// Restore the terminal to its original state.
///
/// Call this on normal exit. The panic hook calls it automatically on crash.
pub fn restore() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
}

/// Install a panic hook that restores the terminal before printing the panic.
fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        restore();
        original_hook(panic_info);
    }));
}
