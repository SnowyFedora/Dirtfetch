mod config;
mod logos;
mod system;

use std::path::PathBuf;

use clap::Parser;

use crate::config::{Config, ModuleSpec};

/// Прайд-флаг
const PRIDE: [(u8, u8, u8); 6] = [
    (228, 3, 3),
    (255, 140, 0),
    (255, 237, 0),
    (0, 128, 60),
    (0, 77, 255),
    (117, 7, 135),
];

/// Флаг натуралов: чёрно-белые полосы
/// (чёрный рендерим серым — на тёмном терминале его иначе не видно)
const TRAD: [(u8, u8, u8); 6] = [
    (255, 255, 255),
    (128, 128, 128),
    (255, 255, 255),
    (128, 128, 128),
    (255, 255, 255),
    (128, 128, 128),
];

#[derive(Parser)]
#[command(name = "dirtfetch", version, about = "fastfetch-style system info, in Rust")]
struct Cli {
    /// Форсировать логотип: dirtfetch --logo debian
    #[arg(short, long, value_name = "NAME")]
    logo: Option<String>,

    /// Не печатать логотип
    #[arg(long)]
    no_logo: bool,

    /// Отключить цвета
    #[arg(long)]
    no_color: bool,

    /// Свой путь до config.json
    #[arg(short, long, value_name = "PATH")]
    config: Option<PathBuf>,

    /// Скрыть модули: --hide swap,gpu,battery
    #[arg(long, value_delimiter = ',')]
    hide: Vec<String>,

    /// Список доступных логотипов
    #[arg(long)]
    list_logos: bool,

    /// Сгенерировать пример config.json
    #[arg(long)]
    gen_config: bool,

    /// Неоновый градиент логотипа cyan -> magenta
    #[arg(long)]
    neon: bool,

    /// Окрасить логотип в цвета прайд-флага
    #[arg(long)]
    homo: bool,

    /// Окрасить логотип в цвета флага натуралов (традиционные ценности)
    #[arg(long)]
    trad: bool,

    /// Отключить неоновую рамку вокруг вывода
    #[arg(long)]
    no_frame: bool,
}

/// Чтобы работало в стиле одного дефиса: -help, -homo, -trad, -neon ...
fn normalize_args() -> Vec<String> {
    std::env::args()
        .enumerate()
        .map(|(i, a)| {
            if i > 0
                && a.len() > 2
                && a.starts_with('-')
                && !a.starts_with("--")
                && a.chars().nth(1).map(|c| c.is_ascii_alphabetic()).unwrap_or(false)
            {
                format!("-{a}")
            } else {
                a
            }
        })
        .collect()
}

struct Theme {
    enabled: bool,
    accent: String,
}

impl Theme {
    fn code(name: &str) -> &'static str {
        match name {
            "red" => "31",
            "green" => "32",
            "yellow" => "33",
            "blue" => "34",
            "magenta" => "35",
            "cyan" => "36",
            "white" => "37",
            _ => "36",
        }
    }

    fn paint(&self, text: &str, code: &str) -> String {
        if self.enabled {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    fn label(&self, text: &str) -> String {
        self.paint(text, &format!("1;{}", self.accent))
    }
}

fn module_value(kind: &str, info: &system::Info) -> Option<(&'static str, String)> {
    Some(match kind {
        "os" => ("OS", info.os.clone()),
        "host" => ("Host", info.host.clone()?),
        "kernel" => ("Kernel", info.kernel.clone()?),
        "uptime" => ("Uptime", info.uptime.clone()?),
        "packages" => ("Packages", info.packages.clone()?),
        "shell" => ("Shell", info.shell.clone()?),
        "de" => ("DE", info.de.clone()?),
        "wm" => ("WM", info.wm.clone()?),
        "terminal" => ("Terminal", info.terminal.clone()?),
        "resolution" => ("Resolution", info.resolution.clone()?),
        "locale" => ("Locale", info.locale.clone()?),
        "ip" => ("Local IP", info.local_ip.clone()?),
        "cpu" => ("CPU", info.cpu.clone()?),
        "gpu" => ("GPU", info.gpu.clone()?),
        "memory" => ("Memory", info.memory.clone()),
        "swap" => ("Swap", info.swap.clone()?),
        "battery" => ("Battery", info.battery.clone()?),
        _ => return None,
    })
}

/// Неоновая рамка с вертикальным градиентом вокруг всего вывода
fn framed(rows: Vec<String>, color: bool) -> Vec<String> {
    let inner = rows.iter().map(|r| logos::visible_len(r)).max().unwrap_or(0) + 2;
    let total = rows.len() + 2;

    let border = |ch: &str, i: usize| -> String {
        if !color {
            return ch.to_string();
        }
        let t = i as f64 / (total - 1).max(1) as f64;
        let r = (255.0 * t) as u8;
        let g = (255.0 * (1.0 - t)) as u8;
        format!("\x1b[38;2;{r};{g};255m{ch}\x1b[0m")
    };

    let mut out = Vec::with_capacity(total);
    let hbar = "─".repeat(inner);
    out.push(format!("{}{hbar}{}", border("╭", 0), border("╮", 0)));
    for (idx, r) in rows.into_iter().enumerate() {
        let i = idx + 1;
        let pad = " ".repeat((inner - 2).saturating_sub(logos::visible_len(&r)));
        out.push(format!("{} {r}{pad} {}", border("│", i), border("│", i)));
    }
    out.push(format!("{}{hbar}{}", border("╰", total - 1), border("╯", total - 1)));
    out
}

fn main() {
    let cli = Cli::parse_from(normalize_args());

    if cli.list_logos {
        let mut all: Vec<String> = logos::KNOWN.iter().map(|s| s.to_string()).collect();
        all.extend(logos::custom_names());
        all.sort();
        all.dedup();
        println!("available logos: {}", all.join(", "));
        return;
    }

    if cli.gen_config {
        let path = cli
            .config
            .clone()
            .or_else(config::default_path)
            .expect("не удалось определить директорию конфига");
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match std::fs::write(&path, config::SAMPLE) {
            Ok(_) => println!("config written: {}", path.display()),
            Err(e) => eprintln!("dirtfetch: не удалось записать конфиг: {e}"),
        }
        return;
    }

    let cfg = config::load(cli.config.as_deref());
    let color = !cli.no_color
        && !cfg.display.no_color
        && std::env::var_os("NO_COLOR").is_none();

    let info = system::collect();

    // ─── логотип ────────────────────────────────────────────────────
    let logo_kind = cfg.logo.kind.as_str();
    let wanted = cli
        .logo
        .clone()
        .or_else(|| cfg.logo.source.clone())
        .or_else(|| match logo_kind {
            "auto" | "off" => None,
            other => Some(other.to_string()),
        });
    let logo = if cli.no_logo || logo_kind == "off" {
        None
    } else {
        let id = wanted.unwrap_or_else(|| logos::detect_id());
        Some(logos::get(&id))
    };

    let accent_name = cfg
        .display
        .color
        .clone()
        .or_else(|| logo.as_ref().map(|l| l.color.clone()))
        .unwrap_or_else(|| "cyan".into());
    let theme = Theme { enabled: color, accent: Theme::code(&accent_name).to_string() };

    // ─── правая колонка ─────────────────────────────────────────────
    let mut right: Vec<String> = Vec::new();

    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "user".into());
    let title = format!("{user}@{}", info.hostname);

    let specs: Vec<ModuleSpec> = if cfg.modules.is_empty() {
        config::DEFAULT_MODULES
            .iter()
            .map(|n| ModuleSpec { kind: n.to_string(), label: None, mount: None })
            .collect()
    } else {
        cfg.modules.iter().map(|e| e.spec()).collect()
    };

    let sep = &cfg.display.separator;

    for spec in &specs {
        if cli.hide.iter().any(|h| h == &spec.kind) {
            continue;
        }
        match spec.kind.as_str() {
            "title" => right.push(theme.label(&title)),
            "separator" => right.push(theme.label(&"─".repeat(title.chars().count()))),
            "disk" => {
                let base = spec.label.clone().unwrap_or_else(|| "Disk".into());
                if let Some(m) = &spec.mount {
                    if let Some((_, text)) = info.disks.iter().find(|(mp, _)| mp == m) {
                        let l = theme.label(&format!("{base} ({m}){sep}"));
                        right.push(format!("{l} {text}"));
                    }
                } else {
                    for (mp, text) in &info.disks {
                        let l = theme.label(&format!("{base} ({mp}){sep}"));
                        right.push(format!("{l} {text}"));
                    }
                }
            }
            other => {
                if let Some((def_label, value)) = module_value(other, &info) {
                    let label = spec.label.clone().unwrap_or_else(|| def_label.to_string());
                    let l = theme.label(&format!("{label}{sep}"));
                    right.push(format!("{l} {value}"));
                }
            }
        }
    }

    if cfg.display.palette {
        let mut pal = String::new();
        for c in 0..8 {
            if color {
                pal.push_str(&format!("\x1b[{}m   \x1b[0m", 40 + c));
            } else {
                pal.push_str("[ ] ");
            }
        }
        right.push(pal);
    }

    // ─── строки вывода ──────────────────────────────────────────────
    let mut logo_lines: Vec<String> = logo.map(|l| l.lines.clone()).unwrap_or_default();

    // неоновый градиент cyan -> magenta
    if (cli.neon || cfg.display.neon) && color && !cli.homo && !cli.trad {
        let n = logo_lines.len();
        for (i, l) in logo_lines.iter_mut().enumerate() {
            let t = if n > 1 { i as f64 / (n - 1) as f64 } else { 0.0 };
            let r = (255.0 * t) as u8;
            let g = (255.0 * (1.0 - t)) as u8;
            let clean = logos::strip_ansi(l);
            *l = format!("\x1b[1;38;2;{r};{g};255m{clean}\x1b[0m");
        }
    }

    // прайд
    if cli.homo && color && !cli.trad {
        let n = logo_lines.len();
        for (i, l) in logo_lines.iter_mut().enumerate() {
            let (r, g, b) = PRIDE[if n > 0 { i * PRIDE.len() / n } else { 0 }];
            let clean = logos::strip_ansi(l);
            *l = format!("\x1b[1;38;2;{r};{g};{b}m{clean}\x1b[0m");
        }
    }

    // традиционные ценности
    if cli.trad && color {
        let n = logo_lines.len();
        for (i, l) in logo_lines.iter_mut().enumerate() {
            let (r, g, b) = TRAD[if n > 0 { i * TRAD.len() / n } else { 0 }];
            let clean = logos::strip_ansi(l);
            *l = format!("\x1b[1;38;2;{r};{g};{b}m{clean}\x1b[0m");
        }
    }

    let width = logo_lines.iter().map(|l| logos::visible_len(l)).max().unwrap_or(0) + 3;

    let mut rows: Vec<String> = Vec::new();
    let n = logo_lines.len().max(right.len());
    for i in 0..n {
        let left = logo_lines.get(i).map(String::as_str).unwrap_or("");
        let r = right.get(i).map(String::as_str).unwrap_or("");
        if logo_lines.is_empty() {
            rows.push(r.to_string());
        } else {
            let pad = width.saturating_sub(logos::visible_len(left));
            rows.push(format!("{left}{}{r}", " ".repeat(pad)));
        }
    }

    println!();
    if !cli.no_frame && cfg.display.frame {
        for l in framed(rows, color) {
            println!("{l}");
        }
    } else {
        for l in rows {
            println!("{l}");
        }
    }
    println!();
}
