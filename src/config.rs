use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;

use crate::color::{HexColor, Palette};
use crate::term::UserEvent;

#[derive(Deserialize)]
#[serde(default)]
pub struct Theme {
    pub palette: Palette,
    pub background: HexColor,
    pub foreground: HexColor,
    pub cursor: HexColor,
    pub dim_strength: f32,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            palette: Palette::default(),
            background: HexColor([0x28, 0x28, 0x28]),
            foreground: HexColor([0xeb, 0xdb, 0xb2]),
            cursor: HexColor([0xeb, 0xdb, 0xb2]),
            dim_strength: 0.66,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
pub struct Font {
    pub size: f64,
    pub family: String,
    pub fallback: Vec<String>,
    pub gamma: f32,
    pub contrast: f32,
    pub bold_is_bright: bool,
}

impl Default for Font {
    fn default() -> Self {
        Self {
            size: 14.0,
            family: "Menlo".into(),
            fallback: default_fallback(),
            gamma: 1.7,
            contrast: 30.0,
            bold_is_bright: false,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
pub struct Cursor {
    pub hollow_cursor_thickness: f32,
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            hollow_cursor_thickness: 0.1,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
pub struct Shell {
    pub locale: Option<String>,
}

impl Default for Shell {
    fn default() -> Self {
        Self { locale: None }
    }
}

#[derive(Deserialize)]
#[serde(default)]
pub struct Config {
    pub theme: Theme,
    pub font: Font,
    pub cursor: Cursor,
    pub shell: Shell,
    pub lines_per_notch: f64,
    pub multi_click_window_ms: u64,
    pub autoscroll_interval_ms: u64,
    pub autoscroll_speed: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            font: Font::default(),
            cursor: Cursor::default(),
            shell: Shell::default(),
            lines_per_notch: 3.0,
            multi_click_window_ms: 400,
            autoscroll_interval_ms: 25,
            autoscroll_speed: 1.0,
        }
    }
}

impl Config {
    fn path() -> Option<PathBuf> {
        let home = std::env::var_os("HOME")?;
        Some(Path::new(&home).join(".config/mediocritty/config.toml"))
    }

    fn read(path: &Path) -> Option<Self> {
        let text = std::fs::read_to_string(path).ok()?;

        match toml::from_str(&text) {
            Ok(cfg) => Some(cfg),
            Err(e) => {
                eprintln!("config parse error: {e}, using defaults");
                None
            }
        }
    }

    pub fn load() -> Self {
        Self::path()
            .and_then(|path| Self::read(&path))
            .unwrap_or_default()
    }

    pub fn watch(proxy: winit::event_loop::EventLoopProxy<UserEvent>) {
        let Some(path) = Self::path() else {
            return;
        };
        let Some(dir) = path.parent().map(Path::to_path_buf) else {
            return;
        };

        std::thread::spawn(move || {
            use notify::{RecursiveMode, Watcher};
            use std::sync::mpsc::channel;

            let (tx, rx) = channel();
            let mut watcher = match notify::recommended_watcher(move |res| {
                if let Ok(event) = res {
                    let _ = tx.send(event);
                }
            }) {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("config watch: {e}");
                    return;
                }
            };

            if let Err(e) = watcher.watch(&dir, RecursiveMode::NonRecursive) {
                eprintln!("config watch: {e}");
                return;
            }

            while let Ok(event) = rx.recv() {
                if !event.paths.iter().any(|p| p == &path) {
                    continue;
                }

                while rx.recv_timeout(Duration::from_millis(100)).is_ok() {}

                if let Some(cfg) = Self::read(&path) {
                    let _ = proxy.send_event(UserEvent::ConfigReload(cfg));
                }
            }
        });
    }
}

#[cfg(target_os = "macos")]
fn default_fallback() -> Vec<String> {
    vec![
        "Apple Color Emoji".into(),
        "Apple Symbols".into(),
        "Hiragino Sans".into(),
    ]
}

#[cfg(not(target_os = "macos"))]
fn default_fallback() -> Vec<String> {
    Vec::new()
}
