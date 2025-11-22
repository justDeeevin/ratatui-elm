use ratatui::{
    backend::TermwizBackend,
    termwiz::input::{InputEvent, KeyCode, KeyEvent},
    text::Text,
    widgets::{Block, Borders},
};
use ratatui_elm::{Task, Update};

fn main() {
    ratatui_elm::AppWithBackend::<TermwizBackend>::new(update, view)
        .run()
        .unwrap();
}

fn update(_state: &mut (), event: Update<(), InputEvent>) -> (Task<()>, bool) {
    let task = if let Update::Terminal(InputEvent::Key(KeyEvent {
        key: KeyCode::Char('q') | KeyCode::Escape,
        ..
    })) = event
    {
        Task::Quit
    } else {
        Task::None
    };
    (task, true)
}

fn view(_state: &mut (), frame: &mut ratatui::Frame) {
    let block = Block::default()
        .title("Hello, world!")
        .borders(Borders::ALL);
    frame.render_widget(&block, frame.area());
    frame.render_widget(Text::raw("Hello, world!"), block.inner(frame.area()));
}
