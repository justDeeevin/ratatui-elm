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

use backend::{Backend, Event, New};
use byor::{
    channel::mpsc::{RuntimeMpsc, UnboundedSender},
    executor::{Executor, Handle, RuntimeExecutor},
};
use cfg_if::cfg_if;
use futures::{
    Stream, StreamExt,
    future::BoxFuture,
    stream::{BoxStream, Fuse, FusedStream, SelectAll},
};
use ratatui::{Frame, Terminal};
use std::sync::Arc;

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

cfg_if! {
    if #[cfg(all(feature = "crossterm", not(feature = "termwiz"), not(feature = "termion")))] {
        /// A message to be sent to the application.
        pub enum Update<M, E: Event = ratatui::crossterm::event::Event> {
            /// A crossterm event.
            Terminal(E),
            /// A message of user-defined type.
            Message(M),
        }
    } else if #[cfg(all(feature = "termwiz", not(feature = "crossterm"), not(feature = "termion")))] {
        /// A message to be sent to the application.
        pub enum Update<M, E: Event = ratatui::termwiz::input::InputEvent> {
            /// A termwiz event.
            Terminal(E),
            /// A message of user-defined type.
            Message(M),
        }
    } else if #[cfg(all(feature = "termion", not(feature = "crossterm"), not(feature = "termwiz")))] {
        /// A message to be sent to the application.
        pub enum Update<M, E: Event = backend::termion::Event> {
            /// A termion event.
            Terminal(E),
            /// A message of user-defined type.
            Message(M),
        }
    } else {
        /// A message to be sent to the application.
        pub enum Update<M, E: Event> {
            /// A terminal event.
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
    pub fn perform(future: impl Future<Output = T> + Send + 'static) -> Self {
        Task::Perform(Box::pin(future))
    }
}

trait TaskFutExt<T: 'static> {
    async fn run(self, tx: impl UnboundedSender<T>);
}

impl<T: 'static, F: Future<Output = T>> TaskFutExt<T> for F {
    async fn run(self, tx: impl UnboundedSender<T>) {
        tx.send(self.await).unwrap();
    }
}

/// A ratatui application.
pub struct App<
    M: 'static,
    U: Updater<State, M, B::Event>,
    V: Viewer<State>,
    B: Backend<R>,
    R: RuntimeExecutor + RuntimeMpsc,
    State = (),
> {
    updater: U,
    viewer: V,
    state: State,
    rx: Fuse<<R as RuntimeMpsc>::UnboundedReceiver<M>>,
    tx: <R as RuntimeMpsc>::UnboundedSender<M>,
    event_stream: B::EventStream,
    subscriptions: SelectAll<BoxStream<'static, M>>,
    executor: Arc<R::Executor>,
}

/// Lets you construct an [`App`] with a custom backend in a more convenient way.
pub struct AppWithBackend<R, B>(std::marker::PhantomData<(R, B)>);

impl<R: RuntimeExecutor + RuntimeMpsc, B: Backend<R>> AppWithBackend<R, B> {
    #[allow(clippy::new_ret_no_self)]
    /// Create a new application with default initial state.
    pub fn new<State: Default, M, U: Updater<State, M, B::Event>, V: Viewer<State>>(
        update: U,
        view: V,
    ) -> App<M, U, V, B, R, State> {
        let (tx, rx) = R::unbounded_channel();
        let executor = Arc::new(R::Executor::new().expect("Failed to build executor"));
        App {
            updater: update,
            viewer: view,
            state: State::default(),
            tx,
            rx: rx.fuse(),
            event_stream: B::EventStream::new(),
            subscriptions: SelectAll::new(),
            executor,
        }
    }

    /// Create a new application with a custom initial state.
    pub fn new_with<State, M, U: Updater<State, M, B::Event>, V: Viewer<State>>(
        state: State,
        update: U,
        view: V,
    ) -> App<M, U, V, B, R, State> {
        let (tx, rx) = R::unbounded_channel();
        let executor = Arc::new(R::Executor::new().expect("Failed to build executor"));
        App {
            updater: update,
            viewer: view,
            state,
            tx,
            rx: rx.fuse(),
            event_stream: B::EventStream::new(),
            subscriptions: SelectAll::new(),
            executor,
        }
    }
}

#[cfg(feature = "futures")]
pub use byor::runtime::Futures;
#[cfg(feature = "smol")]
pub use byor::runtime::Smol;
#[cfg(feature = "tokio")]
pub use byor::runtime::Tokio;

cfg_if! {
    if #[cfg(all(feature = "tokio", not(feature = "smol"), not(feature = "futures")))] {
        pub type DefaultRuntime = Tokio;
    } else if #[cfg(all(feature = "smol", not(feature = "tokio"), not(feature = "futures")))] {
        pub type DefaultRuntime = Smol;
    } else if #[cfg(all(feature = "futures", not(feature = "tokio"), not(feature = "smol")))] {
        pub type DefaultRuntime = Futures;
    }
}

cfg_if! {
    if #[cfg(all(feature = "crossterm", not(feature = "termwiz"), not(feature = "termion")))] {
        pub type DefaultBackend = backend::CrosstermBackend;
        type DefaultEvent = ratatui::crossterm::event::Event;
    } else if #[cfg(all(feature = "termwiz", not(feature = "crossterm"), not(feature = "termion")))] {
        pub type DefaultBackend = ratatui::backend::TermwizBackend;
        type DefaultEvent = ratatui::termwiz::input::InputEvent;
    } else if #[cfg(all(feature = "termion", not(feature = "crossterm"), not(feature = "termwiz")))] {
        pub type DefaultBackend = backend::TermionBackend;
        type DefaultEvent = backend::termion::Event;
    }
}

cfg_if! {
    if #[cfg(all(
            any(feature = "tokio", feature = "smol", feature = "futures"),
            not(all(feature = "tokio", feature = "smol", feature = "futures")),
            any(feature = "crossterm", feature = "termion", feature = "termwiz"),
            not(all(feature = "crossterm", feature = "termion", feature = "termwiz"))
        ))] {
        impl<State, M, U: Updater<State, M, DefaultEvent>, V: Viewer<State>> App<M, U, V, DefaultBackend, DefaultRuntime, State> {
            /// Create a new application with default initial state.
            pub fn new(update: U, view: V) -> Self
            where
                State: Default,
            {
                AppWithBackend::<DefaultRuntime, DefaultBackend>::new(update, view)
            }

            /// Create a new application with a custom initial state.
            pub fn new_with(state: State, update: U, view: V) -> Self {
                AppWithBackend::<DefaultRuntime, DefaultBackend>::new_with(state, update, view)
            }
        }
    }
}

impl<
    State,
    M,
    U: Updater<State, M, B::Event>,
    V: Viewer<State>,
    B: Backend<R>,
    R: RuntimeExecutor + RuntimeMpsc,
> App<M, U, V, B, R, State>
where
    <R as RuntimeMpsc>::UnboundedSender<M>: Send + Sync + 'static,
    <R as RuntimeMpsc>::UnboundedReceiver<M>: Unpin,
    <B as Backend<R>>::EventStream: FusedStream,
{
    /// Add a subscription to the application.
    pub fn subscription(mut self, subscription: impl Stream<Item = M> + Send + 'static) -> Self {
        self.subscriptions.push(Box::pin(subscription));
        self
    }

    /// Run the application.
    pub fn run(self) -> std::io::Result<()> {
        let terminal = B::init();
        let res = self.executor.clone().block_on(self.run_inner(terminal));
        B::restore();
        res
    }

    async fn run_inner(mut self, mut terminal: Terminal<B>) -> std::io::Result<()> {
        let subscriptions_tx = self.tx.clone();
        self.executor
            .spawn(async move {
                while let Some(message) = self.subscriptions.next().await {
                    subscriptions_tx.send(message).unwrap();
                }
            })
            .detach();
        terminal.draw(|f| self.viewer.view(&mut self.state, f))?;
        loop {
            let update = futures::select! {
                message = self.rx.next() => {
                    match message {
                        Some(message) => Update::Message(message),
                        None => break,
                    }
                }
                e = self.event_stream.next() => match e {
                    Some(Ok(e)) => Update::Terminal(e),
                    _ => break,
                },
            };
            let resize = if let Update::Terminal(e) = &update {
                Event::resize(e)
            } else {
                None
            };
            if let Some((width, height)) = &resize {
                terminal.backend_mut().handle_resize(*width, *height);
            }
            let out = self.updater.update(&mut self.state, update);
            let task = out.0;
            let should_render = resize.is_some() || out.1;
            match task {
                Task::Perform(future) => {
                    self.executor.spawn(future.run(self.tx.clone())).detach();
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
