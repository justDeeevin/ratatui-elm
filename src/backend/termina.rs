use futures::{StreamExt, stream::Fuse};
use ratatui_core::terminal::Terminal;
use std::io::Write;
use termina::{
    PlatformTerminal, Terminal as _, WindowSize,
    escape::csi::{Csi, DecPrivateMode, DecPrivateModeCode, Mode},
};

pub type TerminaBackend = ratatui_termina::TerminaBackend<PlatformTerminal>;

impl<R> super::Backend<R> for TerminaBackend {
    type Event = termina::Event;
    type EventStream = Fuse<termina::EventStream>;

    fn init() -> ratatui_core::terminal::Terminal<Self> {
        let mut terminal = PlatformTerminal::new().unwrap();
        terminal.set_panic_hook(|_| <Self as super::Backend<R>>::restore());
        terminal.enter_raw_mode().unwrap();
        write!(
            &mut terminal,
            "{}",
            Csi::Mode(Mode::SetDecPrivateMode(DecPrivateMode::Code(
                DecPrivateModeCode::ClearAndEnableAlternateScreen
            )))
        )
        .unwrap();
        Terminal::new(TerminaBackend::new(terminal)).unwrap()
    }

    fn restore() {
        match PlatformTerminal::new() {
            Ok(mut terminal) => {
                if let Err(e) = write!(
                    &mut terminal,
                    "{}",
                    Csi::Mode(Mode::ResetDecPrivateMode(DecPrivateMode::Code(
                        DecPrivateModeCode::ClearAndEnableAlternateScreen
                    )))
                ) {
                    eprintln!("Failed to leave alternate screen: {e}");
                }
                if let Err(e) = terminal.enter_cooked_mode() {
                    eprintln!("Failed to set raw mode: {e}");
                }
            }
            Err(e) => {
                eprintln!("Failed to create terminal: {e}");
            }
        }
    }
}

impl super::Event for termina::Event {
    fn resize(&self) -> Option<(u16, u16)> {
        if let termina::Event::WindowResized(WindowSize { cols, rows, .. }) = self {
            Some((*cols, *rows))
        } else {
            None
        }
    }
}

impl super::New for Fuse<termina::EventStream> {
    fn new() -> Self {
        termina::EventStream::new(PlatformTerminal::new().unwrap().event_reader(), |_| true).fuse()
    }
}
