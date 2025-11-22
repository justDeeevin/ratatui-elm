use std::time::Duration;

use futures::Stream;
use ratatui::{
    Terminal,
    backend::TermwizBackend,
    termwiz::{
        self,
        caps::Capabilities,
        input::InputEvent,
        terminal::{Terminal as _, UnixTerminal},
    },
};
use tokio::sync::{mpsc, oneshot};

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
}

fn new_terminal() -> termwiz::Result<UnixTerminal> {
    UnixTerminal::new_from_stdio(Capabilities::new_from_env()?)
}

impl super::Event for InputEvent {
    fn is_resize(&self) -> bool {
        matches!(self, InputEvent::Resized { .. })
    }
}

pub struct TermwizEventStream {
    rx: mpsc::UnboundedReceiver<termwiz::Result<InputEvent>>,
    killer: Option<oneshot::Sender<()>>,
}

impl Default for TermwizEventStream {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let (killer_tx, mut killer_rx) = oneshot::channel();

        let mut terminal = new_terminal().unwrap();

        std::thread::spawn(move || {
            loop {
                if let Ok(()) = killer_rx.try_recv() {
                    break;
                }
                if let Some(e) = terminal
                    .poll_input(Some(Duration::from_millis(100)))
                    .transpose()
                {
                    tx.send(e).unwrap();
                }
            }
        });

        Self {
            rx,
            killer: Some(killer_tx),
        }
    }
}

impl Drop for TermwizEventStream {
    fn drop(&mut self) {
        self.killer.take().unwrap().send(()).unwrap();
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
