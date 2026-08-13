use crate::system;

pub const KNOWN: &[&str] = &[
    "arch", "debian", "ubuntu", "fedora", "manjaro", "alpine",
    "void", "gentoo", "nixos", "mint", "opensuse", "linux",
];

pub struct Logo {
    pub lines: Vec<String>,
    pub color: String,
}

fn make(art: &'static str, color: &'static str) -> Logo {
    Logo {
        lines: art.lines().map(str::to_string).collect(),
        color: color.to_string(),
    }
}

pub fn color_code(name: &str) -> String {
    let c = match name {
        "red" => "31",
        "green" => "32",
        "yellow" => "33",
        "blue" => "34",
        "magenta" => "35",
        "cyan" => "36",
        "white" => "37",
        _ => "36",
    };
    format!("\x1b[1;{c}m")
}

fn logos_dir() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|d| d.join("dirtfetch").join("logos"))
}

fn load_custom(id: &str) -> Option<Logo> {
    let path = logos_dir()?.join(format!("{id}.txt"));
    let content = std::fs::read_to_string(&path).ok()?;
    let content = content.strip_prefix('\u{feff}').unwrap_or(&content);

    let mut accent = accent_for(normalize(id)).to_string();
    let mut palette: Vec<String> = Vec::new();
    let mut raws: Vec<String> = Vec::new();

    for raw in content.lines() {
        let line = clean_line(raw);
        if let Some(c) = line.strip_prefix("#color=") {
            accent = c.trim().to_string();
        } else if let Some(cs) = line.strip_prefix("#colors=") {
            palette = cs.split(',').map(|s| color_code(s.trim())).collect();
        } else {
            raws.push(line);
        }
    }

    if raws.is_empty() {
        return None;
    }
    if palette.is_empty() {
        palette = vec![color_code(&accent), color_code("white")];
    }

    let mut lines = Vec::new();
    for line in raws {
        let (out, had) = substitute_colors(&line, &palette);
        if had {
            lines.push(format!("{}{out}\x1b[0m", palette[0]));
        } else {
            lines.push(out);
        }
    }

    Some(Logo { lines, color: accent })
}

fn substitute_colors(line: &str, palette: &[String]) -> (String, bool) {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut had = false;
    let mut i = 0;

    while i < chars.len() {
        // ${cN}
        if chars[i] == '$' && i + 2 < chars.len() && chars[i + 1] == '{' && chars[i + 2] == 'c' {
            if let Some(close_rel) = line[i..].find('}') {
                let close = i + close_rel;
                let num: String = chars[i + 3..close]
                    .iter()
                    .filter(|c| c.is_ascii_digit())
                    .collect();
                if let Ok(n) = num.parse::<usize>() {
                    if n >= 1 {
                        out.push_str(pick(palette, n));
                        had = true;
                        i = close + 1;
                        continue;
                    }
                }
            }
        }
        // $N
        if chars[i] == '$' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
            let n = chars[i + 1].to_digit(10).unwrap() as usize;
            if n >= 1 {
                out.push_str(pick(palette, n));
                had = true;
                i += 2;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }

    (out, had)
}

fn pick(palette: &[String], n: usize) -> &str {
    palette
        .get((n - 1) % palette.len())
        .map(String::as_str)
        .unwrap_or("\x1b[0m")
}

/// Вырезаем управляющие символы и чужие ANSI-последовательности
fn clean_line(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\x1b' => {
                while let Some(&n) = chars.peek() {
                    chars.next();
                    if n == 'm' { break; }
                }
            }
            '\t' => out.push_str("    "),
            '\r' => {}
            c if (c as u32) < 0x20 => {}
            _ => out.push(c),
        }
    }
    out
}

pub fn strip_ansi(line: &str) -> String {
    clean_line(line)
}

pub fn custom_names() -> Vec<String> {
    let mut names = Vec::new();
    if let Some(dir) = logos_dir() {
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.filter_map(|e| e.ok()) {
                let p = e.path();
                if p.extension().and_then(|s| s.to_str()) == Some("txt") {
                    if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                        names.push(stem.to_string());
                    }
                }
            }
        }
    }
    names.sort();
    names
}

pub fn detect_id() -> String {
    system::os_release_field("ID").unwrap_or_else(|| "linux".into())
}

pub fn normalize(id: &str) -> &'static str {
    match id.to_lowercase().as_str() {
        "arch" | "endeavouros" | "artix" | "garuda" | "cachyos" | "parabola" => "arch",
        "manjaro" => "manjaro",
        "debian" | "raspbian" | "kali" | "pureos" => "debian",
        "ubuntu" | "pop" | "elementary" | "zorin" | "kubuntu" | "xubuntu" | "lubuntu" => "ubuntu",
        "linuxmint" | "mint" | "lmde" => "mint",
        "fedora" | "rhel" | "centos" | "rocky" | "alma" | "nobara" | "bazzite" => "fedora",
        "alpine" => "alpine",
        "void" => "void",
        "gentoo" | "funtoo" => "gentoo",
        "nixos" => "nixos",
        "opensuse" | "opensuse-tumbleweed" | "opensuse-leap" | "sles" => "opensuse",
        _ => "linux",
    }
}

pub fn get(id: &str) -> Logo {
    let id = id.strip_suffix(".txt").unwrap_or(id);

    let mut logo = if let Some(custom) = load_custom(id) {
        custom
    } else {
        match id {
            "arch" => make(ARCH, "cyan"),
            "debian" => make(DEBIAN, "red"),
            "ubuntu" => make(UBUNTU, "yellow"),
            "fedora" => make(FEDORA, "blue"),
            "manjaro" => make(MANJARO, "green"),
            "alpine" => make(ALPINE, "blue"),
            "void" => make(VOID, "green"),
            "gentoo" => make(GENTOO, "magenta"),
            "nixos" => make(NIXOS, "blue"),
            "mint" => make(MINT, "green"),
            "opensuse" => make(OPENSUSE, "green"),
            _ => make(LINUX, "yellow"),
        }
    };

    let code = color_code(&logo.color);
    logo.lines = logo
        .lines
        .into_iter()
        .map(|l| {
            if l.contains('\x1b') {
                l
            } else {
                format!("{code}{l}\x1b[0m")
            }
        })
        .collect();

    logo
}

fn accent_for(id: &str) -> &'static str {
    match id {
        "arch" => "cyan",
        "debian" => "red",
        "ubuntu" => "yellow",
        "fedora" => "blue",
        "manjaro" => "green",
        "alpine" => "blue",
        "void" => "green",
        "gentoo" => "magenta",
        "nixos" => "blue",
        "mint" => "green",
        "opensuse" => "green",
        _ => "yellow",
    }
}

pub fn visible_len(line: &str) -> usize {
    let mut len = 0;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            while let Some(&n) = chars.peek() {
                chars.next();
                if n == 'm' { break; }
            }
        } else {
            len += 1;
        }
    }
    len
}

const ARCH: &str = r#"                   -`
                  .o+`
                 `ooo/
                `+oooo:
               `+oooooo:
               -+oooooo+:
             `/:-:++oooo+:
            `/++++/+++++++:
           `/++++++++++++++:
          `/+++ooooooooooooo/`
         ./ooosssso++osssssso+`
        .oossssso-````/ossssss+`
       -osssssso.      :ssssssso.
      :osssssss/        osssso+++.
     /ossssssss/        +ssssooo/-
   `/ossssso+/:-        -:/+osssso+-
  `+sso+:-`                 `.-/+oso:
 `++:.                           `-/+/
 .`                                 `/"#;

const DEBIAN: &str = r#"       _,met$$$$$gg.
    ,g$$$$$$$$$$$$$$$P.
  ,g$$P"        """Y$$.".
 ,$$P'              `$$$.
',$$P       ,ggs.     `$$b:
`d$$'     ,$P"'   .    $$$
 $$P      d$'     ,    $$P
 $$:      $$.   -    ,d$$'
 $$;      Y$b._   _,d$P'
 Y$$.    `.`"Y$$$$P"'
 `$$b      "-.__
  `Y$$
   `Y$$.
     `$$b.
       `Y$$b.
          `"Y$b._
              `""""#;

const UBUNTU: &str = r#"         _
     ---(_)
 _/  ---  \
(_) | |
 \  --- _/
     ---(_)"#;

const FEDORA: &str = r#"           .:lodxkkkxkdl:.
      ;dxxxdol:;;;:odxxxds;
    :xxxdl.         .lxxxd:
   :xxxd.             .dxxx:
  :xxxd.   .;;;;;;;.   .dxxx:
  :xxxd.   :ddddddd:   .dxxx:
   :xxxd.  :ddddddd:  .dxxx:
    :xxxdl.         .lxxxd:
      ;dxxxdl:;;;:odxxxd;
           .:lodxkkkdl:."#;

const MANJARO: &str = r#"||||||||||||||||||||||||
||||||||||||||||||||||||
||||||||||||||||||||||||
|||||||  |||||||||||||||
|||||||  |||||||||||||||
|||||||  |||||||||||||||
|||||||  |||||  ||||||||
|||||||  |||||  ||||||||
|||||||  |||||  ||||||||"#;

const ALPINE: &str = r#"   /\
  /  \
 /\   \
|  \   \
 \   \   \
  \   \   \
   \   \   \
    \   \   \
     \   \   \
      /     \"#;

const VOID: &str = r#"    __________
   /          \
  /   ______   \
 |   |      |   |
 |   |      |   |
  \   ______   /
   \__________/"#;

const GENTOO: &str = r#"   _-----_
  (       \
   \    O  \
   /        \
  |          \
  |           |
   \           \
    \______     \
           \_____\"#;

const NIXOS: &str = r#"  \  \|/  /
   \  *  /
    \ | /
 ----*----
    / | \
   /  *  \
  /  /|\  \"#;

const MINT: &str = r#"  ____________
 |            |
 |  __   __   |
 | |  | |  |  |
 | |  | |  |  |
 | |__| |__|  |
 |____________|"#;

const OPENSUSE: &str = r#"    _______
   /       \
  | ()   () |
   \  ===  /
    \_____/"#;

const LINUX: &str = r#"      .---.
     / o o \
    |   ^   |
    |  \_/  |
     \_____/
   /|     |\
  (_|     |_)"#;
