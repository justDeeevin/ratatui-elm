use futures::{
    Stream, StreamExt,
    future::BoxFuture,
    stream::{BoxStream, SelectAll},
};
use ratatui::DefaultTerminal;
use std::sync::{LazyLock, Mutex};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

#[allow(clippy::type_complexity, reason = "IDFC")]
static QUIT: LazyLock<(UnboundedSender<()>, Mutex<UnboundedReceiver<()>>)> = LazyLock::new(|| {
    let (tx, rx) = mpsc::unbounded_channel();
    (tx, Mutex::new(rx))
});

pub trait Update<State, M: Message> {
    fn update(&self, state: &mut State, message: M) -> Task<M>;
}

impl<State, M: Message, F: Fn(&mut State, M) -> Task<M>> Update<State, M> for F {
    fn update(&self, state: &mut State, message: M) -> Task<M> {
        self(state, message)
    }
}

pub trait View<State> {
    fn view(&self, state: &mut State, frame: &mut ratatui::Frame);
}

impl<State, F: Fn(&mut State, &mut ratatui::Frame)> View<State> for F {
    fn view(&self, state: &mut State, frame: &mut ratatui::Frame) {
        self(state, frame)
    }
}

pub trait Message {
    /// Determines whether the message should trigger a re-render of the UI.
    fn should_render(&self) -> bool;
}

pub enum Task<T> {
    Some(BoxFuture<'static, T>),
    None,
}

impl<T> Task<T> {
    pub fn new(future: impl Future<Output = T> + 'static + Send) -> Self {
        Self::Some(Box::pin(future))
    }

    async fn run(self, tx: UnboundedSender<T>) {
        if let Self::Some(fut) = self {
            tx.send(fut.await).unwrap();
        }
    }
}

pub struct App<M: Message, U: Update<State, M>, V: View<State>, State = ()> {
    updater: U,
    viewer: V,
    state: State,
    rx: UnboundedReceiver<M>,
    tx: UnboundedSender<M>,
    subscriptions: SelectAll<BoxStream<'static, M>>,
    _marker: std::marker::PhantomData<M>,
}

impl<State, M: Message + Send + 'static, U: Update<State, M>, V: View<State>> App<M, U, V, State> {
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
            _marker: std::marker::PhantomData,
        }
    }

    pub fn new_with(state: State, update: U, view: V) -> Self {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        Self {
            updater: update,
            viewer: view,
            state,
            tx,
            rx,
            subscriptions: SelectAll::new(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn subscription(mut self, subscription: impl Stream<Item = M> + 'static + Send) -> Self {
        self.subscriptions.push(Box::pin(subscription));
        self
    }

    pub fn subscriptions(
        mut self,
        subscriptions: impl IntoIterator<Item = impl Stream<Item = M> + 'static + Send>,
    ) -> Self {
        self.subscriptions.extend(
            subscriptions
                .into_iter()
                .map(|s| Box::pin(s) as BoxStream<'static, M>),
        );
        self
    }

    pub fn run(self) -> std::io::Result<()> {
        let terminal = ratatui::init();
        let mut rx_quit = QUIT.1.lock().unwrap();
        let res = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to build tokio runtime")
            .block_on(self.run_inner(terminal, &mut rx_quit));
        ratatui::restore();
        res
    }

    async fn run_inner(
        mut self,
        mut terminal: DefaultTerminal,
        rx_quit: &mut UnboundedReceiver<()>,
    ) -> std::io::Result<()> {
        let subscriptions_tx = self.tx.clone();
        tokio::spawn(async move {
            while let Some(message) = self.subscriptions.next().await {
                subscriptions_tx.send(message).unwrap();
            }
        });
        terminal.draw(|f| self.viewer.view(&mut self.state, f))?;
        loop {
            tokio::select! {
                Some(()) = rx_quit.recv() => {
                    break;
                }
                Some(message) = self.rx.recv() => {
                    let should_render = message.should_render();
                    tokio::spawn(self.updater.update(&mut self.state, message).run(self.tx.clone()));
                    if should_render {
                        terminal.draw(|f| self.viewer.view(&mut self.state, f))?;
                    }
                }
            }
        }

        Ok(())
    }
}

pub fn quit() {
    QUIT.0.send(()).unwrap();
}
