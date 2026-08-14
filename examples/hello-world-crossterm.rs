use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui_core::text::Text;
use ratatui_elm::{Task, Update, backend::CrosstermBackend, runtime::Tokio};
use ratatui_widgets::{block::Block, borders::Borders};

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

fn view(_state: &mut (), frame: &mut ratatui_core::terminal::Frame) {
    let block = Block::default()
        .title("Hello, world!")
        .borders(Borders::ALL);
    frame.render_widget(&block, frame.area());
    frame.render_widget(Text::raw("Hello, world!"), block.inner(frame.area()));
}
