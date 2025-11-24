use futures::{
    Stream, StreamExt,
    stream::{BoxStream, SelectAll},
};
use ratatui::{
    Terminal,
    termion::{
        event::Event as TermionEvent,
        input::TermRead,
        raw::{IntoRawMode, RawTerminal},
        screen::{AlternateScreen, IntoAlternateScreen},
        terminal_size,
    },
};
use signal_hook_tokio::Signals;
use std::io::Result;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

pub type TermionBackend =
    ratatui::backend::TermionBackend<AlternateScreen<RawTerminal<std::io::Stdout>>>;

/// Termion events _or_ resize events.
///
/// Termion itself doesn't expose resizes. ratatui-elm manually handles signals internally, and
/// exposes them here.
pub enum Event {
    Termion(TermionEvent),
    Resize(u16, u16),
}

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

/// An asynchronous stream of termion events.
pub struct TermionEventStream {
    select: SelectAll<BoxStream<'static, Result<Event>>>,
}

impl Default for TermionEventStream {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        std::thread::spawn(move || {
            for event in std::io::stdin().events() {
                tx.send(event).unwrap();
            }
        });

        let mut select: SelectAll<BoxStream<'static, Result<Event>>> = SelectAll::new();
        select.push(Box::pin(
            UnboundedReceiverStream::new(rx).map(|r| r.map(Event::Termion)),
        ));
        select.push(Box::pin(async_stream::stream! {
            let mut signals = Signals::new([signal_hook::consts::SIGWINCH]).unwrap();
            while signals.next().await.is_some() {
                let (x, y) = terminal_size()?;
                yield Ok(Event::Resize(x, y));
            }
        }));

        Self { select }
    }
}

impl Stream for TermionEventStream {
    type Item = Result<Event>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.select.poll_next_unpin(cx)
    }
}

impl super::Event for Event {
    fn resize(&self) -> Option<(u16, u16)> {
        if let Event::Resize(width, height) = self {
            Some((*width, *height))
        } else {
            None
        }
    }
}
