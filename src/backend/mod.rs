#[cfg(feature = "crossterm")]
mod crossterm;
#[cfg(feature = "crossterm")]
pub use crossterm::CrosstermBackend;

#[cfg(feature = "termion")]
pub mod termion;
use futures::{Stream, stream::FusedStream};
#[cfg(feature = "termion")]
pub use termion::TermionBackend;

#[cfg(feature = "termwiz")]
mod termwiz;

use ratatui::Terminal;

/// Some extra functionality that a backend must have for ratatui-elm to work.
pub trait Backend<R>: ratatui::backend::Backend + Sized {
    /// The type of event that the backend produces.
    type Event: Event;
    /// The type of error that the backend produces.
    type Error: std::error::Error;
    /// An asynchronous stream of events.
    type EventStream: FusedStream + Stream<Item = Result<Self::Event, Self::Error>> + New + Unpin;

    /// Initialize the backend.
    fn init() -> Terminal<Self>;
    /// Restore the terminal to its original state.
    fn restore();

    fn handle_resize(&mut self, _width: u16, _height: u16) {}
}

/// Specific functionality a backend's event must have for ratatui-elm to work.
pub trait Event {
    /// Check if the event is a resize event.
    fn resize(&self) -> Option<(u16, u16)>;
}

/// Rewrite of [`Default`].
///
/// This is only necessary because crossterm's impl of [`Backend::EventStream`] uses [`futures::stream::Fuse`], which doesn't provide a blanked `Default` impl. ☹️
pub trait New {
    fn new() -> Self;
}
