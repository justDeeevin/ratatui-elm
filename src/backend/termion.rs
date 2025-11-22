use futures::Stream;
use ratatui::{
    Terminal,
    termion::{
        event::Event,
        input::TermRead,
        raw::{IntoRawMode, RawTerminal},
        screen::{AlternateScreen, IntoAlternateScreen},
        terminal_size,
    },
};
use std::{io::Result, sync::LazyLock};
use tokio::sync::mpsc;

pub type TermionBackend =
    ratatui::backend::TermionBackend<AlternateScreen<RawTerminal<std::io::Stdout>>>;

impl super::Backend for TermionBackend {
    type Event = Event;
    type Error = std::io::Error;
    type EventStream = TermionEventStream;

    fn init() -> ratatui::Terminal<Self> {
        let stdout = std::io::stdout()
            .into_raw_mode()
            .unwrap()
            .into_alternate_screen()
            .unwrap();
        Terminal::new(TermionBackend::new(stdout)).unwrap()
    }

    fn restore() {}
}

pub struct TermionEventStream {
    rx: mpsc::UnboundedReceiver<Result<Event>>,
}

impl Default for TermionEventStream {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        std::thread::spawn(move || {
            let mut events = std::io::stdin().events();
            loop {
                tx.send(events.next().unwrap()).unwrap();
            }
        });

        Self { rx }
    }
}

impl Stream for TermionEventStream {
    type Item = Result<Event>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

impl super::Event for Event {
    fn is_resize(&self) -> bool {
        static mut LAST_SIZE: LazyLock<(u16, u16)> = LazyLock::new(|| terminal_size().unwrap());

        let size = terminal_size().unwrap();
        // SAFETY: LAST_SIZE is only accessed from within this function and this function is only
        // called synchronously from the event loop.
        if size != unsafe { *LAST_SIZE } {
            unsafe { *LAST_SIZE = size };
            true
        } else {
            false
        }
    }
}
