use ratatui::{
    termion::event::{Event as TermionEvent, Key},
    text::Text,
    widgets::{Block, Borders},
};
use ratatui_elm::{
    Task, Tokio, Update,
    backend::{TermionBackend, termion::Event},
};

fn main() {
    ratatui_elm::AppWithBackend::<Tokio, TermionBackend>::new(update, view)
        .run()
        .unwrap();
}

fn update(_state: &mut (), event: Update<(), Event>) -> (Task<()>, bool) {
    let task =
        if let Update::Terminal(Event::Termion(TermionEvent::Key(Key::Char('q') | Key::Esc))) =
            event
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
