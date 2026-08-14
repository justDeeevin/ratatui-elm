use ratatui_core::text::Text;
use ratatui_elm::{Task, Update, runtime::Tokio};
use ratatui_termwiz::TermwizBackend;
use ratatui_widgets::{block::Block, borders::Borders};
use termwiz::input::{InputEvent, KeyCode, KeyEvent};

fn main() {
    ratatui_elm::AppWithBackend::<Tokio, TermwizBackend>::new(update, view)
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

fn view(_state: &mut (), frame: &mut ratatui_core::terminal::Frame) {
    let block = Block::default()
        .title("Hello, world!")
        .borders(Borders::ALL);
    frame.render_widget(&block, frame.area());
    frame.render_widget(Text::raw("Hello, world!"), block.inner(frame.area()));
}
