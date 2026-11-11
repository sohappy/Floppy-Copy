//! Sector bytes: click a sector on the disk view to see its bytes in the image just read,
//! written or converted.

use crate::app::App;
use crate::theme;
use eframe::egui::{self, RichText, Ui};
use egui_phosphor::regular as ph;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

pub struct SectorView {
    pub path: PathBuf,
    pub cyl: u16,
    pub head: u8,
    pub sector: usize,
    /// Offset in the image and the bytes, or why they cannot be shown.
    pub data: Result<(u64, Vec<u8>), String>,
    /// gw's verdict for this sector (None: unknown).
    pub good: Option<bool>,
}

/// Hex dump: offset, 16 bytes, their ASCII.
pub fn hex_dump(base: u64, data: &[u8]) -> String {
    let mut out = String::new();
    for (i, row) in data.chunks(16).enumerate() {
        let hex: Vec<String> = row.iter().map(|b| format!("{b:02X}")).collect();
        let (a, b) = hex.split_at(hex.len().min(8));
        let ascii: String =
            row.iter().map(|&c| if (0x20..0x7f).contains(&c) { c as char } else { '.' }).collect();
        out += &format!("{:06X}  {:<23}  {:<23}  {ascii}\n", base + i as u64 * 16, a.join(" "), b.join(" "));
    }
    out
}

impl App {
    /// Show the bytes of one sector of the image behind the disk view.
    pub fn open_sector(&mut self, cyl: u16, head: u8, sector: usize) {
        let Some(path) = self.map_image.clone() else {
            self.notice = Some(
                "Sector bytes are shown after reading, writing or converting a sector image \
                 (ADF, IMG, ST, D64…) with a disk format: flux images hold no sectors."
                    .into(),
            );
            return;
        };
        let good = self.map.get(cyl, head).sector_map.get(sector).copied().flatten();
        let data = (|| {
            let len = std::fs::metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?.len();
            let place = self.map.sector_place(len, cyl, head, sector)?;
            let mut f = std::fs::File::open(&path).map_err(|e| e.to_string())?;
            f.seek(SeekFrom::Start(place.offset)).map_err(|e| e.to_string())?;
            let mut buf = vec![0u8; place.size as usize];
            f.read_exact(&mut buf).map_err(|e| e.to_string())?;
            Ok((place.offset, buf))
        })();
        self.sector_view = Some(SectorView { path, cyl, head, sector, data, good });
    }

    /// Next or previous sector, across tracks in the image's order.
    fn step_sector(&mut self, forward: bool) {
        let Some(v) = &self.sector_view else { return };
        let mut order: Vec<(u16, u8, usize)> = Vec::new();
        for &c in &self.map.geometry.cyls {
            for &h in &self.map.geometry.heads {
                for s in 0..self.map.get(c, h).sector_map.len() {
                    order.push((c, h, s));
                }
            }
        }
        let Some(i) = order.iter().position(|&x| x == (v.cyl, v.head, v.sector)) else { return };
        let j = if forward { i + 1 } else { i.wrapping_sub(1) };
        if let Some(&(c, h, s)) = order.get(j) {
            self.open_sector(c, h, s);
        }
    }

    pub fn sector_window(&mut self, ctx: &egui::Context) {
        let Some(v) = &self.sector_view else { return };
        let mut open = true;
        let mut step: Option<bool> = None;
        let title = format!("Sector {} · cylinder {} · side {}", v.sector, v.cyl, v.head);
        let file = v.path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
        let (good, data) = (v.good, &v.data);
        egui::Window::new(title)
            .id(egui::Id::new("sector_view"))
            .open(&mut open)
            .resizable(true)
            .default_width(theme::w(640.0))
            .show(ctx, |ui: &mut Ui| {
                ui.horizontal(|ui| {
                    if ui.button(format!("{}  Previous", ph::CARET_LEFT)).clicked() {
                        step = Some(false);
                    }
                    if ui.button(format!("Next  {}", ph::CARET_RIGHT)).clicked() {
                        step = Some(true);
                    }
                    let (txt, col) = match good {
                        Some(true) => ("read correctly", theme::OK),
                        Some(false) => ("MISSING: the image holds gw's filler", theme::BAD),
                        None => ("no verdict from gw", theme::MUTED),
                    };
                    ui.label(RichText::new(txt).color(col).strong());
                });
                match data {
                    Err(e) => {
                        ui.label(RichText::new(e).color(theme::WARN));
                    }
                    Ok((offset, bytes)) => {
                        let dump = hex_dump(*offset, bytes);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!(
                                    "{file} · offset {offset} (0x{offset:X}) · {} bytes",
                                    bytes.len()
                                ))
                                .small()
                                .color(theme::MUTED),
                            );
                            if ui.small_button(ph::COPY).on_hover_text("Copy the dump").clicked() {
                                ui.ctx().copy_text(dump.clone());
                            }
                        });
                        egui::ScrollArea::vertical().max_height(theme::w(420.0)).show(ui, |ui| {
                            ui.add(
                                egui::Label::new(RichText::new(dump).monospace().size(theme::fs(12.5)))
                                    .extend(),
                            );
                        });
                    }
                }
            });
        if !open {
            self.sector_view = None;
        } else if let Some(f) = step {
            self.step_sector(f);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dumps_rows() {
        let d = hex_dump(0x200, b"DOS\0Hello, world!\x01");
        let lines: Vec<&str> = d.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0]
            .starts_with("000200  44 4F 53 00 48 65 6C 6C  6F 2C 20 77 6F 72 6C 64  DOS.Hello, world"));
        assert!(lines[1].starts_with("000210  21 01"));
        assert!(lines[1].ends_with("!."));
    }
}
