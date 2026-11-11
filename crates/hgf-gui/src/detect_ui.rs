//! Format detection: read cylinders 0-2 as flux (or take a flux image), decode cylinder 0 with
//! every format gw knows, then apply the most likely one to the page.

use crate::app::{App, JobExtras, Level, Outcome, Page};
use crate::{theme, widgets};
use eframe::egui::{self, RichText, Ui};
use egui_phosphor::regular as ph;
use hgf_core::detect::{self, Candidate, Detection, Line};
use hgf_core::drivelimit::DriveType;
use hgf_core::formats::{self, PRESETS};
use hgf_core::gw::{Action, ReadOpts, ReadProfile, TrackOpts};
use hgf_core::runner::{self, Job, JobEvent};
use hgf_core::Drive;
use std::collections::VecDeque;
use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// gw reads cylinders 0-2 of the disk.
    Reading,
    /// The formats are being tried.
    Decoding,
}

/// One `gw convert` of the fallback (no Python reachable).
struct Convert {
    format: String,
    job: Job,
    out: PathBuf,
    found: Option<(u32, u32)>,
}

pub struct DetectRun {
    pub target: Page,
    pub stage: Stage,
    /// Flux image analysed.
    flux: PathBuf,
    /// The flux was read for the detection: delete it afterwards.
    temp: bool,
    /// Drive the disk was read in.
    drive: Drive,
    started: Instant,
    /// Detector run with gw's Python.
    job: Option<Job>,
    /// `gw convert` per format when there is no Python.
    queue: VecDeque<String>,
    active: Vec<Convert>,
    pub formats: usize,
    pub done: usize,
    results: Vec<Candidate>,
    /// Lines of the detector that are not results (shown if it fails).
    errors: Vec<String>,
}

impl DetectRun {
    pub fn progress(&self) -> f32 {
        if self.formats == 0 {
            0.0
        } else {
            self.done as f32 / self.formats as f32
        }
    }

    pub fn elapsed(&self) -> u64 {
        self.started.elapsed().as_secs()
    }

    fn cancel(&self) {
        if let Some(j) = &self.job {
            j.cancel();
        }
        for c in &self.active {
            c.job.cancel();
        }
    }

    fn cleanup(&self) {
        if self.temp {
            let _ = std::fs::remove_file(&self.flux);
        }
        for c in &self.active {
            let _ = std::fs::remove_file(&c.out);
        }
    }
}

/// Flux images the detector can analyse without reading a disk.
pub fn is_flux_image(path: &std::path::Path) -> bool {
    let ext = formats::extension(path).unwrap_or_default();
    formats::FLUX_SUFFIXES.contains(&ext.as_str()) || ext == "hfe"
}

/// Image extension for a detected format.
fn extension_for(format: &str) -> &'static str {
    if let Some(p) = PRESETS.iter().find(|p| p.format == format) {
        return p.extension;
    }
    match format.split('.').next() {
        Some("amiga") => "adf",
        Some("atarist") => "st",
        Some("commodore") if format.contains("1541") => "d64",
        Some("commodore") if format.contains("1571") => "d71",
        Some("commodore") if format.contains("1581") => "d81",
        _ => "img",
    }
}

impl App {
    /// A detection is decoding (the drive is free, but the page waits for the result).
    pub fn detect_decoding(&self) -> bool {
        self.detect.as_ref().is_some_and(|d| d.stage == Stage::Decoding)
    }

    /// Read cylinders 0-2 of the disk in the selected drive, then detect its format.
    pub fn detect_disk(&mut self, target: Page) {
        if self.busy() {
            return;
        }
        self.detect_seq += 1;
        let flux =
            std::env::temp_dir().join(format!("hgf-detect-{}-{}.scp", std::process::id(), self.detect_seq));
        let _ = std::fs::remove_file(&flux);
        let read = ReadOpts {
            output: flux.clone(),
            format: String::new(),
            profile: ReadProfile::Standard,
            tracks: TrackOpts { custom: detect::READ_TRACKS.into(), ..Default::default() },
            ..Default::default()
        };
        self.detection = None;
        self.next = JobExtras {
            page: Some(target),
            success: Some("Cylinders 0-2 read: trying every format…".into()),
            ..Default::default()
        };
        let drive = self.s.drive;
        self.request(Action::Read(read));
        if self.busy() {
            self.detect = Some(DetectRun {
                target,
                stage: Stage::Reading,
                flux,
                temp: true,
                drive,
                started: Instant::now(),
                job: None,
                queue: VecDeque::new(),
                active: Vec::new(),
                formats: 0,
                done: 0,
                results: Vec::new(),
                errors: Vec::new(),
            });
        }
    }

    /// Detect the format of a flux image file (no disk is read).
    pub fn detect_file(&mut self, target: Page, path: PathBuf) {
        if self.busy() {
            return;
        }
        if !path.is_file() {
            self.notice = Some(format!("Image not found: {}", path.display()));
            return;
        }
        self.detection = None;
        self.detect = Some(DetectRun {
            target,
            stage: Stage::Decoding,
            flux: path,
            temp: false,
            drive: self.s.drive,
            started: Instant::now(),
            job: None,
            queue: VecDeque::new(),
            active: Vec::new(),
            formats: 0,
            done: 0,
            results: Vec::new(),
            errors: Vec::new(),
        });
        self.detect_decode();
    }

    /// The read of cylinders 0-2 ended.
    pub fn detect_read_finished(&mut self, success: bool) {
        let Some(d) = &self.detect else { return };
        if d.stage != Stage::Reading {
            return;
        }
        if success && d.flux.is_file() {
            if let Some(d) = self.detect.as_mut() {
                d.stage = Stage::Decoding;
            }
            self.detect_decode();
        } else if let Some(d) = self.detect.take() {
            d.cleanup();
        }
    }

    /// Start decoding: gw's Python in one process, or one `gw convert` per format.
    fn detect_decode(&mut self) {
        let Some(flux) = self.detect.as_ref().map(|d| d.flux.clone()) else { return };
        let python = detect::gw_python(&self.s.cfg.program);
        let script = python.as_ref().and_then(|_| write_script().ok());
        match (python, script) {
            (Some(py), Some(script)) => {
                self.console.push(
                    format!("$ {} {} {}", py.display(), script.display(), flux.display()),
                    Level::Command,
                );
                let args: Vec<OsString> = vec![script.into_os_string(), flux.into_os_string()];
                match runner::spawn(&py.to_string_lossy(), &args) {
                    Ok(job) => {
                        if let Some(d) = self.detect.as_mut() {
                            d.job = Some(job);
                        }
                    }
                    Err(e) => {
                        self.console.push(format!("WARNING: {e}"), Level::Warn);
                        self.detect_fallback();
                    }
                }
            }
            _ => self.detect_fallback(),
        }
    }

    /// No Python: try the formats with `gw convert`, several at a time.
    fn detect_fallback(&mut self) {
        let formats: VecDeque<String> =
            self.catalog.formats.iter().filter(|f| detect::worth_trying(f)).cloned().collect();
        self.console.push(
            format!(
                "Trying {} formats with gw convert (gw's Python was not found: this takes longer).",
                formats.len()
            ),
            Level::Info,
        );
        if let Some(d) = self.detect.as_mut() {
            d.job = None;
            d.results.clear();
            d.formats = formats.len();
            d.done = 0;
            d.queue = formats;
        }
        self.detect_fill();
    }

    fn detect_fill(&mut self) {
        let parallel = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(2, 8);
        let program = self.s.cfg.program.clone();
        let Some(d) = self.detect.as_mut() else { return };
        while d.active.len() < parallel {
            let Some(format) = d.queue.pop_front() else { break };
            self.detect_seq += 1;
            let out = std::env::temp_dir().join(format!(
                "hgf-detect-{}-{}.img",
                std::process::id(),
                self.detect_seq
            ));
            let args: Vec<OsString> = vec![
                "convert".into(),
                "--format".into(),
                format.clone().into(),
                "--tracks".into(),
                "c=0".into(),
                d.flux.clone().into_os_string(),
                out.clone().into_os_string(),
            ];
            match runner::spawn(&program, &args) {
                Ok(job) => d.active.push(Convert { format, job, out, found: None }),
                Err(_) => d.done += 1,
            }
        }
    }

    /// Drain the detector's output; returns true when something changed.
    pub fn poll_detect(&mut self) -> bool {
        let Some(d) = self.detect.as_mut() else { return false };
        if d.stage != Stage::Decoding {
            return false;
        }
        let mut changed = false;
        let mut finished = false;
        let mut failed = false;
        if let Some(job) = &d.job {
            while let Some(e) = job.try_next() {
                changed = true;
                match e {
                    JobEvent::Line(l) => match detect::parse_line(&l) {
                        Line::Formats(n) => d.formats = n,
                        Line::Result(c) => {
                            d.done += 1;
                            d.results.push(c);
                        }
                        Line::Done => {}
                        Line::Other => {
                            if !l.trim().is_empty() {
                                d.errors.push(l);
                            }
                        }
                    },
                    JobEvent::Partial(_) => {}
                    JobEvent::Finished { code, cancelled } => {
                        finished = true;
                        failed = !cancelled && (code != Some(0) || d.results.is_empty());
                    }
                }
            }
        } else {
            let mut i = 0;
            while i < d.active.len() {
                let c = &mut d.active[i];
                let mut ended = false;
                while let Some(e) = c.job.try_next() {
                    changed = true;
                    match e {
                        JobEvent::Line(l) => {
                            if let Some(rest) = l.trim().strip_prefix("Found ") {
                                let n: Vec<u32> =
                                    rest.split_whitespace().filter_map(|w| w.parse().ok()).take(2).collect();
                                if n.len() == 2 {
                                    c.found = Some((n[0], n[1]));
                                }
                            }
                        }
                        JobEvent::Partial(_) => {}
                        JobEvent::Finished { .. } => ended = true,
                    }
                }
                if ended {
                    let c = d.active.remove(i);
                    d.done += 1;
                    if let Some((found, total)) = c.found {
                        let boot = if found > 0 { detect::first_bytes(&c.out, 64) } else { String::new() };
                        d.results.push(Candidate {
                            cyls: hgf_core::drivelimit::format_cyls(&c.format).unwrap_or(80),
                            format: c.format,
                            found,
                            total,
                            boot,
                            ..Default::default()
                        });
                    }
                    let _ = std::fs::remove_file(&c.out);
                } else {
                    i += 1;
                }
            }
            finished = d.active.is_empty() && d.queue.is_empty();
        }
        if failed {
            let why = d.errors.iter().rev().take(3).cloned().collect::<Vec<_>>().join(" / ");
            self.console
                .push(format!("WARNING: the fast detector failed ({why}); trying gw convert."), Level::Warn);
            self.detect_fallback();
            return true;
        }
        if finished {
            self.detect_finish();
            return true;
        }
        if self.detect.as_ref().is_some_and(|d| d.job.is_none()) {
            self.detect_fill();
        }
        changed
    }

    fn detect_finish(&mut self) {
        let Some(d) = self.detect.take() else { return };
        d.cleanup();
        let result = detect::rank(&d.results);
        let secs = d.elapsed();
        let drive80 = self.s.drive_type(d.drive) == DriveType::Tracks80;
        let summary = result.summary();
        if let Some(best) = result.best() {
            let format = best.format.clone();
            let double = result.double_step && drive80;
            self.apply_detected(d.target, &format, double);
        }
        let level = if result.best().is_some() { Level::Good } else { Level::Warn };
        let message = format!("{summary}  ({secs} s)");
        self.console.push(message.clone(), level);
        self.outcome = Some(Outcome { page: d.target, level, message, hint: None });
        self.detection = Some((d.target, result));
    }

    /// Put a detected format on the page it was asked for.
    pub fn apply_detected(&mut self, target: Page, format: &str, double_step: bool) {
        match target {
            Page::Read => {
                let o = &mut self.s.read;
                o.format = format.to_string();
                o.tracks.custom.clear();
                o.tracks.double_step = double_step;
                widgets::set_extension(&mut o.output, extension_for(format));
            }
            Page::Write => {
                let o = &mut self.s.write;
                o.format = format.to_string();
                o.tracks.custom.clear();
                o.tracks.double_step = double_step;
            }
            Page::Convert => {
                let o = &mut self.s.convert;
                o.format = format.to_string();
                widgets::set_extension(&mut o.output, extension_for(format));
            }
            _ => {}
        }
    }

    /// Stop a running detection.
    pub fn detect_cancel(&mut self) {
        if let Some(d) = self.detect.take() {
            d.cancel();
            d.cleanup();
            self.console.push("Detection stopped.", Level::Warn);
        }
    }

    /// The Detect button and, after a detection, the formats found (click one to use it).
    pub fn detect_controls(&mut self, ui: &mut Ui, target: Page) {
        let busy = self.busy();
        let file = match target {
            Page::Write => Some(self.s.write.input.clone()),
            Page::Convert => Some(self.s.convert.input.clone()),
            _ => None,
        };
        ui.horizontal(|ui| {
            match &file {
                None => {
                    if ui
                        .add_enabled(!busy, egui::Button::new(format!("{}  Detect", ph::MAGIC_WAND)))
                        .on_hover_text(
                            "Reads cylinders 0-2 of the disk as flux, then tries every format gw knows on \
                             cylinder 0. Also spots a 40-track disk in an 80-track drive.",
                        )
                        .clicked()
                    {
                        self.detect_disk(target);
                    }
                }
                Some(path) => {
                    let can = is_flux_image(path);
                    if ui
                        .add_enabled(!busy && can, egui::Button::new(format!("{}  Detect", ph::MAGIC_WAND)))
                        .on_hover_text("Tries every format gw knows on cylinder 0 of this flux image")
                        .on_disabled_hover_text("Detection works on flux images (.scp, .hfe, .raw…)")
                        .clicked()
                    {
                        self.detect_file(target, path.clone());
                    }
                }
            }
            if let Some(d) = &self.detect {
                if d.target == target {
                    ui.spinner();
                    let text = match d.stage {
                        Stage::Reading => "reading cylinders 0-2…".to_string(),
                        Stage::Decoding if d.formats > 0 => format!("{} / {} formats", d.done, d.formats),
                        Stage::Decoding => "starting…".to_string(),
                    };
                    ui.label(RichText::new(text).small().color(theme::MUTED));
                }
            }
        });
        let Some((page, result)) = &self.detection else { return };
        if *page != target {
            return;
        }
        let mut pick: Option<String> = None;
        let mut close = false;
        egui::Frame::new().fill(theme::tint(theme::ACCENT, 16)).corner_radius(6).inner_margin(6).show(
            ui,
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Formats that decode cylinder 0").small().strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        close = ui.small_button(ph::X).on_hover_text("Hide").clicked();
                    });
                });
                if result.ranked.is_empty() {
                    ui.label(RichText::new(result.summary()).small().color(theme::WARN));
                }
                ui.horizontal_wrapped(|ui| {
                    for (i, c) in result.ranked.iter().take(8).enumerate() {
                        let label = format!("{}  {}/{}", c.format, c.found, c.total);
                        let r = ui.selectable_label(i == 0, RichText::new(label).small());
                        if r.on_hover_text("Use this format").clicked() {
                            pick = Some(c.format.clone());
                        }
                    }
                });
                if result.double_step {
                    ui.label(
                        RichText::new("40-track disk in an 80-track drive: double step is needed.")
                            .small()
                            .color(theme::WARN),
                    );
                }
            },
        );
        if let Some(f) = pick {
            let double = result.double_step && self.s.drive_type(self.s.drive) == DriveType::Tracks80;
            self.apply_detected(target, &f, double);
        }
        if close {
            self.detection = None;
        }
    }
}

/// Write the detector script into a private folder (never run from a shared temp folder,
/// where another user's `json.py` would be imported first).
fn write_script() -> std::io::Result<PathBuf> {
    let dir = std::env::temp_dir().join(format!("floppy-copy-detect-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
    }
    let path = dir.join("detect.py");
    std::fs::write(&path, detect::SCRIPT)?;
    Ok(path)
}

/// Results kept for the page that asked.
pub type DetectionShown = (Page, Detection);
