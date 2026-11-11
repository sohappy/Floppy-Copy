//! Drive type (40 or 80 tracks) and the limits it puts on every command.
//!
//! A 5.25" DD drive (360K) has 40 cylinders: sending its head beyond cylinder 39 drives it into
//! the mechanical end stop. gw does not know the drive type, so Floppy-Copy checks every command
//! against the type chosen for the drive before running it.

use crate::gw::{Action, TrackOpts};
use crate::trackset::parse_set;
use serde::{Deserialize, Serialize};

/// Mechanical type of a drive.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DriveType {
    /// 3.5" drives and 5.25" HD (1.2M) drives: 80 cylinders and a few more.
    #[default]
    Tracks80,
    /// 5.25" DD drives (360K PC, many 8-bit machines), 3" drives: 40 cylinders.
    Tracks40,
}

impl DriveType {
    pub const ALL: [DriveType; 2] = [DriveType::Tracks80, DriveType::Tracks40];

    pub fn label(self) -> &'static str {
        match self {
            DriveType::Tracks80 => "80 tracks",
            DriveType::Tracks40 => "40 tracks",
        }
    }

    pub fn help(self) -> &'static str {
        match self {
            DriveType::Tracks80 => "3.5\" drives and 5.25\" HD (1.2M) drives",
            DriveType::Tracks40 => {
                "5.25\" DD drives (PC 360K, IBM PC/XT, most 8-bit machines) and 3\" drives: \
                 the head stops at cylinder 39"
            }
        }
    }

    /// Highest cylinder the head can reach safely.
    pub fn last_cyl(self) -> u16 {
        match self {
            DriveType::Tracks80 => 83,
            DriveType::Tracks40 => 39,
        }
    }
}

/// Cylinders defined by gw's disk definitions (`greaseweazle/data/diskdefs_*.cfg`).
const FORMAT_CYLS: &[(&str, u16)] = &[
    ("acorn.adfs.160", 40),
    ("acorn.adfs.320", 80),
    ("acorn.adfs.640", 80),
    ("acorn.adfs.800", 80),
    ("acorn.adfs.1600", 80),
    ("acorn.dfs.ss", 40),
    ("acorn.dfs.ss80", 80),
    ("acorn.dfs.ds", 40),
    ("acorn.dfs.ds80", 80),
    ("akai.800", 80),
    ("akai.1600", 80),
    ("amiga.amigados", 80),
    ("amiga.amigados_hd", 80),
    ("apple2.nofs.140", 35),
    ("apple2.nofs.114", 35),
    ("apple2.appledos.140", 35),
    ("apple2.appledos.114", 35),
    ("apple2.prodos.140", 35),
    ("apricot.single", 70),
    ("apricot.double", 80),
    ("atari.90", 40),
    ("atari.130", 40),
    ("atarist.360", 80),
    ("atarist.400", 80),
    ("atarist.440", 80),
    ("atarist.720", 80),
    ("atarist.800", 80),
    ("atarist.880", 80),
    ("coco.decb", 35),
    ("coco.decb.40t", 40),
    ("coco.os9.40ss", 40),
    ("coco.os9.40ds", 40),
    ("coco.os9.80ss", 80),
    ("coco.os9.80ds", 80),
    ("commodore.1541", 40),
    ("commodore.1571", 35),
    ("commodore.1581", 80),
    ("commodore.cmd.fd2000.dd", 81),
    ("commodore.cmd.fd2000.hd", 81),
    ("commodore.cmd.fd4000.ed", 81),
    ("compucorp665.disk_77_ss", 77),
    ("compucorp665.disk_40_ss", 40),
    ("compucorp665.disk_40_ds", 40),
    ("compucorp665.disk_70_ss", 70),
    ("compucorp665.disk_70_ds", 70),
    ("compucorp665.disk_80_ss", 80),
    ("compucorp665.disk_80_ds", 80),
    ("datageneral.2f", 77),
    ("dec.rx01", 77),
    ("dec.rx02", 77),
    ("dec.rx50", 80),
    ("dragon.40ss", 40),
    ("dragon.40ds", 40),
    ("dragon.80ss", 80),
    ("dragon.80ds", 80),
    ("eagle.ssqd.400", 80),
    ("eagle.dsqd.800", 80),
    ("ensoniq.800", 80),
    ("ensoniq.1600", 80),
    ("ensoniq.mirage", 80),
    ("epson.qx10.logo", 40),
    ("epson.qx10.booter", 15),
    ("epson.qx10.400", 40),
    ("epson.qx10.399", 40),
    ("epson.qx10.396", 40),
    ("epson.qx10.320", 40),
    ("gem.1600", 80),
    ("hp.mmfm.9885", 67),
    ("hp.mmfm.9895", 77),
    ("ibm.160", 40),
    ("ibm.180", 40),
    ("ibm.320", 40),
    ("ibm.360", 40),
    ("ibm.720", 80),
    ("ibm.800", 80),
    ("ibm.1200", 80),
    ("ibm.1440", 80),
    ("ibm.1680", 80),
    ("ibm.dmf", 80),
    ("ibm.2880", 80),
    ("ibm.scan", 80),
    ("kaypro.ssdd.40", 40),
    ("kaypro.dsdd.40", 40),
    ("kaypro.dsdd.80", 80),
    ("luxor.80", 40),
    ("luxor.160", 40),
    ("luxor.320", 40),
    ("luxor.640", 80),
    ("luxor.1000.data", 77),
    ("luxor.1000.program", 77),
    ("luxor.1000.abcnet", 77),
    ("mac.400", 80),
    ("mac.800", 80),
    ("micropolis.48tpi.ss", 35),
    ("micropolis.48tpi.ds", 35),
    ("micropolis.48tpi.ss.275", 35),
    ("micropolis.48tpi.ds.275", 35),
    ("micropolis.100tpi.ss", 77),
    ("micropolis.100tpi.ds", 77),
    ("micropolis.100tpi.ss.275", 77),
    ("micropolis.100tpi.ds.275", 77),
    ("mm1.os9.80dshd_32", 80),
    ("mm1.os9.80dshd_33", 80),
    ("mm1.os9.80dshd_36", 80),
    ("mm1.os9.80dshd_37", 80),
    ("msx.1d", 40),
    ("msx.1dd", 80),
    ("msx.2d", 40),
    ("msx.2dd", 80),
    ("northstar.fm.ss", 35),
    ("northstar.fm.ds", 35),
    ("northstar.mfm.ss", 35),
    ("northstar.mfm.ds", 35),
    ("occ1.sd", 40),
    ("occ1.dd", 40),
    ("olivetti.m20", 35),
    ("pc98.2d", 40),
    ("pc98.2dd", 77),
    ("pc98.2hd", 77),
    ("pc98.n88basic.hd", 77),
    ("pc98.2hs", 81),
    ("rm.8in.sd", 77),
    ("rm.5in.sd", 40),
    ("rm.5in.dd", 40),
    ("rm.5in.qd", 80),
    ("sci.prophet", 80),
    ("sega.sf7000", 40),
    ("sharp.2d", 40),
    ("sharp.2hs", 81),
    ("thomson.1s80", 40),
    ("thomson.1s160", 40),
    ("thomson.2s160", 40),
    ("thomson.1s320", 80),
    ("thomson.2s320", 80),
    ("tsc.flex.dsdd", 80),
    ("tsc.flex.ssdd", 80),
    ("xerox.860.ss", 77),
    ("xerox.860.dssd", 77),
    ("zx.trdos.ds80", 80),
    ("zx.quorum.ds80", 80),
    ("zx.d80.ds80", 80),
    ("zx.fdd3000.ss40", 40),
    ("zx.fdd3000.ds80", 80),
    ("zx.kempston.ss40", 40),
    ("zx.kempston.ds80", 80),
    ("zx.plusd.ds80", 80),
    ("zx.opus.ss40", 40),
    ("zx.opus.ds80", 80),
    ("zx.3dos.ss40", 40),
    ("zx.3dos.ds80", 80),
    ("zx.rocky.ss40", 40),
    ("zx.rocky.ds80", 80),
    ("zx.turbodrive.ds40", 40),
    ("zx.turbodrive.ds80", 80),
    ("zx.watford.ss40", 40),
    ("zx.watford.ds80", 80),
];

/// Number of cylinders a gw format uses, when known.
pub fn format_cyls(format: &str) -> Option<u16> {
    FORMAT_CYLS.iter().find(|(f, _)| *f == format).map(|&(_, c)| c)
}

/// Highest cylinder of the `c=` part of a TSPEC (`None` when there is none).
fn max_cyl(spec: &str) -> Result<Option<u16>, String> {
    for part in spec.split(':') {
        if let Some(set) = part.trim().strip_prefix("c=") {
            return Ok(parse_set(set)?.into_iter().max());
        }
    }
    Ok(None)
}

fn step_of(spec: &str) -> u32 {
    spec.split(':').find_map(|p| p.trim().strip_prefix("step=").and_then(|s| s.parse().ok())).unwrap_or(1)
}

fn is_scan(format: &str) -> bool {
    format == "ibm.scan" || format.starts_with("raw.")
}

const SET_HINT: &str = "Change the drive type next to the drive selector if it is wrong.";

/// Check (and where safe, restrict) the tracks of a disk command for a 40-track drive.
fn limit_tracks(
    t: &mut TrackOpts,
    format: &str,
    verb: &str,
    warnings: &mut Vec<String>,
) -> Result<(), String> {
    let custom = t.custom.trim().to_string();
    if t.double_step || step_of(&custom) > 1 {
        return Err(format!(
            "Double step (step=2) is for 40-track disks in an 80-track drive. This drive is a \
             40-track drive: it needs no double step, which would send the head to cylinder 78. \
             Untick Double step. {SET_HINT}"
        ));
    }
    match max_cyl(&custom).map_err(|e| format!("Tracks: {e}"))? {
        Some(m) if m > 39 => Err(format!(
            "The tracks ({custom}) go up to cylinder {m}, but this drive has 40 tracks (cylinders \
             0-39): the head would hit its end stop. Limit the tracks to c=0-39. {SET_HINT}"
        )),
        Some(_) => Ok(()),
        // Scanning and raw formats read whatever cylinders they are given.
        None => match format_cyls(format).filter(|_| !is_scan(format)) {
            Some(c) if c <= 40 => Ok(()),
            Some(c) => Err(format!(
                "{format} uses {c} cylinders, but this drive has 40 tracks: the head would hit its \
                 end stop at cylinder 40. This format needs an 80-track drive (3.5\" or 5.25\" HD). \
                 {SET_HINT}"
            )),
            None if format.is_empty() || is_scan(format) => {
                t.custom = if custom.is_empty() { "c=0-39".into() } else { format!("c=0-39:{custom}") };
                warnings.push(format!("40-track drive: only cylinders 0-39 are {verb}."));
                Ok(())
            }
            None => {
                warnings.push(format!(
                    "Floppy-Copy does not know how many cylinders {format} uses. If it uses more \
                     than 40, stop at once: this drive has 40 tracks."
                ));
                Ok(())
            }
        },
    }
}

/// Check an action against the type of the drive it runs on. Commands that would send the head
/// beyond the last cylinder are refused; a command without a cylinder range (raw flux, erase) is
/// limited to the cylinders the drive has. Returns the warnings to show before running.
pub fn apply(action: &mut Action, drive: DriveType) -> Result<Vec<String>, String> {
    let mut warnings = Vec::new();
    if drive == DriveType::Tracks80 {
        return Ok(warnings);
    }
    match action {
        Action::Read(o) => {
            let f = o.format.clone();
            limit_tracks(&mut o.tracks, &f, "read", &mut warnings)?;
        }
        Action::Write(o) => {
            let f = o.format.clone();
            limit_tracks(&mut o.tracks, &f, "written", &mut warnings)?;
        }
        Action::Erase(o) => limit_tracks(&mut o.tracks, "", "erased", &mut warnings)?,
        Action::Seek(o) if o.cylinder > 39 => {
            return Err(format!(
                "Cylinder {} is beyond the last cylinder of this 40-track drive (39). {SET_HINT}",
                o.cylinder
            ))
        }
        Action::Clean(o) if o.cyls > 40 => {
            o.cyls = 40;
            warnings.push("40-track drive: the cleaning sweeps cylinders 0-39 only.".into());
        }
        Action::Align(o) => {
            if let Some(m) = max_cyl(&o.tracks).map_err(|e| format!("Track: {e}"))? {
                if m > 39 {
                    return Err(format!(
                        "Cylinder {m} is beyond the last cylinder of this 40-track drive (39). {SET_HINT}"
                    ));
                }
            }
        }
        _ => {}
    }
    Ok(warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gw::*;

    fn read(format: &str, custom: &str) -> Action {
        let mut o = ReadOpts { format: format.into(), ..Default::default() };
        o.tracks.custom = custom.into();
        Action::Read(o)
    }

    #[test]
    fn eighty_track_drives_are_unchanged() {
        let mut a = read("amiga.amigados", "");
        assert!(apply(&mut a, DriveType::Tracks80).unwrap().is_empty());
        assert_eq!(a, read("amiga.amigados", ""));
    }

    #[test]
    fn refuses_80_cylinder_formats_on_a_40_track_drive() {
        let e = apply(&mut read("amiga.amigados", ""), DriveType::Tracks40).unwrap_err();
        assert!(e.contains("80 cylinders"), "{e}");
        let mut w = Action::Write(WriteOpts { format: "ibm.720".into(), ..Default::default() });
        assert!(apply(&mut w, DriveType::Tracks40).is_err());
        assert!(apply(&mut read("ibm.360", ""), DriveType::Tracks40).unwrap().is_empty());
        assert!(apply(&mut read("commodore.1541", ""), DriveType::Tracks40).is_ok());
        // An explicit range inside the drive is accepted, even for an 80-cylinder format.
        assert!(apply(&mut read("ibm.720", "c=0-39:h=0"), DriveType::Tracks40).is_ok());
        assert!(apply(&mut read("ibm.360", "c=0-41"), DriveType::Tracks40).is_err());
    }

    #[test]
    fn limits_raw_reads_and_erase() {
        let mut a = read("", "");
        let w = apply(&mut a, DriveType::Tracks40).unwrap();
        assert_eq!(w.len(), 1);
        let Action::Read(o) = &a else { panic!() };
        assert_eq!(o.tracks.custom, "c=0-39");
        let mut e = Action::Erase(EraseOpts::default());
        apply(&mut e, DriveType::Tracks40).unwrap();
        let Action::Erase(o) = &e else { panic!() };
        assert_eq!(o.tracks.spec().as_deref(), Some("c=0-39"));
        let mut e = Action::Erase(EraseOpts::default());
        if let Action::Erase(o) = &mut e {
            o.tracks.custom = "h=0".into();
        }
        apply(&mut e, DriveType::Tracks40).unwrap();
        let Action::Erase(o) = &e else { panic!() };
        assert_eq!(o.tracks.custom, "c=0-39:h=0");
        // The CPC scan format defines 80 cylinders: limited too.
        let mut a = read("ibm.scan", "h=0");
        apply(&mut a, DriveType::Tracks40).unwrap();
        let Action::Read(o) = &a else { panic!() };
        assert_eq!(o.tracks.custom, "c=0-39:h=0");
    }

    #[test]
    fn refuses_double_step_seek_and_far_alignment() {
        let mut a = read("ibm.360", "");
        if let Action::Read(o) = &mut a {
            o.tracks.double_step = true;
        }
        assert!(apply(&mut a, DriveType::Tracks40).unwrap_err().contains("Double step"));
        assert!(apply(&mut read("", "c=0-39:step=2"), DriveType::Tracks40).is_err());
        let mut s = Action::Seek(SeekOpts { cylinder: 79, motor_on: true });
        assert!(apply(&mut s, DriveType::Tracks40).is_err());
        let mut s = Action::Seek(SeekOpts { cylinder: 39, motor_on: true });
        assert!(apply(&mut s, DriveType::Tracks40).is_ok());
        let mut c = Action::Clean(CleanOpts::default());
        assert_eq!(apply(&mut c, DriveType::Tracks40).unwrap().len(), 1);
        let Action::Clean(o) = &c else { panic!() };
        assert_eq!(o.cyls, 40);
        let mut al = Action::Align(AlignOpts { tracks: "c=79:h=0".into(), ..Default::default() });
        assert!(apply(&mut al, DriveType::Tracks40).is_err());
    }

    #[test]
    fn table_covers_the_presets() {
        for p in crate::formats::PRESETS.iter().filter(|p| !p.format.is_empty()) {
            assert!(format_cyls(p.format).is_some(), "{}", p.format);
        }
        assert_eq!(format_cyls("amiga.amigados"), Some(80));
        assert_eq!(format_cyls("ibm.360"), Some(40));
    }
}
