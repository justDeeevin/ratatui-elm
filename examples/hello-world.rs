use crossterm::event::{Event, KeyCode};
use ratatui::{
    text::Text,
    widgets::{Block, Borders},
};
use ratatui_elm::{Task, Update};

fn main() {
    ratatui_elm::App::new(update, view).run().unwrap();
}

fn update(_state: &mut (), event: Update<()>) -> (Task<()>, bool) {
    let task = if let Update::Terminal(Event::Key(e)) = event
        && matches!(e.code, KeyCode::Char('q') | KeyCode::Esc)
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
