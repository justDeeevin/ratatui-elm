use crossterm::event::KeyEvent;
use ratatui::{
    crossterm::event::{Event, KeyCode},
    text::Text,
    widgets::{Block, Borders},
};
use ratatui_elm::{Task, Tokio, Update, backend::CrosstermBackend};

fn main() {
    ratatui_elm::AppWithBackend::<Tokio, CrosstermBackend>::new(update, view)
        .run()
        .unwrap();
}

fn update(_state: &mut (), event: Update<(), Event>) -> (Task<()>, bool) {
    let task = if let Update::Terminal(Event::Key(KeyEvent {
        code: KeyCode::Char('q') | KeyCode::Esc,
        ..
    })) = event
    {
        Task::Quit
    } else {
        Task::None
    };
    (task, false)
}

fn view(_state: &mut (), frame: &mut ratatui::Frame) {
    let block = Block::default()
        .title("Hello, world!")
        .borders(Borders::ALL);
    frame.render_widget(&block, frame.area());
    frame.render_widget(Text::raw("Hello, world!"), block.inner(frame.area()));
}
