use futures::Stream;
use ratatui::{
    Terminal,
    backend::TermwizBackend,
    termwiz::{
        self,
        caps::Capabilities,
        input::InputEvent,
        terminal::{Terminal as _, UnixTerminal, buffered::BufferedTerminal},
    },
};
use tokio::sync::mpsc;

impl super::Backend for TermwizBackend {
    type Event = InputEvent;
    type Error = termwiz::Error;
    type EventStream = TermwizEventStream;

    fn init() -> Terminal<Self> {
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            Self::restore();
            hook(info);
        }));

        Terminal::new(TermwizBackend::new().expect("Failed to create backend"))
            .expect("Failed to create terminal")
    }

    fn restore() {
        match new_terminal() {
            Ok(mut terminal) => {
                if let Err(e) = terminal.exit_alternate_screen() {
                    eprintln!("Failed to leave alternate screen: {e}");
                }
                if let Err(e) = terminal.set_cooked_mode() {
                    eprintln!("Failed to set raw mode: {e}");
                }
            }
            Err(e) => {
                eprintln!("Failed to create terminal: {e}");
            }
        }
    }

    fn handle_resize(&mut self, width: u16, height: u16) {
        self.buffered_terminal_mut()
            .resize(width as usize, height as usize);
    }
}

fn new_terminal() -> termwiz::Result<UnixTerminal> {
    UnixTerminal::new_from_stdio(Capabilities::new_from_env()?)
}

impl super::Event for InputEvent {
    fn resize(&self) -> Option<(u16, u16)> {
        if let InputEvent::Resized { cols, rows } = self {
            BufferedTerminal::new(new_terminal().unwrap())
                .unwrap()
                .resize(*cols, *rows);
            Some((*cols as u16, *rows as u16))
        } else {
            None
        }
    }
}

pub struct TermwizEventStream {
    rx: mpsc::UnboundedReceiver<termwiz::Result<InputEvent>>,
}

impl Default for TermwizEventStream {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut terminal = new_terminal().unwrap();

        std::thread::spawn(move || {
            while let Ok(e) = terminal.poll_input(None).transpose().unwrap() {
                tx.send(Ok(e)).unwrap();
            }
        });

        Self { rx }
    }
}

impl Stream for TermwizEventStream {
    type Item = termwiz::Result<InputEvent>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}
