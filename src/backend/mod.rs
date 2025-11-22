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

pub trait Backend: ratatui::backend::Backend + Sized {
    type Event: Event;
    type Error: std::error::Error;
    type EventStream: futures::Stream<Item = Result<Self::Event, Self::Error>> + Default + Unpin;

    fn init() -> Terminal<Self>;
    fn restore();
}

pub trait Event {
    fn is_resize(&self) -> bool;
}
