# Gridvana

Gridvana is a grid-based pixel art and animation editor, written in Rust with
[`iced`](https://github.com/iced-rs/iced). It supports square, triangular, and
hexagonal grids, frame-by-frame animation, and it exposes its editing operations
over MCP so AI agents can draw alongside you.

![Gridvana](screenshots/1.jpg)

## Features

- **Multiple grid systems** — square, triangle, and hexagon canvases.
- **Drawing tools** — brush, eraser, paint bucket, color picker, line, rectangle
  and ellipse (filled or hollow), magic wand, and color select.
- **Selections & transforms** — box selection with implicit combine modes
  (Shift add, Alt subtract, Shift+Alt intersect), on-canvas move/scale/rotate
  handles, and floating pasted selections.
- **Animation** — timeline with frames, layers, tags, and a live preview panel.
- **Export** — PNG, animated GIF, and sprite sheets.
- **AI agent integration** — a built-in MCP server plus an embedded terminal, so
  an agent can start an edit session, preview operations, and commit them into
  your undo history.

## Building

```bash
cargo build --workspace
cargo test --workspace
cargo run --package app
```

## macOS packaging

Install Rust, the Xcode Command Line Tools, and `cargo-bundle`:

```bash
xcode-select -p
cargo install cargo-bundle --locked
```

Then from the project root:

```bash
cargo fetch
cargo test --workspace
cargo build --release --package app
cargo bundle --release --package app --format osx
codesign --force --deep --sign - target/release/bundle/osx/Gridvana.app
```

The resulting app lives at `target/release/bundle/osx/Gridvana.app`:

```bash
open target/release/bundle/osx/Gridvana.app
```

The icon comes from `app/assets/logo.png` and the bundle settings live in
`app/Cargo.toml`. A plain `cargo build --release` only produces a bare
executable with no icon, so use `cargo bundle` to get a proper `.app`.

The `codesign` command above uses ad-hoc signing, which is fine for local
development. Distribution still requires signing with an Apple Developer
certificate and notarizing the app.

## Releases

Pushing a `v*` tag runs `.github/workflows/release.yml`, which builds and
bundles the app for both Apple Silicon and Intel macOS and attaches the archives
to a GitHub release.

## Project layout

| Crate   | Description                                                        |
| ------- | ------------------------------------------------------------------ |
| `core/` | Document model, grid systems, compositing, transforms, persistence |
| `app/`  | The `iced` GUI application                                          |
| `mcp/`  | MCP server exposing the editing operations to AI agents             |

## License

Gridvana is licensed under the [GNU General Public License v3.0](LICENSE) or
later.
