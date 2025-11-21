//! A simple Elm architecture framework for ratatui.
//!
//! The architecture is heavily inspired by [iced](https://github.com/iced-rs/iced). It provides an ergonomic interface for executing long-running tasks in the background and handling events concurrently, while only rerendering when strictly necessary.
//!
//! See [the hello world example](https://github.com/justdeeevin/ratatui-elm/blob/main/examples/hello-world.rs) for a basic usage example.
//!
//! <div class="warning">
//! This framework provides a built-in subscription to crossterm events. <strong>Do not manually
//! construct an instance of <code>EventStream</code></strong>, as crossterm only
//! sends events to one stream at a time, and the construction of a second stream will cause the
//! two to fight over each event.
//! </div>

use crossterm::event::EventStream;
use futures::{
    Stream, StreamExt,
    future::BoxFuture,
    stream::{BoxStream, SelectAll},
};
use ratatui::DefaultTerminal;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

/// A trait for a struct that can update the state of the application.
///
/// You shouldn't need to manually implement this trait. The provided implementation should be
/// sufficient.
pub trait Updater<State, M> {
    fn update(&self, state: &mut State, update: Update<M>) -> (Task<M>, bool);
}

impl<State, M, F: Fn(&mut State, Update<M>) -> (Task<M>, bool)> Updater<State, M> for F {
    fn update(&self, state: &mut State, update: Update<M>) -> (Task<M>, bool) {
        self(state, update)
    }
}

/// A trait for a struct that can render the state of the application.
///
/// You shouldn't need to manually implement this trait. The provided implementation should be
/// sufficient.
pub trait Viewer<State> {
    fn view(&self, state: &mut State, frame: &mut ratatui::Frame);
}

impl<State, F: Fn(&mut State, &mut ratatui::Frame)> Viewer<State> for F {
    fn view(&self, state: &mut State, frame: &mut ratatui::Frame) {
        self(state, frame)
    }
}

/// A message to be sent to the application.
pub enum Update<M> {
    /// A crossterm event.
    Terminal(crossterm::event::Event),
    /// A message of user-defined type.
    Message(M),
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
pub struct App<M, U: Updater<State, M>, V: Viewer<State>, State = ()> {
    updater: U,
    viewer: V,
    state: State,
    rx: UnboundedReceiver<M>,
    tx: UnboundedSender<M>,
    event_stream: EventStream,
    subscriptions: SelectAll<BoxStream<'static, M>>,
}

impl<State, M: Send + 'static, U: Updater<State, M>, V: Viewer<State>> App<M, U, V, State> {
    /// Create a new application with default initial state.
    pub fn new(update: U, view: V) -> Self
    where
        State: Default,
    {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        Self {
            updater: update,
            viewer: view,
            state: State::default(),
            tx,
            rx,
            event_stream: EventStream::new(),
            subscriptions: SelectAll::new(),
        }
    }

    /// Create a new application with a custom initial state.
    pub fn new_with(state: State, update: U, view: V) -> Self {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        Self {
            updater: update,
            viewer: view,
            state,
            tx,
            rx,
            event_stream: EventStream::new(),
            subscriptions: SelectAll::new(),
        }
    }

    /// Add a subscription to the application.
    pub fn subscription(mut self, subscription: impl Stream<Item = M> + 'static + Send) -> Self {
        self.subscriptions.push(Box::pin(subscription));
        self
    }

    /// Run the application.
    pub fn run(self) -> std::io::Result<()> {
        let terminal = ratatui::init();
        let res = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to build tokio runtime")
            .block_on(self.run_inner(terminal));
        ratatui::restore();
        res
    }

    async fn run_inner(mut self, mut terminal: DefaultTerminal) -> std::io::Result<()> {
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
            let resize = matches!(
                update,
                Update::Terminal(crossterm::event::Event::Resize(..))
            );
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

#[allow(
    dead_code,
    reason = "this is to ensure that my crossterm version matches ratatui's re-export"
)]
fn crossterm_version() -> ratatui::crossterm::event::Event {
    crossterm::event::Event::FocusGained
}
