use base64::Engine;
use serde::Deserialize;

const XTERM_JS: &str = include_str!("../assets/xterm/xterm.js");
const XTERM_CSS: &str = include_str!("../assets/xterm/xterm.css");
const FIT_ADDON_JS: &str = include_str!("../assets/xterm/addon-fit.js");

const TERMINAL_BOOTSTRAP: &str = r#"
<script>
(() => {
  const host = document.getElementById('terminal');
  const terminal = new Terminal({
    allowProposedApi: false,
    convertEol: false,
    cursorBlink: true,
    cursorStyle: 'block',
    fontFamily: 'SFMono-Regular, SF Mono, Menlo, Monaco, PingFang SC, Hiragino Sans GB, Microsoft YaHei UI, Noto Sans Mono CJK SC, monospace',
    fontSize: 13,
    fontWeight: '400',
    letterSpacing: 0,
    lineHeight: 1.2,
    scrollback: 5000,
    theme: {
      background: '#0e0f13',
      foreground: '#d8dbe5',
      cursor: '#d8dbe5',
      selectionBackground: '#5865a866',
      black: '#111218',
      red: '#ef7373',
      green: '#75d89b',
      yellow: '#e7c66b',
      blue: '#78a9ff',
      magenta: '#c792ea',
      cyan: '#70d7da',
      white: '#d8dbe5'
    }
  });
  const fitAddon = new FitAddon.FitAddon();
  const decoder = new TextDecoder('utf-8');
  terminal.loadAddon(fitAddon);
  terminal.open(host);

  const post = payload => window.ipc.postMessage(JSON.stringify(payload));
  terminal.onData(data => post({ type: 'input', data }));
  terminal.onResize(({ cols, rows }) => post({ type: 'resize', cols, rows }));

  const fit = () => {
    try { fitAddon.fit(); } catch (_) {}
  };
  const resizeObserver = new ResizeObserver(() => requestAnimationFrame(fit));
  resizeObserver.observe(host);

  window.gridvanaTerminal = {
    writeBase64(encoded) {
      const binary = atob(encoded);
      const bytes = new Uint8Array(binary.length);
      for (let index = 0; index < binary.length; index += 1) {
        bytes[index] = binary.charCodeAt(index);
      }
      terminal.write(decoder.decode(bytes, { stream: true }));
    },
    clear() {
      terminal.reset();
      terminal.clear();
    },
    focus() {
      terminal.focus();
    },
    fit
  };

  requestAnimationFrame(() => {
    fit();
    terminal.focus();
    post({ type: 'ready', cols: terminal.cols, rows: terminal.rows });
  });
})();
</script>
"#;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WebTerminalEvent {
    Ready { cols: u16, rows: u16 },
    Input { data: String },
    Resize { cols: u16, rows: u16 },
}

pub fn webview_config() -> iced_wry::WebViewConfig {
    iced_wry::WebViewConfig::default()
        .html(terminal_html())
        .devtools(cfg!(debug_assertions))
}

fn terminal_html() -> String {
    let mut html = String::from(
        "<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><style>",
    );
    html.push_str(XTERM_CSS);
    html.push_str(
        "html,body,#terminal{width:100%;height:100%;margin:0;background:#0e0f13;overflow:hidden}body{box-sizing:border-box;padding:8px}#terminal{box-sizing:border-box}.xterm{height:100%}.xterm-viewport{overflow-y:auto!important}",
    );
    html.push_str("</style></head><body><div id=\"terminal\"></div><script>");
    html.push_str(XTERM_JS);
    html.push_str("</script><script>");
    html.push_str(FIT_ADDON_JS);
    html.push_str("</script>");
    html.push_str(TERMINAL_BOOTSTRAP);
    html.push_str("</body></html>");
    html
}

pub fn parse_ipc(body: &str) -> Result<WebTerminalEvent, String> {
    serde_json::from_str(body).map_err(|error| format!("终端 IPC 消息无效：{error}"))
}

pub fn output_script(bytes: &[u8]) -> String {
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    let argument = serde_json::to_string(&encoded).expect("base64 should serialize");
    format!("window.gridvanaTerminal?.writeBase64({argument});")
}

pub const CLEAR_SCRIPT: &str = "window.gridvanaTerminal?.clear();";
pub const FOCUS_SCRIPT: &str = "window.gridvanaTerminal?.fit();window.gridvanaTerminal?.focus();";

#[cfg(test)]
mod tests {
    use super::{WebTerminalEvent, output_script, parse_ipc, terminal_html};

    #[test]
    fn parses_unicode_terminal_input() {
        assert_eq!(
            parse_ipc(r#"{"type":"input","data":"画一个红色小球"}"#).unwrap(),
            WebTerminalEvent::Input {
                data: "画一个红色小球".to_string()
            }
        );
    }

    #[test]
    fn output_script_preserves_utf8_bytes_as_base64() {
        let script = output_script("中文".as_bytes());
        assert!(script.contains("5Lit5paH"));
    }

    #[test]
    fn embeds_xterm_with_cjk_font_fallbacks() {
        let html = terminal_html();
        assert!(html.contains("new Terminal"));
        assert!(html.contains("PingFang SC"));
        assert!(html.contains("TextDecoder('utf-8')"));
        assert!(html.contains("body{box-sizing:border-box;padding:8px}"));
    }
}
