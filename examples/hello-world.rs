use crossterm::event::{Event, EventStream, KeyCode};
use futures::StreamExt;
use ratatui::text::Text;
use ratatui_elm::Task;

struct Message(std::io::Result<Event>);

impl ratatui_elm::Message for Message {
    fn should_render(&self) -> bool {
        false
    }
}

fn main() {
    ratatui_elm::App::new(update, view)
        .subscription(EventStream::new().map(Message))
        .run()
        .unwrap();
}

fn update(_state: &mut (), message: Message) -> Task<Message> {
    if let Ok(Event::Key(e)) = message.0
        && matches!(e.code, KeyCode::Char('q') | KeyCode::Esc)
    {
        Task::Quit
    } else {
        Task::None
    }
}

fn view(_state: &mut (), frame: &mut ratatui::Frame) {
    frame.render_widget(Text::raw("Hello, world!"), frame.area())
}
