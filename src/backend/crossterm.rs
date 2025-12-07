use crossterm::event::EventStream;
use futures::{StreamExt, stream::Fuse};
use ratatui::crossterm::event::Event;

use crate::backend::New;

pub type CrosstermBackend = ratatui::backend::CrosstermBackend<std::io::Stdout>;

impl<R> super::Backend<R> for CrosstermBackend {
    type Event = Event;
    type Error = std::io::Error;
    type EventStream = Fuse<EventStream>;

    fn init() -> ratatui::Terminal<Self> {
        ratatui::init()
    }

    fn restore() {
        ratatui::restore();
    }
}

impl super::Event for Event {
    fn resize(&self) -> Option<(u16, u16)> {
        if let Event::Resize(w, h) = self {
            Some((*w, *h))
        } else {
            None
        }
    }
}

impl New for Fuse<EventStream> {
    fn new() -> Self {
        EventStream::new().fuse()
    }
}
