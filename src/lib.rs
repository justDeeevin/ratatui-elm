//! A simple Elm architecture framework for ratatui.
//!
//! The architecture is heavily inspired by [iced](https://github.com/iced-rs/iced). It provides an ergonomic interface for executing long-running tasks in the background and handling events concurrently, while only rerendering when strictly necessary.
//!
//! See [the hello world example](https://github.com/justdeeevin/ratatui-elm/blob/main/examples/hello-world.rs) for a basic usage example.

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
pub trait Update<State, M: Message> {
    fn update(&self, state: &mut State, message: M) -> Task<M>;
}

impl<State, M: Message, F: Fn(&mut State, M) -> Task<M>> Update<State, M> for F {
    fn update(&self, state: &mut State, message: M) -> Task<M> {
        self(state, message)
    }
}

/// A trait for a struct that can render the state of the application.
///
/// You shouldn't need to manually implement this trait. The provided implementation should be
/// sufficient.
pub trait View<State> {
    fn view(&self, state: &mut State, frame: &mut ratatui::Frame);
}

impl<State, F: Fn(&mut State, &mut ratatui::Frame)> View<State> for F {
    fn view(&self, state: &mut State, frame: &mut ratatui::Frame) {
        self(state, frame)
    }
}

/// A trait for messages that can be sent to the application.
pub trait Message {
    /// Determines whether the message should trigger a re-render of the UI.
    fn should_render(&self) -> bool;
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

trait TaskFutExt<T> {
    async fn run(self, tx: UnboundedSender<T>);
}

impl<T, F: Future<Output = T>> TaskFutExt<T> for F {
    async fn run(self, tx: UnboundedSender<T>) {
        tx.send(self.await).unwrap();
    }
}

/// A ratatui application.
pub struct App<M: Message, U: Update<State, M>, V: View<State>, State = ()> {
    updater: U,
    viewer: V,
    state: State,
    rx: UnboundedReceiver<M>,
    tx: UnboundedSender<M>,
    subscriptions: SelectAll<BoxStream<'static, M>>,
}

impl<State, M: Message + Send + 'static, U: Update<State, M>, V: View<State>> App<M, U, V, State> {
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
        while let Some(message) = self.rx.recv().await {
            let should_render = message.should_render();
            let task = self.updater.update(&mut self.state, message);
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
