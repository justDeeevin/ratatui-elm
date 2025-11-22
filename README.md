A simple Elm architecture framework for ratatui.

The architecture is heavily inspired by [iced](https://github.com/iced-rs/iced). It provides an
ergonomic interface forr executing long-running tasks in the background and handling events
concurrently, while only re-rendering when strictly necessary.

See [the hello world
example](https://github.com/justdeeevin/ratatui-elm/blob/main/examples/hello-world-crossterm.rs)
for a basic usage example.

> [!WARNING]
> This framework provides a built-in subscription to terminal events. <strong>Do not manually
> subscribe to events</strong>, as this will cause the two subscriptions to fight over each event.

# Features

This crate works with all three officially supported ratatui backends:

- [crossterm](https://docs.rs/crossterm)—the default
- [termwiz](https://docs.rs/termwiz)
- [termion](https://docs.rs/termion)

There is a cargo feature for each backend implementation. **These feature flags are not
mutually exclusive**, though if you have only one enabled that backend will be used without
manual specification.
