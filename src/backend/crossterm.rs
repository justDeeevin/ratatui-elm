use crossterm::event::EventStream;
use ratatui::crossterm::event::Event;

impl super::Backend for ratatui::backend::CrosstermBackend<std::io::Stdout> {
    type Event = Event;
    type Error = std::io::Error;
    type EventStream = EventStream;

    fn init() -> ratatui::Terminal<Self> {
        ratatui::init()
    }

    fn restore() {
        ratatui::restore();
    }
}

impl super::Event for Event {
    fn is_resize(&self) -> bool {
        matches!(self, Event::Resize(..))
    }
}
