use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub logo: LogoCfg,
    pub display: DisplayCfg,
    pub modules: Vec<ModuleEntry>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            logo: LogoCfg::default(),
            display: DisplayCfg::default(),
            modules: vec![],
        }
    }
}

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct LogoCfg {
    /// auto | off | имя логотипа
    #[serde(rename = "type")]
    pub kind: String,
    /// форсированный логотип
    pub source: Option<String>,
}

impl Default for LogoCfg {
    fn default() -> Self {
        Self { kind: "auto".into(), source: None }
    }
}

#[derive(Deserialize, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct DisplayCfg {
    pub separator: String,
    pub color: Option<String>,
    pub palette: bool,
    pub no_color: bool,
    pub neon: bool,
    pub frame: bool,
}

impl Default for DisplayCfg {
    fn default() -> Self {
        Self {
            separator: ":".into(),
            color: None,
            palette: true,
            no_color: false,
            neon: false,
            frame: true,
        }
    }
}

/// Модуль — строка "cpu" или объект { "type": "cpu", "label": "Проц" }
#[derive(Deserialize, Clone)]
#[serde(untagged)]
pub enum ModuleEntry {
    Name(String),
    Full(ModuleSpec),
}

#[derive(Deserialize, Clone, Default)]
pub struct ModuleSpec {
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub mount: Option<String>,
}

impl ModuleEntry {
    pub fn spec(&self) -> ModuleSpec {
        match self {
            ModuleEntry::Name(s) => ModuleSpec { kind: s.clone(), label: None, mount: None },
            ModuleEntry::Full(m) => m.clone(),
        }
    }
}

pub const DEFAULT_MODULES: &[&str] = &[
    "title", "separator", "os", "host", "kernel", "uptime", "packages",
    "shell", "de", "wm", "terminal", "resolution", "locale", "ip",
    "cpu", "gpu", "memory", "swap", "disk", "battery",
];

pub const SAMPLE: &str = r#"{
    // Логотип: type = auto | off | имя (arch, debian, ubuntu, ...)
    "logo": {
        "type": "auto"
        // "source": "arch"
    },

    "display": {
        "separator": ":",
        "palette": true,
        "neon": false,
        "frame": true
        // "color": "magenta",
        // "noColor": true
    },

    // Порядок в списке = порядок вывода.
    // Строки или объекты с кастомным label/mount.
    "modules": [
        "title",
        "separator",
        "os",
        "host",
        "kernel",
        "uptime",
        "packages",
        "shell",
        "de",
        "wm",
        "terminal",
        "resolution",
        "locale",
        "ip",
        "cpu",
        "gpu",
        "memory",
        "swap",
        "disk",
        "battery"
        // { "type": "disk", "mount": "/home", "label": "Home" },
        // { "type": "cpu", "label": "Проц" }
    ]
}"#;

pub fn default_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("rustfetch").join("config.json"))
}

pub fn load(path: Option<&Path>) -> Config {
    let path = match path {
        Some(p) => p.to_path_buf(),
        None => match default_path() {
            Some(p) => p,
            None => return Config::default(),
        },
    };

    let raw = match std::fs::read_to_string(&path) {
        Ok(r) => r,
        Err(_) => return Config::default(),
    };

    // JSONC: режем комментарии и trailing commas, как в fastfetch
    let cleaned = strip_trailing_commas(&strip_comments(&raw));
    match serde_json::from_str::<Config>(&cleaned) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("rustfetch: bad config {path:?}: {e}");
            Config::default()
        }
    }
}

fn strip_comments(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut in_string = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if in_string {
            out.push(c);
            if c == '\\' && i + 1 < chars.len() {
                out.push(chars[i + 1]);
                i += 2;
                continue;
            }
            if c == '"' { in_string = false; }
            i += 1;
            continue;
        }
        if c == '"' { in_string = true; out.push(c); i += 1; continue; }
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            while i < chars.len() && chars[i] != '\n' { i += 1; }
            continue;
        }
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') { i += 1; }
            i += 2;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

fn strip_trailing_commas(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut in_string = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if in_string {
            out.push(c);
            if c == '\\' && i + 1 < chars.len() {
                out.push(chars[i + 1]);
                i += 2;
                continue;
            }
            if c == '"' { in_string = false; }
            i += 1;
            continue;
        }
        match c {
            '"' => { in_string = true; out.push(c); i += 1; }
            ',' => {
                let mut j = i + 1;
                while j < chars.len() && chars[j].is_whitespace() { j += 1; }
                if !(j < chars.len() && (chars[j] == ']' || chars[j] == '}')) {
                    out.push(c);
                }
                i += 1;
            }
            _ => { out.push(c); i += 1; }
        }
    }
    out
}
