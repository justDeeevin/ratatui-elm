//! A simple Elm architecture framework for ratatui.
//!
//! The architecture is heavily inspired by [iced](https://github.com/iced-rs/iced). It provides an
//! ergonomic interface forr executing long-running tasks in the background and handling events
//! concurrently, while only re-rendering when strictly necessary.
//!
//! See [the hello world
//! example](https://github.com/justdeeevin/ratatui-elm/blob/main/examples/hello-world-crossterm.rs)
//! for a basic usage example.
//!
//! <div class="warning">
//! This framework provides a built-in subscription to terminal events. <strong>Do not manually
//! subscribe to events</strong>, as this will cause the two subscriptions to fight over each event.
//! </div>
//!
//! # Features
//!
//! This crate works with all three officially supported ratatui backends:
//!
//! - [crossterm](https://docs.rs/crossterm)—the default
//! - [termwiz](https://docs.rs/termwiz)
//! - [termion](https://docs.rs/termion)
//!
//! There is a cargo feature for each backend implementation. **These feature flags are not
//! mutually exclusive**, though if you have only one enabled that backend will be used without
//! manual specification.

pub mod backend;

use backend::{Backend, Event};
use futures::{
    Stream, StreamExt,
    future::BoxFuture,
    stream::{BoxStream, SelectAll},
};
use ratatui::{Frame, Terminal};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

/// A trait for a struct that can update the state of the application.
///
/// Returns a task to be executed along with whether the interface should be re-rendered.
///
/// You shouldn't need to manually implement this trait. The provided implementation should be
/// sufficient.
pub trait Updater<State, M, E: Event> {
    fn update(&self, state: &mut State, update: Update<M, E>) -> (Task<M>, bool);
}

impl<State, M, E: Event, F: Fn(&mut State, Update<M, E>) -> (Task<M>, bool)> Updater<State, M, E>
    for F
{
    fn update(&self, state: &mut State, update: Update<M, E>) -> (Task<M>, bool) {
        self(state, update)
    }
}

/// A trait for a struct that can render the state of the application.
///
/// You shouldn't need to manually implement this trait. The provided implementation should be
/// sufficient.
pub trait Viewer<State> {
    fn view(&self, state: &mut State, frame: &mut Frame);
}

impl<State, F: Fn(&mut State, &mut Frame)> Viewer<State> for F {
    fn view(&self, state: &mut State, frame: &mut Frame) {
        self(state, frame)
    }
}

cfg_if::cfg_if! {
    if #[cfg(all(feature = "crossterm", not(feature = "termwiz")))] {
        /// A message to be sent to the application.
        pub enum Update<M, E: Event = ratatui::crossterm::event::Event> {
            /// A crossterm event.
            Terminal(E),
            /// A message of user-defined type.
            Message(M),
        }
    } else if #[cfg(all(feature = "termwiz", not(feature = "crossterm")))] {
        /// A message to be sent to the application.
        pub enum Update<M, E: Event = termwiz::input::InputEvent> {
            /// A termion event.
            Terminal(E),
            /// A message of user-defined type.
            Message(M),
        }
    } else {
        /// A message to be sent to the application.
        pub enum Update<M, E: Event> {
            /// A crossterm event.
            Terminal(E),
            /// A message of user-defined type.
            Message(M),
        }
    }
}

/// A task to be executed by the runtime.
pub enum Task<T> {
    /// A future to execute in the background. The returned value will be sent back to the
    /// application.
    Perform(BoxFuture<'static, T>),
    /// What it sounds like. Ignored by the runtime.
    None,
    /// Quit the application.
    ///
    /// This simply breaks out of the runtime's main loop and allows program execution to
    /// continue to completion. It will not cancel any pending tasks.
    Quit,
}

impl<T> Task<T> {
    /// Create a new task that will be executed in the background.
    pub fn perform(future: impl Future<Output = T> + 'static + Send) -> Self {
        Task::Perform(Box::pin(future))
    }
}

trait TaskFutExt<T> {
    async fn run(self, tx: UnboundedSender<T>);
}

impl<T, F: Future<Output = T>> TaskFutExt<T> for F {
    async fn run(self, tx: UnboundedSender<T>) {
        tx.send(self.await).unwrap();
    }
}

/// A ratatui application.
pub struct App<M, U: Updater<State, M, B::Event>, V: Viewer<State>, B: Backend, State = ()> {
    updater: U,
    viewer: V,
    state: State,
    rx: UnboundedReceiver<M>,
    tx: UnboundedSender<M>,
    event_stream: B::EventStream,
    subscriptions: SelectAll<BoxStream<'static, M>>,
}

/// Lets you construct an [`App`] with a custom backend in a more convenient way.
pub struct AppWithBackend<B>(std::marker::PhantomData<B>);

impl<B: Backend> AppWithBackend<B> {
    #[allow(clippy::new_ret_no_self)]
    /// Create a new application with default initial state.
    pub fn new<State: Default, M, U: Updater<State, M, B::Event>, V: Viewer<State>>(
        update: U,
        view: V,
    ) -> App<M, U, V, B, State> {
        let (tx, rx) = unbounded_channel();
        App {
            updater: update,
            viewer: view,
            state: State::default(),
            tx,
            rx,
            event_stream: B::EventStream::default(),
            subscriptions: SelectAll::new(),
        }
    }

    /// Create a new application with a custom initial state.
    pub fn new_with<State, M, U: Updater<State, M, B::Event>, V: Viewer<State>>(
        state: State,
        update: U,
        view: V,
    ) -> App<M, U, V, B, State> {
        let (tx, rx) = unbounded_channel();
        App {
            updater: update,
            viewer: view,
            state,
            tx,
            rx,
            event_stream: B::EventStream::default(),
            subscriptions: SelectAll::new(),
        }
    }
}

#[cfg(all(feature = "crossterm", not(feature = "termwiz")))]
type CrosstermBackend = ratatui::backend::CrosstermBackend<std::io::Stdout>;

#[cfg(all(feature = "crossterm", not(feature = "termwiz")))]
impl<State, M, U: Updater<State, M, ratatui::crossterm::event::Event>, V: Viewer<State>>
    App<M, U, V, CrosstermBackend, State>
{
    pub fn new(update: U, view: V) -> Self
    where
        State: Default,
    {
        AppWithBackend::<CrosstermBackend>::new(update, view)
    }

    pub fn new_with(state: State, update: U, view: V) -> Self {
        AppWithBackend::<CrosstermBackend>::new_with(state, update, view)
    }
}

impl<State, M: Send + 'static, U: Updater<State, M, B::Event>, V: Viewer<State>, B: Backend>
    App<M, U, V, B, State>
{
    /// Add a subscription to the application.
    pub fn subscription(mut self, subscription: impl Stream<Item = M> + 'static + Send) -> Self {
        self.subscriptions.push(Box::pin(subscription));
        self
    }

    /// Run the application.
    pub fn run(self) -> std::io::Result<()> {
        let terminal = B::init();
        let res = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to build tokio runtime")
            .block_on(self.run_inner(terminal));
        ratatui::restore();
        res
    }

    async fn run_inner(mut self, mut terminal: Terminal<B>) -> std::io::Result<()> {
        let subscriptions_tx = self.tx.clone();
        tokio::spawn(async move {
            while let Some(message) = self.subscriptions.next().await {
                subscriptions_tx.send(message).unwrap();
            }
        });
        terminal.draw(|f| self.viewer.view(&mut self.state, f))?;
        loop {
            let update = tokio::select! {
                Some(message) = self.rx.recv() => {
                    Update::Message(message)
                }
                Some(Ok(e)) = self.event_stream.next() => Update::Terminal(e),
            };
            let resize = if let Update::Terminal(e) = &update {
                Event::is_resize(e)
            } else {
                false
            };
            let out = self.updater.update(&mut self.state, update);
            let task = out.0;
            let should_render = resize || out.1;
            match task {
                Task::Perform(future) => {
                    tokio::spawn(future.run(self.tx.clone()));
                }
                Task::None => {}
                Task::Quit => break,
            }
            if should_render {
                terminal.draw(|f| self.viewer.view(&mut self.state, f))?;
            }
        }

        Ok(())
    }
}
