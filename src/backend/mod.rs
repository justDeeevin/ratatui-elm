#[cfg(feature = "crossterm")]
mod crossterm;
#[cfg(feature = "termion")]
mod termion;
#[cfg(feature = "termwiz")]
mod termwiz;
#[cfg(feature = "termion")]
pub use termion::TermionBackend;

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
