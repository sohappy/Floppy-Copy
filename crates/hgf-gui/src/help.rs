//! Help: the short explanation shown at the top of each page, and the Help page.

use crate::app::{App, Page};
use crate::theme;
use crate::widgets::card;
use eframe::egui::{self, RichText, Ui};
use egui_phosphor::regular as ph;

impl Page {
    /// What the page is for, in plain words.
    pub fn about(self) -> &'static str {
        match self {
            Page::Read => {
                "Copies a floppy disk into an image file on this computer. Choose the computer the \
                 disk comes from, the file to create, then READ. Red tracks could not be read: \
                 re-read them with more retries. Hover over a ring of the disk for its details."
            }
            Page::Write => {
                "Writes an image file (ADF, IMG, ST, D64, SCP, HFE…) onto a floppy disk, then reads \
                 every track back to check it. Everything on the disk is replaced."
            }
            Page::Copy => {
                "Disk-to-disk copy: the source disk is read into a temporary image, then written to \
                 the target disk. With a single drive, you are asked to swap the disks."
            }
            Page::Format => {
                "Creates an empty disk, ready to use on the chosen computer (with its directory or \
                 filesystem), then writes and verifies it. The blank disk can also be saved as an image."
            }
            Page::Convert => {
                "Changes an image file into another type, without any disk or drive. Examples: a \
                 flux image (.scp) read from a disk into an .adf for an emulator, or an .adf into an \
                 .hfe for a Gotek. The extension of the output file selects its type; a flux input \
                 also needs the disk format, to decode its sectors."
            }
            Page::Erase => {
                "Wipes the magnetic surface of the disk. No filesystem is created: use Format disk \
                 for a usable disk. The tracks erased follow the drive type (40 or 80 tracks) set \
                 next to the drive selector."
            }
            Page::Health => {
                "Guided check of a drive with a disk you know is good: speed, USB link, read \
                 quality, head alignment, optional write test, then a verdict."
            }
            Page::Rpm => {
                "Measures how fast the disk turns. Expect 300 rpm (3.5\" and most 5.25\" drives) or \
                 360 rpm (5.25\" HD drives). Beyond about 1 % off, reads and writes can fail."
            }
            Page::Align => {
                "Reads the same track again and again while you adjust the drive's head: the bars \
                 show how many sectors decode on each read. Needs an alignment disk or a known-good \
                 disk, and Greaseweazle host tools 1.21 or later (gw align)."
            }
            Page::Seek => {
                "Moves the head to a cylinder and leaves it there, to test the stepper motor or \
                 check a head position. Many drives only step while their motor turns: keep \
                 \"Spin the motor\" ticked."
            }
            Page::Clean => {
                "Sweeps the head back and forth over a wet cleaning disk. Only with a CLEANING disk \
                 in the drive."
            }
            Page::Info => {
                "Shows the Greaseweazle model, its firmware and the host tools version: the first \
                 thing to check when nothing works."
            }
            Page::Delays => {
                "For experts. The timings the Greaseweazle uses to drive the floppy drive (select, \
                 step, settle, motor…). The factory values suit almost every drive: change one only \
                 to fix a specific problem, and restore the factory values afterwards."
            }
            Page::Pins => {
                "For experts. Reads or sets single pins of the 34-pin floppy connector. Normal \
                 reading and writing never need it: gw drives the pins itself."
            }
            Page::Bandwidth => {
                "Checks that the USB link is fast enough to stream flux. If it fails, plug the \
                 Greaseweazle directly into the computer with another cable, without a hub."
            }
            Page::Reset => {
                "Puts the Greaseweazle back in its power-on state, as if it had been unplugged: use \
                 it when a drive stays selected or its motor keeps turning after an interrupted \
                 command."
            }
            Page::Update => {
                "Installs a new firmware in the Greaseweazle, from GitHub or a local file. Keep the \
                 device plugged in until the update is over."
            }
            Page::Inspector => {
                "Looks inside an image file without touching any disk: size, checksum, probable \
                 format and, for Amiga ADF images, the volume and its files. Use it to check an \
                 image before writing it; the buttons send it to Write disk or Convert."
            }
            Page::Help => "How Floppy-Copy works, the words it uses, and where to find more help.",
            Page::Settings => "Where gw is, the Greaseweazle port, the drives, appearance and log files.",
        }
    }
}

const GLOSSARY: &[(&str, &str)] = &[
    ("Image", "A file holding the content of a disk (.adf, .img, .st, .d64, .scp…)."),
    (
        "Sector image",
        "Only the data, decoded sector by sector (ADF, IMG, D64…). Small, used by emulators; \
         copy-protected disks do not fit in it.",
    ),
    (
        "Flux image",
        "The raw magnetic signal (SCP, HFE, KryoFlux RAW). Keeps everything, copy protections \
         included; needs the disk format to be decoded or converted.",
    ),
    (
        "Cylinder, side, track",
        "A cylinder is one head position (0 = outer edge); each side (head 0 or 1) of a cylinder is \
         a track. 3.5\" disks have 80 cylinders, 5.25\" DD disks 40.",
    ),
    (
        "Tracks (TSPEC)",
        "Which tracks a command uses, e.g. c=0-39:h=0 (cylinders 0 to 39, side 0). Parts are joined \
         by ':'; empty = the format's own tracks.",
    ),
    (
        "40 / 80-track drive",
        "Set next to the drive selector. A 5.25\" DD drive (360K) stops at cylinder 39: Floppy-Copy \
         then refuses or limits commands that would go further.",
    ),
    (
        "Double step",
        "For a 40-track disk (C64, Apple II, PC 360K…) in an 80-track 5.25\" drive: the head moves \
         two steps per cylinder. Never on a 40-track drive.",
    ),
    ("DD / HD", "Density of the disk: DD (one square hole) for 720K/880K, HD (two holes) for 1.44M."),
    ("Verify", "After writing, every track is read back and compared."),
];

impl App {
    /// Short explanation at the top of the page (hidden with the ? next to the title).
    pub fn page_help_banner(&mut self, ui: &mut Ui) {
        let page = self.s.page;
        if !self.s.page_help || page == Page::Help {
            return;
        }
        egui::Frame::new()
            .fill(theme::tint(page.color(), 18))
            .corner_radius(8)
            .inner_margin(egui::Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    ui.label(RichText::new(ph::INFO).color(page.color()));
                    ui.add(egui::Label::new(RichText::new(page.about()).small()).wrap());
                });
            });
        ui.add_space(6.0);
    }

    pub fn page_help(&mut self, ui: &mut Ui) {
        let c = theme::DEVICE;
        let mut go = None;
        egui::ScrollArea::vertical().id_salt("help").show(ui, |ui| {
            card(ui, ph::ROCKET_LAUNCH, "Getting started", c, |ui| {
                for (i, step) in [
                    "Plug the Greaseweazle into a USB port, then connect the drive with its ribbon \
                     cable and its power supply.",
                    "Choose the drive in the top bar: PC twisted cable → Drive A (connector after the \
                     twist) or Drive B (middle); straight cable, Amiga or other old drive → Unit 0 or 1.",
                    "Next to it, choose the drive type: 80 tracks (3.5\", 5.25\" HD) or 40 tracks \
                     (5.25\" DD, 3\"). A wrong type can send the head into its end stop.",
                    "Check everything with Drive health and a disk you know is good.",
                    "Read a disk with Read disk, write one with Write disk, or use the Retro screen \
                     for quick disk-to-disk copies.",
                ]
                .iter()
                .enumerate()
                {
                    ui.horizontal_top(|ui| {
                        ui.label(RichText::new(format!("{}.", i + 1)).strong());
                        ui.add(egui::Label::new(*step).wrap());
                    });
                }
            });
            ui.add_space(8.0);
            card(ui, ph::SIGNPOST, "Which page for what?", c, |ui| {
                ui.label(RichText::new("Click a page name to open it.").small().color(theme::MUTED));
                for (group, pages) in Page::GROUPS {
                    ui.add_space(4.0);
                    ui.label(RichText::new(*group).small().strong().color(theme::MUTED));
                    for &p in *pages {
                        if p == Page::Help {
                            continue;
                        }
                        ui.horizontal_top(|ui| {
                            let r = ui.add(
                                egui::Button::new(
                                    RichText::new(format!("{}  {}", p.icon(), p.title())).color(p.color()),
                                )
                                .frame(false)
                                .min_size(egui::vec2(theme::w(150.0), 0.0)),
                            );
                            if r.clicked() {
                                go = Some(p);
                            }
                            ui.add(egui::Label::new(RichText::new(p.about()).small()).wrap());
                        });
                    }
                }
            });
            ui.add_space(8.0);
            card(ui, ph::BOOK_OPEN, "Words used", c, |ui| {
                egui::Grid::new("glossary").num_columns(2).spacing([12.0, 6.0]).striped(true).show(
                    ui,
                    |ui| {
                        for (word, text) in GLOSSARY {
                            ui.label(RichText::new(*word).strong());
                            ui.add(egui::Label::new(RichText::new(*text).small()).wrap());
                            ui.end_row();
                        }
                    },
                );
            });
            ui.add_space(8.0);
            card(ui, ph::WRENCH, "Known limits of the host tools", c, |ui| {
                for (title, text) in [
                    (
                        "GCR disks (Apple II, Commodore 1541/1571, Macintosh)",
                        "gw needs its compiled data routines for them. If it says \"Optimised data \
                         routines not found\" or \"requires optimised C extension\": on Windows use \
                         the official release; on macOS and Linux reinstall the host tools with a C \
                         compiler present (macOS: xcode-select --install; Debian/Ubuntu: \
                         build-essential and python3-dev).",
                    ),
                    (
                        "Head alignment",
                        "gw align exists since host tools 1.21. Older versions do not have it: \
                         update the host tools (see the links below).",
                    ),
                    (
                        "IPF images",
                        "Need the SPS/CAPS library (included in the Windows release of the host tools).",
                    ),
                ] {
                    ui.label(RichText::new(title).strong());
                    ui.add(egui::Label::new(RichText::new(text).small()).wrap());
                    ui.add_space(4.0);
                }
            });
            ui.add_space(8.0);
            card(ui, ph::TERMINAL, "Typing gw commands", c, |ui| {
                ui.add(
                    egui::Label::new(
                        RichText::new(
                            "The field at the bottom of the console runs any gw command, e.g. reset, \
                             info, rpm --nr 5 or erase --tracks c=0-39. The selected drive (--drive) \
                             and port (--device) are added unless you type them. Commands that \
                             change a disk or the device ask for confirmation, but they skip \
                             Floppy-Copy's checks (drive type, image size). Every page also shows, \
                             under its button, the exact command it runs.",
                        )
                        .small(),
                    )
                    .wrap(),
                );
            });
            ui.add_space(8.0);
            card(ui, ph::MONITOR, "Retro screen", c, |ui| {
                ui.add(
                    egui::Label::new(
                        RichText::new(
                            "Retro in the top bar opens a full-screen copier in the spirit of the \
                             classic Amiga copiers; CLASSIC comes back here. Click a value to change \
                             it (right click: previous value), choose the source drive with the lamps \
                             above the drives and the target with those below, then START. In the \
                             grids, 0 is a good track, a digit the number of missing sectors, R a \
                             track read after retries.",
                        )
                        .small(),
                    )
                    .wrap(),
                );
            });
            ui.add_space(8.0);
            card(ui, ph::ARROW_SQUARE_OUT, "More help (Greaseweazle wiki)", c, |ui| {
                for (label, url) in LINKS {
                    ui.hyperlink_to(format!("{}  {label}", ph::ARROW_SQUARE_OUT), *url);
                }
            });
            ui.add_space(8.0);
            ui.label(
                RichText::new(format!(
                    "Floppy-Copy {} — Retro Disk Copier. © 2026 RCC(A). A front-end for Keir \
                     Fraser's Greaseweazle host tools.",
                    hgf_core::VERSION
                ))
                .small()
                .color(theme::MUTED),
            );
        });
        if let Some(p) = go {
            self.s.page = p;
        }
    }
}

pub const LINKS: &[(&str, &str)] = &[
    ("Getting started", "https://github.com/keirf/greaseweazle/wiki/Getting-Started"),
    (
        "Troubleshooting FAQ: No Index, Track 0 not found…",
        "https://github.com/keirf/greaseweazle/wiki/Troubleshooting-FAQ",
    ),
    ("Drive select: A, B, 0, 1…", "https://github.com/keirf/greaseweazle/wiki/Drive-Select"),
    ("Specifying tracks (TSPEC)", "https://github.com/keirf/greaseweazle/wiki/Specifying-Tracks"),
    (
        "Supported image types and file options",
        "https://github.com/keirf/greaseweazle/wiki/Supported-Image-Types",
    ),
    ("Write precompensation", "https://github.com/keirf/greaseweazle/wiki/Write-Precompensation"),
    ("Download the host tools", "https://github.com/keirf/greaseweazle/wiki/Download-Host-Tools"),
];
