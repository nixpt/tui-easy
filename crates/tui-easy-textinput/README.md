# tui-easy-textinput

A multi-line text input widget for [ratatui](https://ratatui.rs): `TextArea`
with `TextAreaState`, an edit buffer with undo/redo, a keymap, mouse
selection and soft wrapping.

Part of [tui-easy](https://github.com/nixpt/tui-easy); most users depend on the
`tui-easy` umbrella crate instead of this crate directly.

Key and mouse event types are ratatui's re-exported crossterm
(`ratatui::crossterm`), so they always match your ratatui backend.

## Features

- `debug-logs`: log unhandled key events through `tracing`.

## Attribution

Vendored from the `xai-ratatui-textarea` crate of
[xai-org/grok-build](https://github.com/xai-org/grok-build) (Apache-2.0). See
[NOTICE](NOTICE) for the exact files.

## License

Apache-2.0 only (the upstream code is Apache-2.0 only). See
[LICENSE-APACHE](LICENSE-APACHE).
