//! Disk format detection.
//!
//! The Greaseweazle host tools cannot tell a disk's format. Floppy-Copy reads cylinders 0-2 as
//! raw flux, then decodes cylinder 0 with every format gw knows and keeps those that find every
//! sector. The fast way runs `py/detect.py` with gw's own Python, in one process; when gw has no
//! reachable Python (a frozen `gw.exe`), `gw convert` is run once per format.
//!
//! Ranking: formats that found every sector, the most sectors first. Formats with the same
//! layout on cylinder 0 (PC 720K, Atari ST 720K, MSX 2DD…) are told apart by the boot sector:
//! the BIOS parameter block gives the disk size, the first byte and the OEM name the system.
//! For a 40-track format, cylinder 1 decoded from physical cylinder 2 but not 1 shows a
//! 40-track disk read in an 80-track drive: it needs a double step.

use crate::formats::PRESETS;
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// The detector run with gw's Python.
pub const SCRIPT: &str = include_str!("py/detect.py");

/// Prefix of the detector's result lines.
pub const PREFIX: &str = "FCDETECT ";

/// The cylinders read from the disk for detection.
pub const READ_TRACKS: &str = "c=0-2";

/// Result of one format on cylinder 0.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct Candidate {
    pub format: String,
    #[serde(default)]
    pub found: u32,
    #[serde(default)]
    pub total: u32,
    #[serde(default)]
    pub cyls: u16,
    #[serde(default)]
    pub heads: u8,
    /// First bytes of cylinder 0, side 0, as hex.
    #[serde(default)]
    pub boot: String,
    #[serde(default)]
    pub cyl1_step1: Option<u32>,
    #[serde(default)]
    pub cyl1_step2: Option<u32>,
    #[serde(default)]
    pub cyl1_total: Option<u32>,
    #[serde(default)]
    pub error: Option<String>,
}

impl Candidate {
    pub fn complete(&self) -> bool {
        self.total > 0 && self.found == self.total && self.error.is_none()
    }

    fn boot_bytes(&self) -> Vec<u8> {
        hex::decode(&self.boot).unwrap_or_default()
    }

    /// Cylinder 1 is where a double step puts it, not where a single step does.
    pub fn needs_double_step(&self) -> bool {
        match (self.cyl1_step1, self.cyl1_step2, self.cyl1_total) {
            (Some(one), Some(two), Some(total)) => total > 0 && two == total && one < total,
            _ => false,
        }
    }
}

/// One line of the detector's output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Line {
    /// Number of formats about to be tried.
    Formats(usize),
    Result(Candidate),
    Done,
    Other,
}

pub fn parse_line(line: &str) -> Line {
    let Some(json) = line.trim_end().strip_prefix(PREFIX) else { return Line::Other };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else { return Line::Other };
    if let Some(n) = v.get("formats").and_then(|n| n.as_u64()) {
        return Line::Formats(n as usize);
    }
    if v.get("done").is_some() {
        return Line::Done;
    }
    match serde_json::from_value::<Candidate>(v) {
        Ok(c) => Line::Result(c),
        Err(_) => Line::Other,
    }
}

/// What the boot sector says about the disk.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct BootHints {
    /// Disk size from a BIOS parameter block (FAT disks: PC, Atari ST, MSX).
    fat_size: Option<u64>,
    /// x86 jump: a PC (or MSX) boot sector.
    pc_jump: bool,
    msx: bool,
    amiga: bool,
}

fn boot_hints(b: &[u8]) -> BootHints {
    let mut h = BootHints::default();
    if b.len() >= 3 && &b[..3] == b"DOS" {
        h.amiga = true;
    }
    if b.len() >= 30 {
        let u16le = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]) as u64;
        let (bps, spt, heads, total) = (u16le(11), u16le(24), u16le(26), u16le(19));
        if [128, 256, 512, 1024].contains(&bps)
            && (1..=63).contains(&spt)
            && (1..=2).contains(&heads)
            && total > 0
        {
            h.fat_size = Some(total * bps);
        }
        h.pc_jump = b[0] == 0xEB || b[0] == 0xE9;
        h.msx = b[3..11].windows(3).any(|w| w.eq_ignore_ascii_case(b"MSX"));
    }
    h
}

/// Image size a format implies, from its cylinder-0 sector count and the sector size.
fn format_size(c: &Candidate, sector: u64) -> u64 {
    c.cyls as u64 * c.total as u64 * sector
}

/// Bonus from the boot sector: higher is more likely.
fn hint_score(c: &Candidate, h: &BootHints) -> i32 {
    let mut s = 0;
    let family = c.format.split('.').next().unwrap_or("");
    if h.amiga && family == "amiga" {
        s += 8;
    }
    if let Some(size) = h.fat_size {
        if format_size(c, 512) == size {
            s += 4;
        }
        match family {
            "ibm" if h.pc_jump && !h.msx => s += 2,
            "msx" if h.msx => s += 3,
            "atarist" if !h.pc_jump => s += 2,
            _ => {}
        }
    }
    s
}

/// Position in the presets (common machines first), for ties.
fn preset_rank(format: &str) -> usize {
    PRESETS.iter().position(|p| p.format == format).unwrap_or(PRESETS.len())
}

/// The outcome of a detection.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Detection {
    /// Formats that decoded every sector of cylinder 0, most likely first.
    pub ranked: Vec<Candidate>,
    /// Formats that decoded some sectors only, most sectors first (for the report).
    pub partial: Vec<Candidate>,
    /// The best format is a 40-track one and the disk was read in an 80-track drive.
    pub double_step: bool,
}

impl Detection {
    pub fn best(&self) -> Option<&Candidate> {
        self.ranked.first()
    }

    /// One-line summary for the console and the banner.
    pub fn summary(&self) -> String {
        match self.best() {
            None => match self.partial.first() {
                Some(p) => format!(
                    "No format decodes every sector of cylinder 0. Closest: {} ({}/{} sectors). \
                     The disk may be damaged, unformatted or copy-protected: read it as flux (SCP).",
                    p.format, p.found, p.total
                ),
                None => "No known format found on cylinder 0: the disk may be blank, or in a format \
                         gw cannot decode. Read it as flux (SCP)."
                    .into(),
            },
            Some(b) => {
                let mut s = format!("Detected {} ({}/{} sectors on cylinder 0)", b.format, b.found, b.total);
                let others: Vec<&str> =
                    self.ranked.iter().skip(1).take(3).map(|c| c.format.as_str()).collect();
                if !others.is_empty() {
                    s += &format!("; also possible: {}", others.join(", "));
                }
                s.push('.');
                if self.double_step {
                    s += " 40-track disk in an 80-track drive: double step set.";
                }
                s
            }
        }
    }
}

/// Rank the detector's results.
pub fn rank(cands: &[Candidate]) -> Detection {
    let boot = cands
        .iter()
        .filter(|c| c.complete() && !c.boot.is_empty())
        .max_by_key(|c| c.found)
        .map(|c| boot_hints(&c.boot_bytes()))
        .unwrap_or_default();
    let mut ranked: Vec<Candidate> = cands.iter().filter(|c| c.complete()).cloned().collect();
    ranked.sort_by(|a, b| {
        b.found
            .cmp(&a.found)
            .then(hint_score(b, &boot).cmp(&hint_score(a, &boot)))
            .then(preset_rank(&a.format).cmp(&preset_rank(&b.format)))
            .then(a.format.cmp(&b.format))
    });
    let mut partial: Vec<Candidate> =
        cands.iter().filter(|c| !c.complete() && c.found > 0 && c.error.is_none()).cloned().collect();
    partial.sort_by(|a, b| b.found.cmp(&a.found).then(a.format.cmp(&b.format)));
    let double_step = ranked.first().is_some_and(|b| b.cyls <= 42 && b.needs_double_step());
    Detection { ranked, partial, double_step }
}

/// gw's Python interpreter, when it can be found: the one the bundled tools ship with, the
/// `#!` line of a `gw` script, or the Python of a Windows `Scripts\gw.exe`.
pub fn gw_python(program: &str) -> Option<PathBuf> {
    let gw = crate::locate::resolve(program)?;
    let dir = gw.parent()?;
    // The macOS bundle ships one Python per processor.
    let arch = if cfg!(target_arch = "aarch64") { "arm64" } else { "x86_64" };
    let mut tries: Vec<PathBuf> = vec![
        dir.join(format!("python-{arch}")).join("bin").join("python3"),
        dir.join("python").join("bin").join("python3"),
        dir.join("python").join("python.exe"),
        dir.join("python.exe"),
        dir.join("..").join("python.exe"),
    ];
    // A `#!` naming Python (pip installs); the bundle's own launcher is a shell script.
    if let Some(interp) = shebang(&gw)
        .filter(|i| i.file_name().is_some_and(|n| n.to_string_lossy().to_lowercase().starts_with("python")))
    {
        tries.insert(0, interp);
    }
    tries.into_iter().find(|p| p.is_file())
}

/// Interpreter of a `#!` script (`#!/usr/bin/python3`, `#!/usr/bin/env python3`).
fn shebang(path: &Path) -> Option<PathBuf> {
    let mut head = [0u8; 256];
    let n = std::io::Read::read(&mut std::fs::File::open(path).ok()?, &mut head).ok()?;
    let text = std::str::from_utf8(&head[..n]).ok()?;
    let line = text.strip_prefix("#!")?.lines().next()?.trim();
    let mut parts = line.split_whitespace();
    let first = parts.next()?;
    if first.ends_with("/env") {
        let name = parts.find(|p| !p.starts_with('-'))?;
        return crate::locate::resolve(name);
    }
    Some(PathBuf::from(first))
}

/// Read the first bytes of a converted image (fallback path: boot sector hints).
pub fn first_bytes(path: &Path, n: usize) -> String {
    match std::fs::read(path) {
        Ok(d) => hex::encode(&d[..d.len().min(n)]),
        Err(_) => String::new(),
    }
}

/// Formats worth trying (the scanning and raw ones decode anything).
pub fn worth_trying(format: &str) -> bool {
    !format.ends_with(".scan") && !format.starts_with("raw.")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(f: &str, found: u32, total: u32, cyls: u16, boot: &[u8]) -> Candidate {
        Candidate {
            format: f.into(),
            found,
            total,
            cyls,
            heads: 2,
            boot: hex::encode(boot),
            ..Default::default()
        }
    }

    fn pc_boot(total: u16, spt: u16, jump: u8, oem: &[u8]) -> Vec<u8> {
        let mut b = vec![0u8; 64];
        b[0] = jump;
        b[3..3 + oem.len()].copy_from_slice(oem);
        b[11..13].copy_from_slice(&512u16.to_le_bytes());
        b[19..21].copy_from_slice(&total.to_le_bytes());
        b[24..26].copy_from_slice(&spt.to_le_bytes());
        b[26..28].copy_from_slice(&2u16.to_le_bytes());
        b
    }

    #[test]
    fn parses_detector_lines() {
        assert_eq!(parse_line("FCDETECT {\"formats\": 140}"), Line::Formats(140));
        assert_eq!(parse_line("FCDETECT {\"done\": true}"), Line::Done);
        assert_eq!(parse_line("T0.0: something"), Line::Other);
        let Line::Result(c) = parse_line(
            "FCDETECT {\"format\": \"ibm.360\", \"found\": 18, \"total\": 18, \"cyls\": 40, \"heads\": 2, \
             \"boot\": \"eb3c90\", \"index\": 3, \"cyl1_step1\": 0, \"cyl1_step2\": 9, \"cyl1_total\": 9}",
        ) else {
            panic!()
        };
        assert!(c.complete() && c.needs_double_step());
        let Line::Result(e) =
            parse_line("FCDETECT {\"format\": \"zx.rocky.ss40\", \"error\": \"x\", \"index\": 1}")
        else {
            panic!()
        };
        assert!(!e.complete());
    }

    #[test]
    fn pc_720k_beats_its_lookalikes() {
        let b = pc_boot(1440, 9, 0xEB, b"MSDOS5.0");
        let d = rank(&[
            cand("atarist.720", 18, 18, 80, &b),
            cand("ibm.360", 18, 18, 40, &b),
            cand("msx.2dd", 18, 18, 80, &b),
            cand("ibm.720", 18, 18, 80, &b),
            cand("ibm.320", 16, 16, 40, &b),
            cand("ibm.800", 18, 20, 80, &b),
        ]);
        assert_eq!(d.best().unwrap().format, "ibm.720");
        assert_eq!(d.ranked.len(), 5);
        assert_eq!(d.partial[0].format, "ibm.800");
        assert!(!d.double_step);
        assert!(d.summary().starts_with("Detected ibm.720"));
    }

    #[test]
    fn boot_sector_picks_the_system() {
        // Atari ST: no x86 jump.
        let st = pc_boot(1440, 9, 0x60, b"Loader");
        let d = rank(&[cand("ibm.720", 18, 18, 80, &st), cand("atarist.720", 18, 18, 80, &st)]);
        assert_eq!(d.best().unwrap().format, "atarist.720");
        // MSX-DOS.
        let msx = pc_boot(1440, 9, 0xEB, b"MSX_04.0");
        let d = rank(&[cand("ibm.720", 18, 18, 80, &msx), cand("msx.2dd", 18, 18, 80, &msx)]);
        assert_eq!(d.best().unwrap().format, "msx.2dd");
        // PC 360K: the BPB gives the size.
        let b = pc_boot(720, 9, 0xEB, b"IBM  3.3");
        let d = rank(&[cand("ibm.720", 18, 18, 80, &b), cand("ibm.360", 18, 18, 40, &b)]);
        assert_eq!(d.best().unwrap().format, "ibm.360");
    }

    #[test]
    fn more_sectors_win_and_double_step_is_found() {
        let d = rank(&[cand("ibm.1200", 30, 30, 80, &[]), cand("ibm.1440", 36, 36, 80, &[])]);
        assert_eq!(d.best().unwrap().format, "ibm.1440");
        let mut c = cand("ibm.360", 18, 18, 40, &pc_boot(720, 9, 0xEB, b"IBM"));
        c.cyl1_step1 = Some(0);
        c.cyl1_step2 = Some(9);
        c.cyl1_total = Some(9);
        let d = rank(&[c]);
        assert!(d.double_step);
        assert!(d.summary().contains("double step"));
        let none = rank(&[cand("amiga.amigados", 3, 22, 80, &[])]);
        assert!(none.best().is_none());
        assert!(none.summary().contains("Closest: amiga.amigados"));
    }

    #[test]
    fn finds_the_python_of_a_gw_script() {
        let dir = std::env::temp_dir().join(format!("fc-detect-test-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("python/bin")).unwrap();
        let py = dir.join("python/bin/python3");
        std::fs::write(&py, "").unwrap();
        let gw = dir.join("gw");
        std::fs::write(&gw, format!("#!{}\nimport sys\n", py.display())).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&gw, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        assert_eq!(gw_python(gw.to_str().unwrap()), Some(py));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
