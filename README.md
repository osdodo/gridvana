# Gridvana

Gridvana 是一款基于网格的像素画与动画编辑器。

## macOS 打包

打包前请安装 Rust、Xcode Command Line Tools 和 `cargo-bundle`：

```bash
xcode-select -p
cargo install cargo-bundle --locked
```

在项目根目录执行：

```bash
cargo fetch
cargo test --workspace
cargo build --release --package app
cargo bundle --release --package app --format osx
codesign --force --deep --sign - target/release/bundle/osx/Gridvana.app
```

生成的应用位于 `target/release/bundle/osx/Gridvana.app`，可直接启动：

```bash
open target/release/bundle/osx/Gridvana.app
```

应用图标来自 `app/assets/logo.png`，Bundle 配置位于 `app/Cargo.toml`。仅执行
`cargo build --release` 会生成没有应用图标的裸可执行文件，因此请使用
`cargo bundle` 生成标准的 `.app` 应用包。

上述 `codesign` 命令使用 ad-hoc 签名，适合本地开发。正式分发前仍需使用
Apple Developer 证书签名，并完成公证。
