mod crossterm;

use ratatui::Terminal;

pub trait Backend: ratatui::backend::Backend + Sized {
    type Event: Event;
    type Stream: futures::Stream<Item = std::io::Result<Self::Event>> + Default + Unpin;

    fn init() -> Terminal<Self>;
}

pub trait Event {
    fn is_resize(&self) -> bool;
}
