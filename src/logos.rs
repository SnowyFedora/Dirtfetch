use std::path::{Path, PathBuf};

use include_dir::{include_dir, Dir};

use crate::system;

/// Вся папка logos/ из репы вшита в бинарник
static BUNDLED: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/logos");

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

/// Полный ANSI с обёрткой по имени цвета: "\x1b[1;31m"
pub fn color_code(name: &str) -> String {
    format!("\x1b[{}m", accent_code(name))
}

/// Код без обёртки: "1;31" или "1;38;2;r;g;b" — понимает имена И hex
pub fn accent_code(color: &str) -> String {
    if let Some(hex) = color.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
            return format!("1;38;2;{r};{g};{b}");
        }
    }
    let c = match color {
        "red" => "31",
        "green" => "32",
        "yellow" => "33",
        "blue" => "34",
        "magenta" => "35",
        "cyan" => "36",
        "white" => "37",
        "black" => "30",
        _ => "36",
    };
    format!("1;{c}")
}

fn logos_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("dirtfetch").join("logos"))
}

/// Распаковать встроенные лого в конфиг (только недостающие)
pub fn install_bundled() -> usize {
    let root = match logos_dir() {
        Some(r) => r,
        None => return 0,
    };
    let _ = std::fs::create_dir_all(&root);
    let mut count = 0;
    extract_dir(&BUNDLED, &root, &mut count);
    count
}

fn extract_dir(dir: &Dir, dst: &Path, count: &mut usize) {
    for f in dir.files() {
        let name = match f.path().file_name() {
            Some(n) => n,
            None => continue,
        };
        let p = dst.join(name);
        if !p.exists() {
            if std::fs::write(&p, f.contents()).is_ok() {
                *count += 1;
            }
        }
    }
    for d in dir.dirs() {
        let name = match d.path().file_name() {
            Some(n) => n,
            None => continue,
        };
        let nd = dst.join(name);
        let _ = std::fs::create_dir_all(&nd);
        extract_dir(d, &nd, count);
    }
}

/// Ищет <id>.txt прямо в logos/ или рекурсивно в подпапках (logos/a/arch.txt)
fn find_logo_file(id: &str) -> Option<PathBuf> {
    let root = logos_dir()?;
    let target = format!("{id}.txt");
    let direct = root.join(&target);
    if direct.is_file() {
        return Some(direct);
    }

    fn walk(dir: &Path, target: &str) -> Option<PathBuf> {
        let rd = std::fs::read_dir(dir).ok()?;
        for e in rd.filter_map(|e| e.ok()) {
            let p = e.path();
            if p.is_dir() {
                if let Some(f) = walk(&p, target) {
                    return Some(f);
                }
            } else if p.file_name().and_then(|s| s.to_str()) == Some(target) {
                return Some(p);
            }
        }
        None
    }

    walk(&root, &target)
}

fn load_custom(id: &str) -> Option<Logo> {
    let path = find_logo_file(id)?;
    let content = std::fs::read_to_string(&path).ok()?;
    let content = content.strip_prefix('\u{feff}').unwrap_or(&content);

    let mut accent = accent_for(id).to_string();
    let mut palette: Vec<String> = Vec::new();
    let mut raws: Vec<String> = Vec::new();

    for raw in content.lines() {
        let line = clean_line(raw);
        if let Some(c) = line.strip_prefix("#color=") {
            accent = c.trim().to_string();
        } else if let Some(cs) = line.strip_prefix("#colors=") {
            palette = cs
                .split(',')
                .map(|s| format!("\x1b[{}m", accent_code(s.trim())))
                .collect();
        } else {
            raws.push(line);
        }
    }

    if raws.is_empty() {
        return None;
    }
    if palette.is_empty() {
        palette = vec![
            format!("\x1b[{}m", accent_code(&accent)),
            format!("\x1b[{}m", accent_code("white")),
        ];
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
    if let Some(root) = logos_dir() {
        fn walk(dir: &Path, names: &mut Vec<String>) {
            let rd = match std::fs::read_dir(dir) {
                Ok(r) => r,
                Err(_) => return,
            };
            for e in rd.filter_map(|e| e.ok()) {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, names);
                } else if p.extension().and_then(|s| s.to_str()) == Some("txt") {
                    if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                        names.push(stem.to_string());
                    }
                }
            }
        }
        walk(&root, &mut names);
    }
    names.sort();
    names.dedup();
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

    // один цвет на весь логотип (имя или hex — accent_code поймёт)
    let code = format!("\x1b[{}m", accent_code(&logo.color));
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

/// Цвет дистра: точная таблица -> семейные правила -> хэш по имени
fn accent_for(id: &str) -> &'static str {
    let low = id.to_lowercase();

    let exact = match low.as_str() {
        "alpine" | "alpine_small" | "alpine2" | "alpine2_small" | "alpine3_small" => "blue",
        "arch" | "arch2" | "arch_old" | "arch_small" | "archbox" => "cyan",
        "archcraft" | "archcraft2" => "yellow",
        "archlabs" => "blue",
        "archstrike" => "red",
        "arco" | "arco_small" => "blue",
        "artix" | "artix2" | "artix_small" | "artix2_small" => "cyan",
        "bedrock" | "bedrock_small" => "white",
        "cachyos" | "cachyos_small" | "cachyos_old_small" => "cyan",
        "calculate" => "blue",
        "centos" | "centos_small" => "magenta",
        "crux" | "crux_small" => "blue",
        "debian" | "debian_small" => "red",
        "devuan" | "devuan_small" => "magenta",
        "droidian" => "green",
        "elementary" | "elementary_small" => "white",
        "endeavouros" | "endeavouros_small" => "magenta",
        "fedora" | "fedora_small" | "fedora2_small" | "fedora_old"
        | "fedora_coreos" | "fedora_kinoite" | "fedora_sericea" | "fedora_silverblue" => "blue",
        "femboyos" | "yiffos" => "magenta",
        "freebsd" | "freebsd_small" => "red",
        "garuda" | "garuda_dragon" | "garuda_small" => "red",
        "gentoo" | "gentoo_small" => "magenta",
        "grapheneos" => "white",
        "haiku" | "haiku2" | "haiku_small" => "green",
        "harmonyos" => "blue",
        "kali" | "kali_small" => "blue",
        "kdeneon" => "cyan",
        "kiss" | "kiss2" => "white",
        "kubuntu" => "blue",
        "linux" | "linux_small" => "yellow",
        "linuxmint" | "linuxmint2" | "linuxmint_old" | "linuxmint_small" | "mint" => "green",
        "lmde" => "green",
        "macos" | "macos2" | "macos3" | "macos_small" | "macos2_small" => "white",
        "mandriva" => "cyan",
        "manjaro" | "manjaro_small" => "green",
        "cleanjaro" | "cleanjaro_small" => "green",
        "nixos" | "nixos2" | "nixos_old" | "nixos_small" | "nixos_old_small" => "blue",
        "nobara" => "red",
        "openbsd" | "openbsd_small" => "yellow",
        "opensuse" | "opensuse_leap" | "opensuse_leap_old" | "opensuse_microos"
        | "opensuse_slowroll" | "opensuse_small" | "opensuse_tumbleweed"
        | "opensuse_tumbleweed2" | "opensuse_tumbleweed_old"
        | "opensuse_tumbleweed_small" | "suse" => "green",
        "openwrt" | "openwrt_old" | "openwrt_small" => "blue",
        "parrot" => "cyan",
        "pentoo" => "magenta",
        "pop" | "pop_small" => "cyan",
        "postmarketos" | "postmarketos2" | "postmarketos_small" => "green",
        "puppy" => "yellow",
        "raspbian" | "raspbian_small" => "red",
        "rhel" | "rhel_old" | "rhel_small" => "red",
        "rocky" | "rocky_small" => "blue",
        "slackware" | "slackware_small" => "blue",
        "steamdeck" | "steamdeck_small" | "steamos" => "blue",
        "t2" | "t2_small" => "white",
        "templeos" => "yellow",
        "ubuntu" | "ubuntu_old" | "ubuntu_old2" | "ubuntu_old2_small"
        | "ubuntu_small" | "ubuntu_budgie" | "ubuntu_cinnamon" | "ubuntu_gnome"
        | "ubuntu_kylin" | "ubuntu_mate" | "ubuntu_studio" | "ubuntu_sway"
        | "ubuntu_touch" | "ubuntu_unity" => "red",
        "xubuntu" => "blue",
        "lubuntu" => "blue",
        "void" | "void2" | "void_small" | "void2_small" => "green",
        "windows" | "windows_11" | "windows_11_small" | "windows_2025" | "windows_8" => "blue",
        "windows_95" => "cyan",
        "zorin" => "cyan",
        "aix" => "blue",
        "almalinux" => "cyan",
        "android" | "android_small" => "green",
        "antergos" => "blue",
        "antix" => "blue",
        "armbian" | "armbian2" => "red",
        "astra_linux" => "blue",
        "azurelinux" | "azurelinux2" => "blue",
        "biglinux" => "blue",
        "blackarch" => "red",
        "bodhi" => "green",
        "bunsenlabs" => "blue",
        "cbl_mariner" => "blue",
        "chakra" => "blue",
        "chimera_linux" | "chimera_linux2" | "chimera_linux_small" => "red",
        "clear_linux" => "blue",
        "clearos" => "blue",
        "cosmic" => "blue",
        "deepin" => "red",
        "uos" => "red",
        "dietpi" => "red",
        "drauger" => "blue",
        "dragonfly" | "dragonfly_old" | "dragonfly_small" => "red",
        "endless" => "blue",
        "exherbo" => "magenta",
        "feren" => "blue",
        "flatcar" => "blue",
        "frugalware" => "blue",
        "funtoo" => "magenta",
        "galliumos" => "blue",
        "ghostbsd" => "blue",
        "gnu" => "red",
        "guix" | "guix_small" => "yellow",
        "hyperbola" | "hyperbola_small" => "white",
        "kaos" => "blue",
        "kdelinux" => "cyan",
        "korora" => "blue",
        "kylin" => "blue",
        "openkylin" => "red",
        "lede" => "blue",
        "libreelec" => "blue",
        "linuxlite" | "linuxlite_small" => "green",
        "mageia" | "mageia_small" => "cyan",
        "mer" => "blue",
        "midnightbsd" => "white",
        "minix" => "blue",
        "msys2" => "magenta",
        "mx" | "mx2" | "mx_small" => "white",
        "netbsd" | "netbsd2" | "netbsd_small" => "yellow",
        "netrunner" => "blue",
        "nitrux" => "blue",
        "nomadbsd" => "blue",
        "nutyx" => "green",
        "obarun" => "cyan",
        "omnios" => "white",
        "openeuler" => "blue",
        "openindiana" => "blue",
        "openmandriva" => "blue",
        "opnsense" => "blue",
        "oracle" => "red",
        "osmc" => "blue",
        "pacbsd" => "red",
        "parabola" | "parabola_small" | "parabola2_small" => "green",
        "pardus" => "blue",
        "pcbsd" => "red",
        "pclinuxos" => "blue",
        "peppermint" => "green",
        "pikaos" => "blue",
        "pisi" => "cyan",
        "porteus" => "cyan",
        "proxmox" => "red",
        "pureos" | "pureos_small" => "white",
        "q4os" => "blue",
        "qubes" | "qubes_small" => "blue",
        "redcore" => "red",
        "redos" | "redos_small" => "red",
        "redstar" => "red",
        "refracta" => "blue",
        "regata" => "blue",
        "regolith" => "red",
        "rosa" => "blue",
        "sabotage" => "white",
        "sailfish" => "blue",
        "salix" => "green",
        "scientific" => "blue",
        "septor" => "blue",
        "serene" => "blue",
        "serpent_os" => "green",
        "shastraos" => "yellow",
        "siduction" => "blue",
        "slackel" => "blue",
        "slitaz" => "yellow",
        "smartos" => "white",
        "soda" => "blue",
        "solaris" | "solaris_small" => "yellow",
        "solus" => "blue",
        "sparky" => "red",
        "tails" => "magenta",
        "trisquel" => "blue",
        "truenas_scale" => "red",
        "turkish" => "red",
        "tuxedo_os" => "red",
        "twister" => "green",
        "ultramarine" | "ultramarine_small" => "blue",
        "vanilla" | "vanilla2" | "vanilla_small" => "yellow",
        "venom" | "venom_small" => "white",
        "vzlinux" => "red",
        "wii_linux" => "blue",
        "xcp_ng" => "blue",
        "zerene" => "blue",
        "zos" => "white",
        _ => return family_for(&low),
    };
    exact
}

fn family_for(low: &str) -> &'static str {
    if low.starts_with("ubuntu") { "red" }
    else if low.starts_with("fedora") { "blue" }
    else if low.starts_with("opensuse") || low == "suse" { "green" }
    else if low.starts_with("alpine") { "blue" }
    else if low.starts_with("arch") || low.starts_with("arco") || low.starts_with("archcraft")
        || low.starts_with("archlabs") || low.starts_with("archbox") || low.starts_with("archstrike") { "cyan" }
    else if low.starts_with("artix") { "cyan" }
    else if low.starts_with("void") { "green" }
    else if low.starts_with("nixos") { "blue" }
    else if low.starts_with("linuxmint") || low.starts_with("mint") || low == "lmde" { "green" }
    else if low.starts_with("manjaro") || low.starts_with("cleanjaro") { "green" }
    else if low.starts_with("debian") { "red" }
    else if low.starts_with("rhel") || low.starts_with("centos") || low == "rocky"
        || low == "almalinux" || low == "oracle" || low == "scientific"
        || low == "eurolinux" || low == "miracle_linux" || low == "vzlinux" { "red" }
    else if low.starts_with("freebsd") { "red" }
    else if low.starts_with("openbsd") { "yellow" }
    else if low.starts_with("netbsd") { "yellow" }
    else if low.starts_with("dragonfly") { "red" }
    else if low.starts_with("macos") { "white" }
    else if low.starts_with("windows") { "blue" }
    else if low.starts_with("android") { "green" }
    else if low.starts_with("gentoo") || low == "funtoo" { "magenta" }
    else if low.starts_with("slackware") || low == "slackel" { "blue" }
    else if low.starts_with("garuda") { "red" }
    else if low.starts_with("endeavour") { "magenta" }
    else if low.starts_with("cachyos") { "cyan" }
    else if low.starts_with("solus") { "blue" }
    else if low.starts_with("mx") { "white" }
    else if low.starts_with("haiku") { "green" }
    else if low.starts_with("devuan") { "magenta" }
    else if low.starts_with("kali") { "blue" }
    else if low.starts_with("postmarket") { "green" }
    else if low.starts_with("openwrt") { "blue" }
    else if low.starts_with("chimera") { "red" }
    else if low.starts_with("parabola") { "green" }
    else { hash_color(low) }
}

fn hash_color(s: &str) -> &'static str {
    let mut h: u32 = 2166136261;
    for b in s.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(16777619);
    }
    match h % 6 {
        0 => "red",
        1 => "green",
        2 => "yellow",
        3 => "blue",
        4 => "magenta",
        _ => "cyan",
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
