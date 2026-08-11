# Gridvana macOS 打包指南

本文档说明如何将 Gridvana 打包为带应用图标的 `Gridvana.app`。

## 产物区别

`cargo build --release` 只生成裸 Mach-O 可执行文件：

```text
target/release/app
```

macOS 不会为这种裸二进制文件显示应用图标。需要使用 `cargo-bundle` 生成标准 `.app` 应用包，Finder、Dock 和应用切换器才会读取 Gridvana 的 Logo。

## 环境准备

确认已经安装 Rust 和 Xcode Command Line Tools：

```bash
rustc --version
xcode-select -p
```

首次打包前安装 `cargo-bundle`：

```bash
cargo install cargo-bundle --locked
```

## Logo 与配置

应用图标文件位于：

```text
app/assets/logo.png
```

建议保持为带透明背景的正方形 PNG，尺寸至少为 512×512。Bundle 名称、标识符和图标路径配置在 `app/Cargo.toml` 的 `[package.metadata.bundle]` 中。项目同时配置了从仓库根目录和 `app/` 目录执行 `cargo bundle` 时使用的相对路径。

替换 Logo 时必须保持文件路径不变，否则需要同步修改 `icon` 配置。

## 构建应用包

在项目根目录执行：

```bash
cargo fetch
cargo test --workspace
cargo build --release --package app
cargo bundle --release --package app --format osx
codesign --force --deep --sign - target/release/bundle/osx/Gridvana.app
```

`cargo fetch` 会提前下载 `cargo metadata` 所需的跨平台依赖，`cargo build` 会正常显示编译进度。`cargo bundle` 会复用已经完成的 release 构建，并且只生成 macOS `.app`，避免在无输出的状态下同时构建 DMG。最后的 `codesign` 会为本地应用包创建 ad-hoc 签名。

成功后产物位于：

```text
target/release/bundle/osx/Gridvana.app
```

启动应用：

```bash
open target/release/bundle/osx/Gridvana.app
```

安装到系统应用目录：

```bash
ditto target/release/bundle/osx/Gridvana.app /Applications/Gridvana.app
```

## 验证产物

检查应用包元数据：

```bash
plutil -p target/release/bundle/osx/Gridvana.app/Contents/Info.plist
ls -la target/release/bundle/osx/Gridvana.app/Contents/Resources
```

检查应用是否能正常启动：

```bash
open -W target/release/bundle/osx/Gridvana.app
```

## 更新 Logo 后重新打包

替换 `app/assets/logo.png` 后，删除旧应用包并重新构建：

```bash
rm -rf target/release/bundle/osx/Gridvana.app
cargo build --release --package app
cargo bundle --release --package app --format osx
codesign --force --deep --sign - target/release/bundle/osx/Gridvana.app
touch target/release/bundle/osx/Gridvana.app
```

如果 Finder 或 Dock 仍显示旧图标，可以刷新 macOS 图标缓存：

```bash
killall Dock
killall Finder
```

刷新只会重启 Dock 和 Finder，不会删除文件。

## 分发说明

本地生成的 `.app` 默认没有 Developer ID 签名。直接分发给其他用户时，macOS Gatekeeper 可能阻止启动。正式发布前还需要完成以下步骤：

1. 使用 Apple Developer 的 `Developer ID Application` 证书签名。
2. 将应用提交给 Apple 公证服务。
3. 将公证票据附加到 `.app`。
4. 压缩为 ZIP 或制作 DMG 后再分发。

检查当前签名状态：

```bash
codesign --verify --deep --strict --verbose=2 target/release/bundle/osx/Gridvana.app
spctl --assess --type execute --verbose=4 target/release/bundle/osx/Gridvana.app
```

开发阶段只需要本机使用时，无需完成正式签名与公证。

## 常见问题

### 二进制文件仍然没有 Logo

不要直接打开 `target/release/app`。它是 `.app` 内部使用的裸可执行文件，正确入口是：

```text
target/release/bundle/osx/Gridvana.app
```

### `cargo bundle` 命令不存在

重新安装打包工具：

```bash
cargo install cargo-bundle --locked
```

### `cargo bundle` 长时间没有输出

`cargo-bundle 0.11` 会隐藏内部 `cargo metadata` 和 `cargo build` 的实时输出。首次打包时，它可能正在下载跨平台依赖或编译 Iced，因此看起来像没有反应。

按顺序单独执行以下命令，让下载和编译进度保持可见：

```bash
cargo fetch
cargo build --release --package app
cargo bundle --release --package app --format osx
codesign --force --deep --sign - target/release/bundle/osx/Gridvana.app
```

如果 `cargo fetch` 报 `Could not resolve host: static.crates.io`，说明是网络或 DNS 问题，需要恢复 crates.io 访问后重试；这不是 Gridvana 代码或 Logo 配置导致的。

### 应用包显示通用图标

确认 `app/assets/logo.png` 是有效的正方形 PNG，然后删除旧 `.app`、重新打包并刷新 Dock/Finder。不要只执行 `cargo build --release`。

正常的应用包必须同时满足：

```text
Contents/Resources/Gridvana.icns 存在
Info.plist 包含 CFBundleIconFile = Gridvana.icns
```

如果两项都不存在，通常是 `cargo-bundle` 没有从当前工作目录匹配到配置中的相对图标路径。
