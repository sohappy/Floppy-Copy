//! Floppy-Copy's retro copier screen: an optional full-window interface in the spirit of the classic Amiga
//! disk copiers (metal frames, pixel font, drive lamps, one track grid per side).
//!
//! An original design: no logo, artwork or text of any existing program. Every control drives
//! the same jobs as the classic interface (copy wizard, read, write, format, erase, scan, RPM).

use crate::app::{App, JobExtras, Level, Page};
use crate::copy_ui::Stage;
use crate::theme;
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2,
};
use hgf_core::formats::{self, PRESETS};
use hgf_core::gw::{Action, ActionKind, Drive, EraseOpts, ReadOpts, ReadProfile, TrackOpts, WriteOpts};
use hgf_core::TrackState;
use serde::{Deserialize, Serialize};

/// Design space: everything is laid out on this canvas, then scaled to the window.
const W: f32 = 1280.0;
const H: f32 = 860.0;

const BG: Color32 = Color32::from_rgb(0, 0, 0);
const FRAME_LIGHT: Color32 = Color32::from_rgb(158, 206, 210);
const FRAME_MID: Color32 = Color32::from_rgb(84, 140, 150);
const FRAME_DARK: Color32 = Color32::from_rgb(32, 66, 74);
const LABEL: Color32 = Color32::from_rgb(176, 214, 216);
const VALUE: Color32 = Color32::from_rgb(255, 204, 34);
const GOOD: Color32 = Color32::from_rgb(80, 232, 80);
const BAD: Color32 = Color32::from_rgb(255, 70, 60);
const AMBER: Color32 = Color32::from_rgb(255, 170, 40);
const CYAN: Color32 = Color32::from_rgb(90, 220, 230);
const DIM: Color32 = Color32::from_rgb(70, 92, 96);
const BUTTON: Color32 = Color32::from_rgb(26, 54, 60);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    /// Sector copy, disk to disk.
    #[default]
    DosCopy,
    /// Raw flux copy, disk to disk (keeps copy protections).
    Nibble,
    DiskToImage,
    ImageToDisk,
    Format,
    Erase,
    Check,
    Speed,
}

impl Mode {
    const COPY: [Mode; 2] = [Mode::DosCopy, Mode::Nibble];
    const TOOLS: [Mode; 6] =
        [Mode::DiskToImage, Mode::ImageToDisk, Mode::Format, Mode::Erase, Mode::Check, Mode::Speed];

    fn name(self) -> &'static str {
        match self {
            Mode::DosCopy => "DOSCOPY",
            Mode::Nibble => "NIBBLE",
            Mode::DiskToImage => "DISK>IMAGE",
            Mode::ImageToDisk => "IMAGE>DISK",
            Mode::Format => "FORMAT",
            Mode::Erase => "ERASE",
            Mode::Check => "CHECK",
            Mode::Speed => "SPEED",
        }
    }

    fn help(self) -> &'static str {
        match self {
            Mode::DosCopy => "Copy the decoded sectors from the source drive to the target drive.",
            Mode::Nibble => "Copy the raw magnetic flux (keeps copy protections).",
            Mode::DiskToImage => "Read the source disk into the image file (DEVICE).",
            Mode::ImageToDisk => "Write the image file (DEVICE) to the target disk.",
            Mode::Format => "Write a blank, verified disk of the chosen FORMAT to the target.",
            Mode::Erase => "Erase the tracks of the target disk.",
            Mode::Check => "Read the source disk once to see which tracks are good.",
            Mode::Speed => "Measure the rotation speed of the source drive.",
        }
    }

    /// Modes that write the target disk.
    fn writes(self) -> bool {
        matches!(self, Mode::DosCopy | Mode::Nibble | Mode::ImageToDisk | Mode::Format | Mode::Erase)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    #[default]
    Both,
    Zero,
    One,
}

impl Side {
    fn name(self) -> &'static str {
        match self {
            Side::Both => "BOTH",
            Side::Zero => "0 ONLY",
            Side::One => "1 ONLY",
        }
    }

    fn next(self, back: bool) -> Side {
        let all = [Side::Both, Side::Zero, Side::One];
        cycle(&all, self, back)
    }
}

/// Persisted settings of the retro screen.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RetroSettings {
    /// Show the retro screen instead of the classic interface.
    pub enabled: bool,
    pub copy_mode: Mode,
    pub tool_mode: Mode,
    /// The COPY group is selected (else TOOLS).
    pub copy_group: bool,
    pub start: u16,
    pub end: u16,
    pub side: Side,
    pub format: String,
    pub source: Drive,
    pub target: Drive,
}

impl Default for RetroSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            copy_mode: Mode::DosCopy,
            tool_mode: Mode::DiskToImage,
            copy_group: true,
            start: 0,
            end: 79,
            side: Side::Both,
            format: "amiga.amigados".into(),
            source: Drive::A,
            target: Drive::A,
        }
    }
}

impl RetroSettings {
    fn mode(&self) -> Mode {
        if self.copy_group {
            self.copy_mode
        } else {
            self.tool_mode
        }
    }

    fn last_cyl(&self) -> u16 {
        PRESETS.iter().find(|p| p.format == self.format).map(|p| p.cyls.saturating_sub(1)).unwrap_or(79)
    }

    /// `--tracks` for START/END/SIDE; empty when it is the whole disk.
    fn tracks(&self) -> String {
        let whole = self.start == 0 && self.end >= self.last_cyl();
        let heads = match self.side {
            Side::Both => "",
            Side::Zero => ":h=0",
            Side::One => ":h=1",
        };
        if whole && heads.is_empty() {
            // The preset's own tracks (e.g. one side, 40 cylinders for the CPC).
            PRESETS.iter().find(|p| p.format == self.format).map(|p| p.tracks.to_string()).unwrap_or_default()
        } else {
            format!("c={}-{}{heads}", self.start, self.end)
        }
    }
}

/// Non-persisted state.
#[derive(Default)]
pub struct RetroState {
    /// Last mode started, for REPEAT.
    last: Option<Mode>,
    /// DIRECTORY overlay: title and lines.
    directory: Option<(String, Vec<String>)>,
    /// Message of the retro screen itself (shown on the status line).
    message: Option<(String, Color32)>,
}

fn cycle<T: Copy + PartialEq>(all: &[T], cur: T, back: bool) -> T {
    let i = all.iter().position(|&x| x == cur).unwrap_or(0);
    let n = all.len();
    all[if back { (i + n - 1) % n } else { (i + 1) % n }]
}

/// Short name of a format for the FORMAT column.
fn format_short(format: &str) -> &'static str {
    match format {
        "amiga.amigados" => "AMIGA DD",
        "amiga.amigados_hd" => "AMIGA HD",
        "atarist.720" => "ST 720K",
        "atarist.360" => "ST 360K",
        "ibm.1440" => "PC 1.44M",
        "ibm.720" => "PC 720K",
        "ibm.1200" => "PC 1.2M",
        "ibm.360" => "PC 360K",
        "commodore.1541" => "C64 1541",
        "commodore.1571" => "C128 1571",
        "commodore.1581" => "CBM 1581",
        "apple2.appledos.140" => "APPLE II",
        "mac.800" => "MAC 800K",
        "acorn.dfs.ss" => "BBC DFS",
        "ibm.scan" => "CPC/+3",
        "msx.2dd" => "MSX 2DD",
        "" => "FLUX",
        _ => "OTHER",
    }
}

fn drive_name(d: Drive) -> &'static str {
    match d {
        Drive::A => "A",
        Drive::B => "B",
        Drive::Unit0 => "0",
        Drive::Unit1 => "1",
        Drive::Unit2 => "2",
        Drive::Unit3 => "3",
    }
}

const DRIVES: [Drive; 4] = [Drive::A, Drive::B, Drive::Unit0, Drive::Unit1];

/// Maps the design canvas to the screen.
#[derive(Clone, Copy)]
struct Canvas {
    origin: Pos2,
    k: f32,
}

impl Canvas {
    fn p(&self, x: f32, y: f32) -> Pos2 {
        self.origin + Vec2::new(x, y) * self.k
    }
    fn r(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(self.p(x, y), Vec2::new(w, h) * self.k)
    }
    fn font(&self, size: f32) -> FontId {
        FontId::new((size * self.k).max(6.0), theme::retro_family())
    }
}

impl App {
    /// The whole retro screen (replaces the classic layout while enabled).
    pub fn retro_screen(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        ui.painter().rect_filled(full, CornerRadius::ZERO, BG);
        let k = (full.width() / W).min(full.height() / H);
        let origin = full.center() - Vec2::new(W, H) * k / 2.0;
        let cv = Canvas { origin, k };
        let painter = ui.painter().clone();

        self.retro_header(ui, &painter, cv);
        self.retro_functions(ui, &painter, cv);
        self.retro_buttons(ui, &painter, cv);
        self.retro_drives(ui, &painter, cv);
        self.retro_grid(ui, &painter, cv, 0, 552.0);
        self.retro_grid(ui, &painter, cv, 1, 906.0);
        self.retro_status(&painter, cv);
        self.retro_directory(ui, &painter, cv);

        if ui.input(|i| i.key_pressed(egui::Key::Escape)) && self.confirm.is_none() {
            if self.retro.directory.is_some() {
                self.retro.directory = None;
            } else if self.busy() {
                self.cancel();
            }
        }
    }

    // ───────────────────────────── drawing helpers ─────────────────────────────

    fn retro_interact(ui: &Ui, rect: Rect, id: &str) -> egui::Response {
        ui.interact(rect, ui.id().with(("retro", id)), Sense::click())
    }

    // ───────────────────────────── header ─────────────────────────────

    fn retro_header(&self, _ui: &mut Ui, p: &egui::Painter, cv: Canvas) {
        // Three riveted plates.
        for (x, w) in [(40.0, 240.0), (290.0, 700.0), (1000.0, 240.0)] {
            plate(p, cv, cv.r(x, 30.0, w, 170.0));
        }
        // Left plate: version.
        let left = cv.r(70.0, 58.0, 180.0, 114.0);
        p.rect_filled(left, CornerRadius::ZERO, BG);
        bevel(p, left, cv.k, FRAME_DARK, FRAME_LIGHT);
        // Left plate: copyright and version.
        p.text(cv.p(160.0, 82.0), Align2::CENTER_CENTER, "(C) 2026", cv.font(22.0), LABEL);
        p.text(cv.p(160.0, 118.0), Align2::CENTER_CENTER, "RCC(A)", cv.font(34.0), AMBER);
        p.text(
            cv.p(160.0, 154.0),
            Align2::CENTER_CENTER,
            format!("V{}", hgf_core::VERSION),
            cv.font(16.0),
            LABEL,
        );
        // Centre plate: perforated metal and the embossed title.
        let center = cv.r(318.0, 52.0, 644.0, 126.0);
        p.rect_filled(center, CornerRadius::ZERO, Color32::from_rgb(10, 18, 20));
        let step = 9.0;
        let mut y = 56.0;
        while y < 176.0 {
            let mut x = 322.0;
            while x < 960.0 {
                p.circle_filled(cv.p(x, y), 1.4 * cv.k, Color32::from_rgb(54, 82, 88));
                x += step;
            }
            y += step;
        }
        embossed(p, cv, Pos2::new(640.0, 104.0), "FLOPPY-COPY", 60.0);
        // Subtitle on a plate of its own, cut into the perforated metal.
        let sub = cv.r(470.0, 146.0, 340.0, 26.0);
        p.rect_filled(sub, CornerRadius::ZERO, BG);
        bevel(p, sub, cv.k, FRAME_DARK, FRAME_LIGHT);
        p.text(sub.center(), Align2::CENTER_CENTER, "RETRO DISK COPIER", cv.font(17.0), CYAN);
        // Right plate: the device.
        let right = cv.r(1030.0, 58.0, 180.0, 114.0);
        p.rect_filled(right, CornerRadius::ZERO, BG);
        bevel(p, right, cv.k, FRAME_DARK, FRAME_LIGHT);
        let field = |k: &str| {
            self.device.fields.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone()).unwrap_or_default()
        };
        let model = field("Model").to_uppercase().replace("GREASEWEAZLE ", "GW ");
        let lines = match self.device.state {
            crate::app::DeviceState::Connected => vec![
                if model.is_empty() { "CONNECTED".to_string() } else { model },
                format!("FW {}", field("Firmware")),
                field("Port").rsplit(['/', '\\']).next().unwrap_or_default().to_uppercase(),
            ],
            crate::app::DeviceState::Probing => vec!["DETECTING".into(), "…".into()],
            crate::app::DeviceState::NoTools => {
                vec!["NO GW TOOLS".into(), "SEE CLASSIC".into(), "SETTINGS".into()]
            }
            _ => vec!["NO DEVICE".into(), "PLUG IN".into(), "THE GW".into()],
        };
        for (i, l) in lines.iter().enumerate() {
            p.text(cv.p(1120.0, 84.0 + i as f32 * 30.0), Align2::CENTER_CENTER, l, cv.font(16.0), LABEL);
        }
    }

    // ───────────────────────────── function panel ─────────────────────────────

    fn retro_functions(&mut self, ui: &mut Ui, p: &egui::Painter, cv: Canvas) {
        let panel = cv.r(40.0, 212.0, 1200.0, 134.0);
        p.rect_filled(panel, CornerRadius::ZERO, BG);
        bevel(p, panel, cv.k, FRAME_LIGHT, FRAME_DARK);
        bevel(p, panel.shrink(4.0 * cv.k), cv.k, FRAME_DARK, FRAME_LIGHT);
        p.line_segment([cv.p(56.0, 262.0), cv.p(1224.0, 262.0)], Stroke::new(2.0 * cv.k, FRAME_MID));
        let busy = self.busy() || self.copy.active();
        let s = &self.s.retro;
        let mode = s.mode();

        // COPY / TOOLS lamps.
        for (x, label, on) in [(64.0, "COPY", s.copy_group), (200.0, "TOOLS", !s.copy_group)] {
            lamp_dot(p, cv, Pos2::new(x + 8.0, 240.0), on);
            p.text(cv.p(x + 24.0, 240.0), Align2::LEFT_CENTER, label, cv.font(20.0), LABEL);
            let hit = cv.r(x - 4.0, 226.0, 130.0, 28.0);
            if !busy && Self::retro_interact(ui, hit, label).clicked() {
                self.s.retro.copy_group = label == "COPY";
            }
        }
        // Mode (click: next, right click: previous).
        let mode_rect = cv.r(60.0, 278.0, 300.0, 50.0);
        p.text(cv.p(64.0, 303.0), Align2::LEFT_CENTER, mode.name(), cv.font(32.0), VALUE);
        let r = Self::retro_interact(ui, mode_rect, "mode").on_hover_text(mode.help());
        if !busy && (r.clicked() || r.secondary_clicked()) {
            let back = r.secondary_clicked();
            let s = &mut self.s.retro;
            if s.copy_group {
                s.copy_mode = cycle(&Mode::COPY, s.copy_mode, back);
            } else {
                s.tool_mode = cycle(&Mode::TOOLS, s.tool_mode, back);
            }
        }

        // START / END cylinders with arrows over each digit.
        let last = self.s.retro.last_cyl();
        for (x, label, is_start) in [(410.0, "START", true), (540.0, "END", false)] {
            p.text(cv.p(x + 24.0, 240.0), Align2::CENTER_CENTER, label, cv.font(20.0), LABEL);
            let v = if is_start { self.s.retro.start } else { self.s.retro.end };
            p.text(cv.p(x + 24.0, 303.0), Align2::CENTER_CENTER, format!("{v:02}"), cv.font(32.0), VALUE);
            for (dx, step) in [(10.0, 10i32), (38.0, 1)] {
                for (dy, dir) in [(272.0, 1i32), (334.0, -1)] {
                    let c = Pos2::new(x + dx, dy);
                    arrow(p, cv, c, dir > 0, if busy { DIM } else { FRAME_LIGHT });
                    let hit = cv.r(c.x - 12.0, c.y - 10.0, 24.0, 20.0);
                    if !busy && Self::retro_interact(ui, hit, &format!("{label}{dx}{dy}")).clicked() {
                        let s = &mut self.s.retro;
                        let cur = if is_start { s.start } else { s.end } as i32;
                        let nv = (cur + dir * step).clamp(0, last as i32) as u16;
                        if is_start {
                            s.start = nv.min(s.end);
                        } else {
                            s.end = nv.max(s.start);
                        }
                    }
                }
            }
        }

        // SIDE, FORMAT, DEVICE: click to change.
        let device = match mode {
            Mode::DiskToImage => file_label(&self.s.read.output),
            Mode::ImageToDisk => file_label(&self.s.write.input),
            _ => "DISK".to_string(),
        };
        let cols = [
            (680.0, "SIDE", self.s.retro.side.name().to_string()),
            (840.0, "FORMAT", format_short(&self.s.retro.format).to_string()),
            (1066.0, "DEVICE", device),
        ];
        for (x, label, value) in cols {
            lamp_dot(p, cv, Pos2::new(x + 8.0, 240.0), false);
            p.text(cv.p(x + 24.0, 240.0), Align2::LEFT_CENTER, label, cv.font(20.0), LABEL);
            let size = if label == "DEVICE" { 22.0 } else { 26.0 };
            p.text(cv.p(x, 303.0), Align2::LEFT_CENTER, &value, cv.font(size), VALUE);
            let hit = cv.r(x - 6.0, 226.0, if label == "DEVICE" { 164.0 } else { 210.0 }, 104.0);
            let tip = match label {
                "SIDE" => "Both sides, or one side only",
                "FORMAT" => "Disk format (click: next, right click: previous)",
                _ => "Image file for DISK>IMAGE and IMAGE>DISK (click to choose)",
            };
            let r = Self::retro_interact(ui, hit, label).on_hover_text(tip);
            if busy || !(r.clicked() || r.secondary_clicked()) {
                continue;
            }
            let back = r.secondary_clicked();
            match label {
                "SIDE" => self.s.retro.side = self.s.retro.side.next(back),
                "FORMAT" => {
                    let mut all: Vec<&str> = Vec::new();
                    for p in PRESETS {
                        // Flux presets share the empty format: one entry.
                        if !all.contains(&p.format) {
                            all.push(p.format);
                        }
                    }
                    let s = &mut self.s.retro;
                    s.format = cycle(&all, s.format.as_str(), back).to_string();
                    s.start = 0;
                    s.end = s.last_cyl();
                }
                _ => self.retro_pick_file(mode),
            }
        }
    }

    fn retro_pick_file(&mut self, mode: Mode) {
        let mut d = rfd::FileDialog::new();
        if let Some(dir) = &self.s.last_dir {
            d = d.set_directory(dir);
        }
        match mode {
            Mode::DiskToImage => {
                let ext = PRESETS
                    .iter()
                    .find(|p| p.format == self.s.retro.format)
                    .map(|p| p.extension)
                    .unwrap_or("scp");
                if let Some(p) = d.set_file_name(format!("disk.{ext}")).save_file() {
                    self.s.last_dir = p.parent().map(|d| d.to_path_buf());
                    self.s.read.output = p;
                }
            }
            Mode::ImageToDisk => {
                if let Some(p) = d.pick_file() {
                    self.s.last_dir = p.parent().map(|d| d.to_path_buf());
                    let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
                    if let Some(f) = formats::guess_format(&p, size) {
                        self.s.retro.format = f.to_string();
                        self.s.retro.end = self.s.retro.last_cyl();
                    }
                    self.set_write_input(p);
                }
            }
            _ => {}
        }
    }

    // ───────────────────────────── buttons ─────────────────────────────

    fn retro_buttons(&mut self, ui: &mut Ui, p: &egui::Painter, cv: Canvas) {
        let busy = self.busy();
        let buttons: [(&str, Color32, bool); 8] = [
            ("START", VALUE, !busy),
            ("REPEAT", VALUE, !busy && self.retro.last.is_some()),
            ("STOP", VALUE, busy || self.copy.active()),
            ("DRIVE INFO", GOOD, !busy),
            ("DIRECTORY", GOOD, true),
            ("CHECKDISK", GOOD, !busy),
            ("DEFAULT", BAD, !busy),
            ("CLASSIC", CYAN, true),
        ];
        let (x0, w, gap) = (40.0, 143.0, 8.4);
        for (i, (label, color, enabled)) in buttons.into_iter().enumerate() {
            let rect = cv.r(x0 + i as f32 * (w + gap), 356.0, w, 40.0);
            let r = Self::retro_interact(ui, rect, label);
            let down = enabled && r.is_pointer_button_down_on();
            p.rect_filled(rect, CornerRadius::ZERO, if r.hovered() && enabled { FRAME_DARK } else { BUTTON });
            if down {
                bevel(p, rect, cv.k, FRAME_DARK, FRAME_LIGHT);
            } else {
                bevel(p, rect, cv.k, FRAME_LIGHT, FRAME_DARK);
            }
            p.text(
                rect.center(),
                Align2::CENTER_CENTER,
                label,
                cv.font(17.0),
                if enabled { color } else { DIM },
            );
            if enabled && r.clicked() {
                self.retro_button(label);
            }
        }
    }

    fn retro_button(&mut self, label: &str) {
        self.retro.message = None;
        match label {
            "START" => match self.copy.stage {
                // The copy wizard waits for the user.
                Stage::Review => self.copy_after_read(),
                Stage::WaitDestination => self.copy_write(),
                _ => {
                    let m = self.s.retro.mode();
                    self.retro_start(m);
                }
            },
            "REPEAT" => {
                if let Some(m) = self.retro.last {
                    self.retro_start(m);
                }
            }
            "STOP" => {
                if self.busy() {
                    self.cancel();
                } else {
                    self.copy_abort("Stopped by user.");
                }
            }
            "DRIVE INFO" => self.request(Action::Info { bootloader: false }),
            "DIRECTORY" => self.retro_open_directory(),
            "CHECKDISK" => self.retro_start(Mode::Check),
            "DEFAULT" => {
                self.s.retro = crate::retro::RetroSettings { enabled: true, ..Default::default() };
                self.retro.message = Some(("Default settings restored.".into(), GOOD));
            }
            "CLASSIC" => self.s.retro.enabled = false,
            _ => {}
        }
    }

    fn retro_start(&mut self, mode: Mode) {
        let s = self.s.retro.clone();
        let tracks = TrackOpts { custom: s.tracks(), ..Default::default() };
        if mode.writes()
            && s.format.is_empty()
            && !matches!(mode, Mode::Nibble | Mode::ImageToDisk | Mode::Erase)
        {
            self.retro.message = Some(("Choose a FORMAT first.".into(), BAD));
            return;
        }
        self.retro.last = Some(mode);
        self.retro.directory = None;
        match mode {
            Mode::DosCopy | Mode::Nibble => {
                self.s.copy.format = s.format.clone();
                self.s.copy.flux = mode == Mode::Nibble;
                self.s.copy.source = s.source;
                self.s.copy.destination = s.target;
                self.s.copy.recovery = false;
                self.s.copy.tracks = tracks.custom;
                self.copy_start();
            }
            Mode::DiskToImage => {
                if self.s.read.output.as_os_str().is_empty() {
                    self.retro_pick_file(mode);
                    if self.s.read.output.as_os_str().is_empty() {
                        return;
                    }
                }
                let read = ReadOpts {
                    output: self.s.read.output.clone(),
                    format: s.format.clone(),
                    tracks,
                    ..Default::default()
                };
                self.next = JobExtras { drive: Some(s.source), page: Some(Page::Read), ..Default::default() };
                self.request(Action::Read(read));
            }
            Mode::ImageToDisk => {
                if self.s.write.input.as_os_str().is_empty() {
                    self.retro_pick_file(mode);
                    if self.s.write.input.as_os_str().is_empty() {
                        return;
                    }
                }
                let write = WriteOpts {
                    input: self.s.write.input.clone(),
                    format: s.format.clone(),
                    tracks,
                    ..Default::default()
                };
                self.next =
                    JobExtras { drive: Some(s.target), page: Some(Page::Write), ..Default::default() };
                self.request(Action::Write(write));
            }
            Mode::Format => {
                if !self.select_formatter(&s.format) {
                    self.retro.message =
                        Some((format!("No blank-disk builder for {}.", format_short(&s.format)), BAD));
                    return;
                }
                self.format_disk(Some(s.target));
            }
            Mode::Erase => {
                self.next =
                    JobExtras { drive: Some(s.target), page: Some(Page::Erase), ..Default::default() };
                self.request(Action::Erase(EraseOpts { tracks, ..Default::default() }));
            }
            Mode::Check => {
                let read = ReadOpts {
                    format: s.format.clone(),
                    profile: ReadProfile::QuickScan,
                    tracks,
                    ..Default::default()
                };
                self.next = JobExtras { drive: Some(s.source), page: Some(Page::Read), ..Default::default() };
                self.request(Action::Read(read));
            }
            Mode::Speed => {
                self.next = JobExtras { drive: Some(s.source), page: Some(Page::Rpm), ..Default::default() };
                self.request(Action::Rpm { nr: self.s.rpm_nr.max(5) });
            }
        }
    }

    // ───────────────────────────── drives ─────────────────────────────

    fn retro_drives(&mut self, ui: &mut Ui, p: &egui::Painter, cv: Canvas) {
        p.line_segment([cv.p(40.0, 436.0), cv.p(1240.0, 436.0)], Stroke::new(2.0 * cv.k, FRAME_MID));
        p.text(cv.p(285.0, 420.0), Align2::CENTER_CENTER, "SOURCE DRIVES", cv.font(20.0), LABEL);
        p.text(cv.p(729.0, 420.0), Align2::CENTER_CENTER, "SIDE 0", cv.font(20.0), LABEL);
        p.text(cv.p(1083.0, 420.0), Align2::CENTER_CENTER, "SIDE 1", cv.font(20.0), LABEL);
        p.line_segment([cv.p(40.0, 708.0), cv.p(530.0, 708.0)], Stroke::new(2.0 * cv.k, FRAME_MID));
        p.text(cv.p(285.0, 726.0), Align2::CENTER_CENTER, "TARGET DRIVES", cv.font(20.0), LABEL);
        let busy = self.busy() || self.copy.active();
        let running = self.running().map(|(k, _)| k);
        for (i, d) in DRIVES.into_iter().enumerate() {
            let x = 60.0 + i as f32 * 120.0;
            let src = self.s.retro.source == d;
            let tgt = self.s.retro.target == d;
            bulb(p, cv, Pos2::new(x + 50.0, 470.0), src, false);
            bulb(p, cv, Pos2::new(x + 50.0, 660.0), tgt, true);
            let active = running.is_some() && (self.running_drive() == Some(d));
            disk_icon(p, cv, cv.r(x, 512.0, 100.0, 104.0), drive_name(d), src, tgt, active);
            let top = cv.r(x + 20.0, 440.0, 60.0, 70.0);
            let bottom = cv.r(x + 20.0, 628.0, 60.0, 70.0);
            let icon = cv.r(x, 512.0, 100.0, 104.0);
            let name = d.label();
            if !busy
                && Self::retro_interact(ui, top, &format!("src{i}"))
                    .on_hover_text(format!("Source: {name}"))
                    .clicked()
            {
                self.s.retro.source = d;
            }
            if !busy
                && Self::retro_interact(ui, bottom, &format!("tgt{i}"))
                    .on_hover_text(format!("Target: {name}"))
                    .clicked()
            {
                self.s.retro.target = d;
            }
            let r = Self::retro_interact(ui, icon, &format!("icon{i}"))
                .on_hover_text(format!("{name}\nClick: source  ·  Right click: target"));
            if !busy && r.clicked() {
                self.s.retro.source = d;
            }
            if !busy && r.secondary_clicked() {
                self.s.retro.target = d;
            }
        }
    }

    /// Drive used by the running job (source while reading, target while writing).
    fn running_drive(&self) -> Option<Drive> {
        let (kind, _) = self.running()?;
        Some(match kind {
            ActionKind::Write | ActionKind::Erase => self.s.retro.target,
            _ => self.s.retro.source,
        })
    }

    // ───────────────────────────── track grids ─────────────────────────────

    fn retro_grid(&mut self, ui: &mut Ui, p: &egui::Painter, cv: Canvas, head: u8, x0: f32) {
        let (cw, ch, y0) = (31.0, 30.0, 452.0);
        let gx = x0 + 22.0;
        for c in 0..10 {
            p.text(
                cv.p(gx + c as f32 * cw + cw / 2.0, y0),
                Align2::CENTER_CENTER,
                c.to_string(),
                cv.font(16.0),
                LABEL,
            );
        }
        let side_ok = match self.s.retro.side {
            Side::Both => true,
            Side::Zero => head == 0,
            Side::One => head == 1,
        };
        let geometry = &self.map.geometry;
        for row in 0..9u16 {
            let y = y0 + 14.0 + row as f32 * ch;
            p.text(
                cv.p(x0 + 8.0, y + ch / 2.0),
                Align2::CENTER_CENTER,
                row.to_string(),
                cv.font(16.0),
                LABEL,
            );
            let cols = if row == 8 { 4 } else { 10 };
            for c in 0..cols {
                let cyl = row * 10 + c;
                let rect = cv.r(gx + c as f32 * cw, y, cw - 2.0, ch - 2.0);
                let in_geo = geometry.cyls.contains(&cyl) && geometry.heads.contains(&head);
                let in_range = side_ok && cyl >= self.s.retro.start && cyl <= self.s.retro.end;
                let info = self.map.get(cyl, head);
                let active = self.map.active == Some((cyl, head));
                let (ch_txt, color) = cell(info.state, info.sectors);
                let border = if in_range { FRAME_MID } else { FRAME_DARK };
                p.rect_stroke(rect, CornerRadius::ZERO, Stroke::new(1.5 * cv.k, border), StrokeKind::Inside);
                if active {
                    p.rect_filled(rect.shrink(2.0 * cv.k), CornerRadius::ZERO, VALUE);
                    p.text(rect.center(), Align2::CENTER_CENTER, "*", cv.font(18.0), BG);
                } else if in_geo || info.state != TrackState::Pending {
                    p.text(rect.center(), Align2::CENTER_CENTER, ch_txt, cv.font(18.0), color);
                }
                let r = Self::retro_interact(ui, rect, &format!("cell{head}-{cyl}"));
                if r.hovered() {
                    let sectors = info.sectors.map(|(a, b)| format!("\n{a}/{b} sectors")).unwrap_or_default();
                    let retries =
                        if info.retries > 0 { format!("\n{} retries", info.retries) } else { String::new() };
                    r.on_hover_text(format!("Track {cyl}.{head}: {}{sectors}{retries}", info.state.label()));
                }
            }
            if row == 8 {
                // Information box at the end of the last row.
                let rect = cv.r(gx + 4.0 * cw, y, 6.0 * cw - 2.0, ch - 2.0);
                p.rect_stroke(
                    rect,
                    CornerRadius::ZERO,
                    Stroke::new(1.5 * cv.k, FRAME_DARK),
                    StrokeKind::Inside,
                );
                let text = if head == 0 {
                    let c = self.map.counts();
                    format!("OK {}  BAD {}", c.ok + c.recovered, c.bad)
                } else {
                    let t = self.running().map(|(_, t)| t).or(self.last_duration()).unwrap_or_default();
                    format!("TIME: {:02}:{:02}", t.as_secs() / 60, t.as_secs() % 60)
                };
                p.text(rect.center(), Align2::CENTER_CENTER, text, cv.font(16.0), LABEL);
            }
        }
    }

    // ───────────────────────────── status line ─────────────────────────────

    fn retro_status(&self, p: &egui::Painter, cv: Canvas) {
        let rect = cv.r(40.0, 752.0, 1200.0, 74.0);
        p.rect_filled(rect, CornerRadius::ZERO, BG);
        bevel(p, rect, cv.k, FRAME_LIGHT, FRAME_DARK);
        bevel(p, rect.shrink(4.0 * cv.k), cv.k, FRAME_DARK, FRAME_LIGHT);
        let (text, color) = self.retro_status_text();
        let mut t = text.replace('\n', " ");
        let max = 52;
        if t.chars().count() > max {
            t = t.chars().take(max - 1).collect::<String>() + "…";
        }
        p.text(cv.p(64.0, 789.0), Align2::LEFT_CENTER, t, cv.font(26.0), color);
    }

    fn retro_status_text(&self) -> (String, Color32) {
        if let Some((kind, _)) = self.running() {
            let what = match kind {
                ActionKind::Read if self.copy.active() => "Reading source",
                ActionKind::Write if self.copy.active() => "Writing copy",
                ActionKind::Read => "Reading",
                ActionKind::Write => "Writing",
                ActionKind::Erase => "Erasing",
                ActionKind::Rpm => "Measuring speed",
                ActionKind::Info => "Asking the Greaseweazle",
                _ => "Working",
            };
            let track = self.map.active.map(|(c, h)| format!(" track {c}.{h}")).unwrap_or_default();
            let pct = (self.map.progress() * 100.0).round();
            let pct = if pct > 0.0 && kind.uses_disk_map() { format!("  {pct:.0}%") } else { String::new() };
            return (format!("{what}{track}{pct}..."), CYAN);
        }
        if self.confirm.is_some() {
            return ("Confirm in the window to go on.".into(), AMBER);
        }
        match &self.copy.stage {
            Stage::Review => {
                return (format!("{} bad tracks. START: copy anyway, STOP: cancel.", self.copy.bad()), AMBER)
            }
            Stage::WaitDestination => {
                return (
                    format!(
                        "Insert the target disk in drive {}, then START.",
                        drive_name(self.copy.destination())
                    ),
                    AMBER,
                )
            }
            _ => {}
        }
        if let Some((m, c)) = &self.retro.message {
            return (m.clone(), *c);
        }
        if let Some(n) = &self.notice {
            return (n.clone(), BAD);
        }
        if let Stage::Done(ok) = self.copy.stage {
            if let Some(m) = self.copy.message() {
                return (m.to_string(), if ok { GOOD } else { BAD });
            }
        }
        if let Stage::Aborted(reason) = &self.copy.stage {
            return (format!("Copy stopped: {reason}"), AMBER);
        }
        if let Some(o) = &self.outcome {
            let c = match o.level {
                Level::Error => BAD,
                Level::Warn => AMBER,
                _ => GOOD,
            };
            let msg = match (o.level, o.page) {
                // The automatic device detection at start-up.
                (Level::Good, Page::Info) => "Greaseweazle ready. Choose a mode, then START.".to_string(),
                (Level::Good, _) => "Operation done.".to_string(),
                _ => o.message.clone(),
            };
            return (msg, c);
        }
        ("Ready. Choose a mode, the drives, then START.".into(), GOOD)
    }

    // ───────────────────────────── directory ─────────────────────────────

    fn retro_open_directory(&mut self) {
        let path = match self.s.retro.mode() {
            Mode::ImageToDisk => self.s.write.input.clone(),
            _ => self.s.read.output.clone(),
        };
        if !path.is_file() {
            self.retro.message = Some(("No image to list: read a disk to an ADF first.".into(), AMBER));
            return;
        }
        match hgf_core::image::inspect(&path) {
            Ok(rep) => match rep.adf {
                Some(Ok(info)) => {
                    let mut lines: Vec<String> = info
                        .entries
                        .iter()
                        .map(|e| {
                            if e.is_dir {
                                format!("{:<24} (DIR)", e.name)
                            } else {
                                format!("{:<24} {:>7}", e.name, e.size)
                            }
                        })
                        .collect();
                    if lines.is_empty() {
                        lines.push("(empty)".into());
                    }
                    let title = format!(
                        "{}  ({}{})",
                        info.volume,
                        info.filesystem,
                        if info.bootable { ", bootable" } else { "" }
                    );
                    self.retro.directory = Some((title, lines));
                }
                Some(Err(e)) => self.retro.message = Some((e, AMBER)),
                None => self.retro.message = Some(("DIRECTORY reads AmigaDOS (.adf) images.".into(), AMBER)),
            },
            Err(e) => self.retro.message = Some((e, BAD)),
        }
    }

    fn retro_directory(&mut self, ui: &mut Ui, p: &egui::Painter, cv: Canvas) {
        let Some((title, lines)) = &self.retro.directory else { return };
        let rect = cv.r(330.0, 440.0, 620.0, 300.0);
        p.rect_filled(rect, CornerRadius::ZERO, BG);
        bevel(p, rect, cv.k, FRAME_LIGHT, FRAME_DARK);
        p.text(cv.p(350.0, 462.0), Align2::LEFT_CENTER, title, cv.font(18.0), VALUE);
        for (i, l) in lines.iter().take(11).enumerate() {
            p.text(
                cv.p(350.0, 492.0 + i as f32 * 20.0),
                Align2::LEFT_TOP,
                l,
                FontId::monospace(14.0 * cv.k),
                GOOD,
            );
        }
        if lines.len() > 11 {
            p.text(
                cv.p(350.0, 716.0),
                Align2::LEFT_CENTER,
                format!("… {} more", lines.len() - 11),
                cv.font(14.0),
                LABEL,
            );
        }
        p.text(cv.p(930.0, 462.0), Align2::RIGHT_CENTER, "X", cv.font(18.0), BAD);
        if Self::retro_interact(ui, rect, "dir").clicked() {
            self.retro.directory = None;
        }
    }
}

// ───────────────────────────── shapes ─────────────────────────────

/// Symbol and colour of a track cell: "0" good, a digit = missing sectors, R recovered…
fn cell(state: TrackState, sectors: Option<(u32, u32)>) -> (String, Color32) {
    let missing = sectors.map(|(a, b)| b.saturating_sub(a)).unwrap_or(0);
    match state {
        TrackState::Pending => (String::new(), DIM),
        TrackState::Active => ("*".into(), VALUE),
        TrackState::Ok => ("0".into(), GOOD),
        TrackState::Recovered => ("R".into(), CYAN),
        TrackState::Retrying => ("?".into(), AMBER),
        TrackState::Bad => (if missing > 0 { missing.min(9).to_string() } else { "X".into() }, BAD),
        TrackState::Empty => ("-".into(), DIM),
        TrackState::Flux => ("F".into(), CYAN),
        TrackState::OutOfRange => (String::new(), DIM),
    }
}

fn file_label(path: &std::path::Path) -> String {
    let name = path.file_name().map(|n| n.to_string_lossy().to_uppercase()).unwrap_or_default();
    if name.is_empty() {
        return "CHOOSE...".into();
    }
    if name.chars().count() > 9 {
        name.chars().take(7).collect::<String>() + "…"
    } else {
        name
    }
}

/// Raised (light, dark) or sunken (dark, light) bevel around a rectangle.
fn bevel(p: &egui::Painter, r: Rect, k: f32, top_left: Color32, bottom_right: Color32) {
    let w = (2.0 * k).max(1.0);
    p.line_segment([r.left_top(), r.right_top()], Stroke::new(w, top_left));
    p.line_segment([r.left_top(), r.left_bottom()], Stroke::new(w, top_left));
    p.line_segment([r.left_bottom(), r.right_bottom()], Stroke::new(w, bottom_right));
    p.line_segment([r.right_top(), r.right_bottom()], Stroke::new(w, bottom_right));
}

/// A riveted metal plate.
fn plate(p: &egui::Painter, cv: Canvas, r: Rect) {
    p.rect_filled(r, CornerRadius::ZERO, Color32::from_rgb(46, 80, 88));
    bevel(p, r, cv.k, FRAME_LIGHT, FRAME_DARK);
    let spacing = 22.0 * cv.k;
    let inset = 12.0 * cv.k;
    let rivet = |c: Pos2| {
        p.circle_filled(c, 6.5 * cv.k, FRAME_DARK);
        p.circle_filled(c, 5.0 * cv.k, Color32::from_rgb(120, 166, 172));
        p.circle_filled(c - Vec2::splat(1.6 * cv.k), 2.0 * cv.k, Color32::from_rgb(220, 240, 240));
    };
    let mut x = r.left() + inset;
    while x <= r.right() - inset + 0.5 {
        rivet(Pos2::new(x, r.top() + inset));
        rivet(Pos2::new(x, r.bottom() - inset));
        x += spacing;
    }
    let mut y = r.top() + inset + spacing;
    while y < r.bottom() - inset - spacing / 2.0 {
        rivet(Pos2::new(r.left() + inset, y));
        rivet(Pos2::new(r.right() - inset, y));
        y += spacing;
    }
}

/// Embossed metal title text.
fn embossed(p: &egui::Painter, cv: Canvas, at: Pos2, text: &str, size: f32) {
    let f = cv.font(size);
    let c = cv.p(at.x, at.y);
    let o = 3.0 * cv.k;
    for d in [Vec2::new(o, o), Vec2::new(o * 1.6, o * 1.6)] {
        p.text(c + d, Align2::CENTER_CENTER, text, f.clone(), Color32::from_rgb(8, 16, 18));
    }
    for d in [Vec2::new(-1.0, 0.0), Vec2::new(1.0, 0.0), Vec2::new(0.0, -1.0), Vec2::new(0.0, 1.0)] {
        p.text(c + d * 2.0 * cv.k, Align2::CENTER_CENTER, text, f.clone(), Color32::from_rgb(40, 70, 78));
    }
    p.text(
        c - Vec2::splat(1.2 * cv.k),
        Align2::CENTER_CENTER,
        text,
        f.clone(),
        Color32::from_rgb(236, 250, 250),
    );
    p.text(c, Align2::CENTER_CENTER, text, f, Color32::from_rgb(160, 196, 202));
}

/// Small round indicator in front of a label.
fn lamp_dot(p: &egui::Painter, cv: Canvas, at: Pos2, on: bool) {
    let c = cv.p(at.x, at.y);
    p.circle_filled(c, 8.0 * cv.k, if on { Color32::from_rgb(200, 60, 10) } else { FRAME_DARK });
    p.circle_filled(c, 6.0 * cv.k, if on { AMBER } else { DIM });
    p.circle_filled(
        c - Vec2::splat(2.0 * cv.k),
        2.2 * cv.k,
        if on { Color32::from_rgb(255, 240, 170) } else { LABEL },
    );
}

/// Up or down arrow head.
fn arrow(p: &egui::Painter, cv: Canvas, at: Pos2, up: bool, color: Color32) {
    let c = cv.p(at.x, at.y);
    let (w, h) = (9.0 * cv.k, 7.0 * cv.k);
    let pts = if up {
        vec![c + Vec2::new(-w, h), c + Vec2::new(w, h), c + Vec2::new(0.0, -h)]
    } else {
        vec![c + Vec2::new(-w, -h), c + Vec2::new(w, -h), c + Vec2::new(0.0, h)]
    };
    p.add(egui::Shape::convex_polygon(pts, color, Stroke::NONE));
}

/// Light bulb: lit when the drive is chosen. `below`: the bulb points up (target row).
fn bulb(p: &egui::Painter, cv: Canvas, at: Pos2, on: bool, below: bool) {
    let c = cv.p(at.x, at.y);
    let glass = if on { AMBER } else { Color32::from_rgb(120, 146, 152) };
    let base = Color32::from_rgb(150, 160, 160);
    let dir = if below { -1.0 } else { 1.0 };
    if on {
        p.circle_filled(c, 22.0 * cv.k, Color32::from_rgba_unmultiplied(255, 170, 40, 40));
    }
    p.circle_filled(c, 15.0 * cv.k, glass);
    p.circle_filled(
        c - Vec2::new(5.0, 5.0 * dir) * cv.k,
        4.5 * cv.k,
        Color32::from_rgb(255, 250, 220).gamma_multiply(if on { 1.0 } else { 0.5 }),
    );
    for i in 0..3 {
        let y = c.y + dir * (16.0 + i as f32 * 6.0) * cv.k;
        let r = Rect::from_center_size(Pos2::new(c.x, y), Vec2::new(16.0 - i as f32 * 2.0, 4.0) * cv.k);
        p.rect_filled(r, CornerRadius::same(2), base);
    }
}

/// 3.5" disk icon with the drive name; coloured strip when it is the source / target.
fn disk_icon(p: &egui::Painter, cv: Canvas, r: Rect, name: &str, src: bool, tgt: bool, active: bool) {
    let body = Color32::from_rgb(36, 80, 200);
    p.rect_filled(r, CornerRadius::same((4.0 * cv.k) as u8), body);
    bevel(p, r, cv.k, Color32::from_rgb(110, 150, 240), Color32::from_rgb(14, 30, 90));
    let strip = cv.k * 8.0;
    let top = Rect::from_min_size(
        r.min + Vec2::new(cv.k * 10.0, cv.k * 4.0),
        Vec2::new(r.width() - cv.k * 20.0, strip),
    );
    let strip_color = match (src, tgt) {
        (true, true) => VALUE,
        (true, false) => GOOD,
        (false, true) => AMBER,
        _ => Color32::from_rgb(70, 110, 220),
    };
    p.rect_filled(top, CornerRadius::ZERO, strip_color);
    let label = Rect::from_min_size(
        r.min + Vec2::new(cv.k * 10.0, cv.k * 14.0),
        Vec2::new(r.width() - cv.k * 20.0, r.height() * 0.48),
    );
    p.rect_filled(label, CornerRadius::ZERO, Color32::from_rgb(196, 214, 236));
    p.text(
        label.center() - Vec2::new(0.0, 9.0 * cv.k),
        Align2::CENTER_CENTER,
        "DRIVE",
        cv.font(15.0),
        Color32::from_rgb(30, 50, 110),
    );
    p.text(
        label.center() + Vec2::new(0.0, 11.0 * cv.k),
        Align2::CENTER_CENTER,
        name,
        cv.font(19.0),
        Color32::from_rgb(30, 50, 110),
    );
    let shutter = Rect::from_min_size(
        Pos2::new(r.center().x - 26.0 * cv.k, r.bottom() - 30.0 * cv.k),
        Vec2::new(52.0, 26.0) * cv.k,
    );
    p.rect_filled(shutter, CornerRadius::ZERO, Color32::from_rgb(170, 180, 190));
    p.rect_filled(
        Rect::from_min_size(shutter.min + Vec2::new(10.0, 4.0) * cv.k, Vec2::new(10.0, 18.0) * cv.k),
        CornerRadius::ZERO,
        Color32::from_rgb(30, 60, 160),
    );
    if active {
        p.rect_stroke(
            r.expand(3.0 * cv.k),
            CornerRadius::ZERO,
            Stroke::new(2.0 * cv.k, VALUE),
            StrokeKind::Outside,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_ranges() {
        let mut s = RetroSettings::default();
        assert_eq!(s.tracks(), "", "whole disk: format default");
        s.end = 39;
        assert_eq!(s.tracks(), "c=0-39");
        s.side = Side::One;
        assert_eq!(s.tracks(), "c=0-39:h=1");
        s.format = "commodore.1541".into();
        assert_eq!(s.last_cyl(), 39);
        s.side = Side::Both;
        assert_eq!(s.tracks(), "", "0-39 is the whole 1541 disk");
    }

    #[test]
    fn cells() {
        assert_eq!(cell(TrackState::Ok, Some((11, 11))).0, "0");
        assert_eq!(cell(TrackState::Bad, Some((8, 11))).0, "3");
        assert_eq!(cell(TrackState::Bad, Some((0, 18))).0, "9");
        assert_eq!(cell(TrackState::Recovered, None).0, "R");
        assert_eq!(cycle(&[1, 2, 3], 3, false), 1);
        assert_eq!(cycle(&[1, 2, 3], 1, true), 3);
    }
}
