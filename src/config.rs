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
    pub opacity: f32,
    pub blur: u32,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            palette: Palette::default(),
            background: HexColor([0x28, 0x28, 0x28]),
            foreground: HexColor([0xeb, 0xdb, 0xb2]),
            cursor: HexColor([0xeb, 0xdb, 0xb2]),
            dim_strength: 0.66,
            opacity: 1.0,
            blur: 0,
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
#[derive(Default)]
pub struct Shell {
    pub locale: Option<String>,
}

// who may touch the clipboard through OSC 52. copy is the alacritty default:
// programs (ssh sessions included) can set it but never read it back
#[derive(Deserialize, Clone, Copy, Default)]
#[serde(rename_all = "lowercase")]
pub enum Osc52 {
    Disabled,
    #[default]
    Copy,
    Paste,
    CopyPaste,
}

impl From<Osc52> for alacritty_terminal::term::Osc52 {
    fn from(v: Osc52) -> Self {
        use alacritty_terminal::term::Osc52 as T;

        match v {
            Osc52::Disabled => T::Disabled,
            Osc52::Copy => T::OnlyCopy,
            Osc52::Paste => T::OnlyPaste,
            Osc52::CopyPaste => T::CopyPaste,
        }
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
    pub osc52: Osc52,
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
            osc52: Osc52::default(),
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

        match toml::from_str::<Self>(&text) {
            Ok(cfg) => Some(cfg.sanitized()),
            Err(e) => {
                eprintln!("config parse error: {e}, using defaults");
                None
            }
        }
    }

    // anything out of range falls back to the default instead of reaching
    // code that divides by it or hands it to coretext
    fn sanitized(mut self) -> Self {
        let d = Self::default();

        fn finite_in<T: PartialOrd + Copy>(v: T, lo: T, hi: T, def: T, ok: bool) -> T {
            if ok && v >= lo && v <= hi { v } else { def }
        }

        let f = &mut self.font;
        f.size = finite_in(f.size, 1.0, 512.0, d.font.size, f.size.is_finite());
        f.gamma = finite_in(f.gamma, 0.01, 10.0, d.font.gamma, f.gamma.is_finite());
        f.contrast = finite_in(
            f.contrast,
            0.0,
            100.0,
            d.font.contrast,
            f.contrast.is_finite(),
        );

        let t = &mut self.theme;
        t.opacity = finite_in(t.opacity, 0.0, 1.0, d.theme.opacity, t.opacity.is_finite());
        t.dim_strength = finite_in(
            t.dim_strength,
            0.0,
            1.0,
            d.theme.dim_strength,
            t.dim_strength.is_finite(),
        );
        t.blur = t.blur.min(100);

        let c = &mut self.cursor;
        c.hollow_cursor_thickness = finite_in(
            c.hollow_cursor_thickness,
            0.01,
            0.5,
            d.cursor.hollow_cursor_thickness,
            c.hollow_cursor_thickness.is_finite(),
        );

        self.lines_per_notch = finite_in(
            self.lines_per_notch,
            0.01,
            100.0,
            d.lines_per_notch,
            self.lines_per_notch.is_finite(),
        );
        self.autoscroll_speed = finite_in(
            self.autoscroll_speed,
            0.01,
            100.0,
            d.autoscroll_speed,
            self.autoscroll_speed.is_finite(),
        );
        self.autoscroll_interval_ms = self.autoscroll_interval_ms.clamp(1, 1000);

        self
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

        // stow and friends leave a symlink here, and fsevents only ever reports
        // the real location, so watch that instead
        let real = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());

        let Some(dir) = real.parent().map(Path::to_path_buf) else {
            return;
        };
        let Some(name) = real.file_name().map(std::ffi::OsString::from) else {
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
                if !event
                    .paths
                    .iter()
                    .any(|p| p.file_name() == Some(name.as_os_str()))
                {
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
