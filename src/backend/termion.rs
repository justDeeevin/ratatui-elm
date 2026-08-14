use async_signal::{Signal, Signals};
use byor::channel::mpsc::{RuntimeMpsc, UnboundedSender};
use futures::{
    Stream, StreamExt,
    stream::{BoxStream, FusedStream, SelectAll},
};
use ratatui_core::terminal::Terminal;
use std::{io, marker::PhantomData, task};
use termion::{
    event::Event as TermionEvent,
    input::TermRead,
    raw::{IntoRawMode, RawTerminal},
    screen::{AlternateScreen, IntoAlternateScreen},
    terminal_size,
};

pub type TermionBackend = ratatui_termion::TermionBackend<AlternateScreen<RawTerminal<io::Stdout>>>;

type Result<T, E = <TermionBackend as ratatui_core::backend::Backend>::Error> =
    std::result::Result<T, E>;

/// Termion events _or_ resize events.
///
/// Termion itself doesn't expose resizes. ratatui-elm manually handles signals internally, and
/// exposes them here.
pub enum Event {
    Termion(TermionEvent),
    Resize(u16, u16),
}

impl<R: RuntimeMpsc + Unpin> super::Backend<R> for TermionBackend
where
    <R as RuntimeMpsc>::UnboundedReceiver<Result<TermionEvent>>: Send + 'static,
    <R as RuntimeMpsc>::UnboundedSender<Result<TermionEvent>>: Send + 'static,
{
    type Event = Event;
    type EventStream = TermionEventStream<R>;

    fn init() -> ratatui_core::terminal::Terminal<Self> {
        let stdout = io::stdout()
            .into_raw_mode()
            .unwrap()
            .into_alternate_screen()
            .unwrap();
        Terminal::new(TermionBackend::new(stdout)).unwrap()
    }

    fn restore() {}
}

/// An asynchronous stream of termion events.
pub struct TermionEventStream<R: RuntimeMpsc + Unpin> {
    select: SelectAll<BoxStream<'static, Result<Event>>>,
    _marker: PhantomData<R>,
}

impl<R: RuntimeMpsc + Unpin> super::New for TermionEventStream<R>
where
    <R as RuntimeMpsc>::UnboundedReceiver<Result<TermionEvent>>: Send + 'static,
    <R as RuntimeMpsc>::UnboundedSender<Result<TermionEvent>>: Send + 'static,
{
    fn new() -> Self {
        let (tx, rx) = R::unbounded_channel();
        std::thread::spawn(move || {
            for event in io::stdin().events() {
                tx.send(event).unwrap();
            }
        });

        let mut select: SelectAll<BoxStream<'static, Result<Event>>> = SelectAll::new();
        select.push(Box::pin(rx.map(|r| r.map(Event::Termion))));
        select.push(Box::pin(async_stream::stream! {
            let mut signals = Signals::new([Signal::Winch]).unwrap();
            while signals.next().await.is_some() {
                let (x, y) = terminal_size()?;
                yield Ok(Event::Resize(x, y));
            }
        }));

        Self {
            select,
            _marker: PhantomData,
        }
    }
}

impl<R: RuntimeMpsc + Unpin> Stream for TermionEventStream<R> {
    type Item = Result<Event>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Option<Self::Item>> {
        self.select.poll_next_unpin(cx)
    }
}

impl<R: RuntimeMpsc + Unpin> FusedStream for TermionEventStream<R> {
    fn is_terminated(&self) -> bool {
        self.select.is_terminated()
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
