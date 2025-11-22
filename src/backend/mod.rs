#[cfg(feature = "crossterm")]
mod crossterm;
#[cfg(feature = "crossterm")]
pub use crossterm::CrosstermBackend;

#[cfg(feature = "termion")]
mod termion;
#[cfg(feature = "termion")]
pub use termion::TermionBackend;

#[cfg(feature = "termwiz")]
mod termwiz;

use ratatui::Terminal;

/// Some extra functionality that a backend must have for ratatui-elm to work.
pub trait Backend: ratatui::backend::Backend + Sized {
    /// The type of event that the backend produces.
    type Event: Event;
    /// The type of error that the backend produces.
    type Error: std::error::Error;
    /// An asynchronous stream of events.
    type EventStream: futures::Stream<Item = Result<Self::Event, Self::Error>> + Default + Unpin;

    /// Initialize the backend.
    fn init() -> Terminal<Self>;
    /// Restore the terminal to its original state.
    fn restore();
}

/// Specific functionality a backend's event must have for ratatui-elm to work.
pub trait Event {
    /// Check if the event is a resize event.
    fn is_resize(&self) -> bool;
}
