use crossterm::event::EventStream;
use ratatui::crossterm::event::Event;

impl super::Backend for ratatui::backend::CrosstermBackend<std::io::Stdout> {
    type Event = Event;
    type Stream = EventStream;

    fn init() -> ratatui::Terminal<Self> {
        ratatui::init()
    }
}

impl super::Event for Event {
    fn is_resize(&self) -> bool {
        matches!(self, Event::Resize(..))
    }
}
