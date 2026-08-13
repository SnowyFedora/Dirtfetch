use std::env;
use std::net::UdpSocket;
use std::time::Duration;
#[cfg(target_os = "linux")]
use std::process::Command;

use sysinfo::{Disks, System};

pub struct Info {
    pub hostname: String,
    pub host: Option<String>,
    pub os: String,
    pub kernel: Option<String>,
    pub uptime: Option<String>,
    pub packages: Option<String>,
    pub shell: Option<String>,
    pub de: Option<String>,
    pub wm: Option<String>,
    pub terminal: Option<String>,
    pub resolution: Option<String>,
    pub locale: Option<String>,
    pub local_ip: Option<String>,
    pub cpu: Option<String>,
    pub gpu: Option<String>,
    pub memory: String,
    pub swap: Option<String>,
    pub disks: Vec<(String, String)>,
    pub battery: Option<String>,
}

pub fn collect() -> Info {
    let mut sys = System::new_all();
    sys.refresh_all();

    std::thread::sleep(Duration::from_millis(200));
    sys.refresh_cpu_usage();
    let cpus = sys.cpus();
    let usage = if cpus.is_empty() {
        0.0
    } else {
        let avg = cpus.iter().map(|c| c.cpu_usage().clamp(0.0, 100.0)).sum::<f32>()
            / cpus.len() as f32;
        avg.clamp(0.0, 100.0)
    };

    Info {
        hostname: System::host_name().unwrap_or_else(|| "localhost".into()),
        host: hardware_model(),
        os: os_pretty(),
        kernel: System::kernel_version(),
        uptime: Some(format_uptime(System::uptime())),
        packages: package_summary(),
        shell: shell_name(),
        de: desktop(),
        wm: detect_wm(&sys),
        terminal: terminal_name(),
        resolution: resolution(),
        locale: env::var("LANG")
            .ok()
            .map(|s| s.replace(".UTF-8", "").replace(".utf8", "")),
        local_ip: local_ip(),
        cpu: cpu_line(&sys, usage),
        gpu: gpu_line(),
        memory: {
            let used = sys.used_memory();
            let total = sys.total_memory();
            let pct = if total > 0 { used as f64 / total as f64 * 100.0 } else { 0.0 };
            format!("{} / {} ({:.0}%)", bytes(used), bytes(total), pct)
        },
        swap: (sys.total_swap() > 0).then(|| {
            format!("{} / {}", bytes(sys.used_swap()), bytes(sys.total_swap()))
        }),
        disks: disk_lines(),
        battery: battery_info(),
    }
}

pub fn os_release_field(key: &str) -> Option<String> {
    for path in ["/etc/os-release", "/usr/lib/os-release"] {
        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines() {
                if let Some(v) = line.strip_prefix(&format!("{key}=")) {
                    return Some(v.trim().trim_matches(|c| c == '"' || c == '\'').to_string());
                }
            }
        }
    }
    None
}

fn os_pretty() -> String {
    if let Some(p) = os_release_field("PRETTY_NAME") {
        return p;
    }
    let name = System::name().unwrap_or_default();
    let ver = System::os_version().unwrap_or_default();
    if ver.is_empty() { name } else { format!("{name} {ver}") }
}

#[cfg(target_os = "linux")]
fn read_trim(path: &str) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(target_os = "linux")]
fn hardware_model() -> Option<String> {
    let vendor = read_trim("/sys/class/dmi/id/sys_vendor");
    let product = read_trim("/sys/class/dmi/id/product_name");
    match (vendor, product) {
        (Some(v), Some(p)) => Some(format!("{v} {p}")),
        (None, Some(p)) => Some(p),
        (Some(v), None) => Some(v),
        (None, None) => None,
    }
}

#[cfg(not(target_os = "linux"))]
fn hardware_model() -> Option<String> { None }

fn format_uptime(secs: u64) -> String {
    let d = secs / 86_400;
    let h = (secs % 86_400) / 3_600;
    let m = (secs % 3_600) / 60;
    let mut parts = Vec::new();
    if d > 0 { parts.push(format!("{d}d")); }
    if h > 0 { parts.push(format!("{h}h")); }
    parts.push(format!("{m}m"));
    parts.join(" ")
}

fn bytes(b: u64) -> String {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    if (b as f64) < GIB {
        format!("{:.0} MiB", b as f64 / 1024.0 / 1024.0)
    } else {
        format!("{:.1} GiB", b as f64 / GIB)
    }
}

fn shell_name() -> Option<String> {
    let sh = env::var("SHELL").ok().or_else(|| env::var("COMSPEC").ok())?;
    Some(
        std::path::Path::new(&sh)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&sh)
            .to_string(),
    )
}

fn terminal_name() -> Option<String> {
    env::var("TERM_PROGRAM")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| env::var("TERM").ok().filter(|s| !s.is_empty()))
}

fn desktop() -> Option<String> {
    let de = env::var("XDG_CURRENT_DESKTOP")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| env::var("DESKTOP_SESSION").ok().filter(|s| !s.is_empty()))?;
    match env::var("XDG_SESSION_TYPE").ok().filter(|s| !s.is_empty()) {
        Some(s) => Some(format!("{de} ({s})")),
        None => Some(de),
    }
}

fn detect_wm(sys: &System) -> Option<String> {
    const KNOWN: &[(&str, &str)] = &[
        ("hyprland", "Hyprland"),
        ("sway", "Sway"),
        ("i3", "i3"),
        ("river", "River"),
        ("wayfire", "Wayfire"),
        ("niri", "Niri"),
        ("gnome-shell", "Mutter (GNOME)"),
        ("mutter", "Mutter"),
        ("kwin_wayland", "KWin (Wayland)"),
        ("kwin_x11", "KWin (X11)"),
        ("xfwm4", "Xfwm4 (XFCE)"),
        ("openbox", "Openbox"),
        ("bspwm", "bspwm"),
        ("dwm", "dwm"),
        ("awesome", "Awesome"),
        ("qtile", "Qtile"),
        ("xmonad", "xmonad"),
        ("fluxbox", "Fluxbox"),
        ("marco", "Marco (MATE)"),
    ];
    let names: Vec<String> = sys
        .processes()
        .values()
        .map(|p| p.name().to_lowercase())
        .collect();
    for (proc, pretty) in KNOWN {
        if names.iter().any(|n| n == proc) {
            return Some((*pretty).to_string());
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn resolution() -> Option<String> {
    let out = Command::new("xrandr").output().ok()?;
    if !out.status.success() { return None; }
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut res: Vec<String> = Vec::new();
    for line in text.lines() {
        if !line.contains('*') { continue; }
        if let Some(tok) = line.split_whitespace().next() {
            if tok.contains('x') && tok.starts_with(|c: char| c.is_ascii_digit()) {
                let t = tok.to_string();
                if !res.contains(&t) { res.push(t); }
            }
        }
    }
    if res.is_empty() { None } else { Some(res.join(" + ")) }
}

#[cfg(not(target_os = "linux"))]
fn resolution() -> Option<String> { None }

fn local_ip() -> Option<String> {
    let s = UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("8.8.8.8:80").ok()?;
    s.local_addr().ok().map(|a| a.ip().to_string())
}

fn battery_info() -> Option<String> {
    let mut parts = Vec::new();
    let rd = std::fs::read_dir("/sys/class/power_supply").ok()?;
    for e in rd.filter_map(|e| e.ok()) {
        let p = e.path();
        let t = std::fs::read_to_string(p.join("type")).unwrap_or_default();
        if t.trim() != "Battery" { continue; }
        let cap = std::fs::read_to_string(p.join("capacity")).unwrap_or_default();
        let status = std::fs::read_to_string(p.join("status")).unwrap_or_default();
        if !cap.trim().is_empty() {
            parts.push(format!("{}% {}", cap.trim(), status.trim()));
        }
    }
    if parts.is_empty() { None } else { Some(parts.join(", ")) }
}

#[cfg(target_os = "linux")]
fn cpu_model() -> Option<String> {
    let content = std::fs::read_to_string("/proc/cpuinfo").ok()?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("model name") {
            if let Some(colon) = rest.find(':') {
                let s = rest[colon + 1..].trim();
                if !s.is_empty() {
                    return Some(s.to_string());
                }
            }
        }
    }
    None
}

fn cpu_line(sys: &System, usage: f32) -> Option<String> {
    let cpus = sys.cpus();
    let logical = cpus.len();
    if logical == 0 { return None; }

    #[cfg(target_os = "linux")]
    let model = cpu_model().unwrap_or_else(|| {
        cpus.first().map(|c| c.name().trim().to_string()).unwrap_or_default()
    });
    #[cfg(not(target_os = "linux"))]
    let model = cpus.first()?.name().trim().to_string();

    let cores = match sys.physical_core_count() {
        Some(p) => format!("{p}C/{logical}T"),
        None => format!("{logical}T"),
    };
    let freq = cpus.iter().map(|c| c.frequency()).max().unwrap_or(0);
    let mut s = if freq > 0 {
        format!("{model} @ {:.2} GHz ({cores})", freq as f64 / 1000.0)
    } else {
        format!("{model} ({cores})")
    };
    if usage > 0.0 {
        s.push_str(&format!(" [{usage:.0}%]"));
    }
    Some(s)
}

fn pretty_gpu(s: &str) -> String {
    let s2 = s.split(" (rev").next().unwrap_or(s).trim();
    if let (Some(a), Some(b)) = (s2.rfind('['), s2.rfind(']')) {
        if b > a + 1 {
            return s2[a + 1..b].to_string();
        }
    }
    s2.to_string()
}

#[cfg(target_os = "linux")]
fn gpu_line() -> Option<String> {
    let out = Command::new("lspci").output().ok()?;
    if !out.status.success() { return None; }
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let names: Vec<String> = text
        .lines()
        .filter(|l| l.contains("VGA compatible controller") || l.contains("3D controller"))
        .filter_map(|l| l.split(": ").nth(1))
        .map(|s| pretty_gpu(s))
        .filter(|s| !s.is_empty())
        .collect();
    if names.is_empty() { None } else { Some(names.join(" + ")) }
}

#[cfg(not(target_os = "linux"))]
fn gpu_line() -> Option<String> { None }

fn mount_fstypes() -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    if let Ok(content) = std::fs::read_to_string("/proc/mounts") {
        for line in content.lines() {
            let mut it = line.split_whitespace();
            if let (Some(_dev), Some(mp), Some(fs)) = (it.next(), it.next(), it.next()) {
                map.insert(mp.to_string(), fs.to_string());
            }
        }
    }
    map
}

fn disk_lines() -> Vec<(String, String)> {
    const GOOD_FS: &[&str] = &[
        "ext2", "ext3", "ext4", "btrfs", "xfs", "f2fs", "zfs",
        "ntfs", "ntfs3", "exfat", "vfat",
    ];
    let fstypes = mount_fstypes();
    let disks = Disks::new_with_refreshed_list();
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();

    for d in &disks {
        let mp = d.mount_point().to_string_lossy().to_string();

        // бут-разделы не показываем
        if mp == "/boot" || mp.starts_with("/boot/") { continue; }

        let fs = fstypes.get(&mp).cloned().unwrap_or_default();
        if !GOOD_FS.contains(&fs.as_str()) { continue; }
        let total = d.total_space();
        if total == 0 { continue; }
        let dev = d.name().to_string_lossy().to_string();
        if !seen.insert(format!("{dev}|{total}")) { continue; }
        let used = total.saturating_sub(d.available_space());
        let pct = used as f64 / total as f64 * 100.0;
        out.push((
            mp,
            format!("[{dev}] {} / {} ({:.0}%)", bytes(used), bytes(total), pct),
        ));
    }
    out
}

fn package_summary() -> Option<String> {
    let mut parts = Vec::new();
    if let Some(n) = count_pacman() { parts.push(format!("{n} (pacman)")); }
    if let Some(n) = count_dpkg() { parts.push(format!("{n} (dpkg)")); }
    if let Some(n) = count_rpm() { parts.push(format!("{n} (rpm)")); }
    if let Some(n) = count_xbps() { parts.push(format!("{n} (xbps)")); }
    if let Some(n) = count_apk() { parts.push(format!("{n} (apk)")); }
    if let Some(n) = count_flatpak() { parts.push(format!("{n} (flatpak)")); }
    if let Some(n) = count_snap() { parts.push(format!("{n} (snap)")); }
    if parts.is_empty() { None } else { Some(parts.join(", ")) }
}

fn count_pacman() -> Option<usize> {
    let n = std::fs::read_dir("/var/lib/pacman/local").ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .count();
    (n > 0).then_some(n)
}

fn count_dpkg() -> Option<usize> {
    let status = std::fs::read_to_string("/var/lib/dpkg/status").ok()?;
    let n = status.lines().filter(|l| *l == "Status: install ok installed").count();
    (n > 0).then_some(n)
}

#[cfg(target_os = "linux")]
fn count_rpm() -> Option<usize> {
    let out = Command::new("rpm").arg("-qa").output().ok()?;
    if !out.status.success() { return None; }
    let n = String::from_utf8_lossy(&out.stdout).lines().count();
    (n > 0).then_some(n)
}

#[cfg(not(target_os = "linux"))]
fn count_rpm() -> Option<usize> { None }

fn count_xbps() -> Option<usize> {
    let n = std::fs::read_dir("/var/db/xbps").ok()?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.contains('-') && !name.ends_with(".plist")
        })
        .count();
    (n > 0).then_some(n)
}

fn count_apk() -> Option<usize> {
    let content = std::fs::read_to_string("/lib/apk/db/installed").ok()?;
    let n = content.lines().filter(|l| l.starts_with("P:")).count();
    (n > 0).then_some(n)
}

fn count_flatpak() -> Option<usize> {
    let mut n = 0;
    let mut found = false;
    if let Ok(rd) = std::fs::read_dir("/var/lib/flatpak/app") {
        found = true;
        n += rd.filter_map(|e| e.ok()).filter(|e| e.path().is_dir()).count();
    }
    if let Ok(home) = env::var("HOME") {
        let user_dir = std::path::Path::new(&home).join(".local/share/flatpak/app");
        if let Ok(rd) = std::fs::read_dir(user_dir) {
            found = true;
            n += rd.filter_map(|e| e.ok()).filter(|e| e.path().is_dir()).count();
        }
    }
    (found && n > 0).then_some(n)
}

fn count_snap() -> Option<usize> {
    let n = std::fs::read_dir("/var/lib/snapd/snaps").ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().ends_with(".snap"))
        .count();
    (n > 0).then_some(n)
}
