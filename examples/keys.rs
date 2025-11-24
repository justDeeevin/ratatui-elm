use ratatui::{
    Frame,
    crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers},
    text::{Line, Text},
};
use ratatui_elm::{App, Task, Update};

fn main() {
    App::new(update, view).run().unwrap();
}

fn view(state: &mut Vec<KeyEvent>, frame: &mut Frame) {
    let text = Text::from_iter(state.iter().map(|key| Line::raw(format!("{key:?}"))));
    frame.render_widget(text, frame.area());
}

fn update(state: &mut Vec<KeyEvent>, update: Update<()>) -> (Task<()>, bool) {
    let render = match update {
        Update::Terminal(Event::Key(KeyEvent {
            code: KeyCode::Char('c'),
            modifiers,
            ..
        })) if modifiers == KeyModifiers::CONTROL => {
            return (Task::Quit, false);
        }
        Update::Terminal(Event::Key(event)) => {
            state.push(event);
            true
        }
        _ => false,
    };
    (Task::None, render)
}
