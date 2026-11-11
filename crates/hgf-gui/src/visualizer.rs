//! Circular disk visualizer: one disc per side, one ring per cylinder (outer = cylinder 0),
//! split into sectors once gw has printed its sector map.

use crate::theme;
use eframe::egui::{self, Color32, FontId, Pos2, RichText, Sense, Shape, Stroke, Ui, Vec2};
use hgf_core::{DiskMap, TrackState};
use std::f32::consts::{PI, TAU};

/// Draws the disk; returns the sector clicked (cylinder, side, sector index).
pub fn show(ui: &mut Ui, map: &DiskMap, busy: bool) -> Option<(u16, u8, usize)> {
    let mut hit = None;
    let dark = ui.visuals().dark_mode;
    let heads = map.geometry.heads.clone();
    let n = heads.len().max(1) as f32;
    let gap = 16.0;
    let avail = ui.available_size();
    let legend_h = 130.0;
    // Side by side, or stacked when the column is narrow (large text, small window): whichever
    // gives the bigger discs. Never larger than the space available, so nothing is clipped.
    let fit = |w: f32, h: f32| w.min(h).clamp(0.0, 460.0);
    let row = fit((avail.x - gap * (n - 1.0)) / n, avail.y - legend_h);
    let column = fit(avail.x, (avail.y - legend_h - gap * (n - 1.0)) / n);
    let stacked = n > 1.0 && column > row * 1.15;
    let side = if stacked { column } else { row }.max(48.0).min(avail.x.max(48.0));
    let mut discs = |ui: &mut Ui| {
        for (i, &h) in heads.iter().enumerate() {
            if i > 0 {
                ui.add_space(gap);
            }
            if let Some(x) = disc(ui, map, h, side, busy, dark) {
                hit = Some(x);
            }
        }
    };
    if stacked {
        ui.vertical_centered(|ui| discs(ui));
    } else {
        ui.horizontal(|ui| {
            let used = side * n + gap * (n - 1.0);
            ui.add_space(((avail.x - used) / 2.0).max(0.0));
            discs(ui);
        });
    }
    ui.add_space(6.0);
    legend(ui, map, dark);
    hit
}

fn arc_points(center: Pos2, r: f32, a0: f32, a1: f32) -> Vec<Pos2> {
    let steps = (((a1 - a0) * r / 3.0).ceil() as usize).clamp(2, 256);
    (0..=steps)
        .map(|i| {
            let a = a0 + (a1 - a0) * i as f32 / steps as f32;
            center + Vec2::angled(a) * r
        })
        .collect()
}

fn disc(ui: &mut Ui, map: &DiskMap, head: u8, side: f32, busy: bool, dark: bool) -> Option<(u16, u8, usize)> {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(side), Sense::click());
    let mut hit = None;
    let painter = ui.painter_at(rect);
    let c = rect.center();
    let outer = side * 0.47;
    let inner = side * 0.17;
    let cyls = &map.geometry.cyls;
    let ncyl = cyls.len().max(1);
    let ring = (outer - inner) / ncyl as f32;
    let media = if dark { Color32::from_rgb(12, 13, 16) } else { Color32::from_rgb(40, 42, 48) };

    painter.circle_filled(c, outer + 3.0, media);
    // Pending tracks form one continuous band (avoids moiré between thin rings).
    painter.circle_filled(c, outer, theme::state_color(TrackState::Pending, dark));
    let top = -PI / 2.0;
    for (i, &cyl) in cyls.iter().enumerate() {
        let r = outer - ring * (i as f32 + 0.5);
        let t = map.get(cyl, head);
        if t.state == TrackState::Pending && t.sector_map.is_empty() {
            continue;
        }
        // Slight overlap between neighbouring rings avoids anti-aliasing moiré.
        let width = ring + 0.7;
        if t.sector_map.is_empty() {
            painter.circle_stroke(c, r, Stroke::new(width, theme::state_color(t.state, dark)));
        } else {
            let ns = t.sector_map.len() as f32;
            let seg = TAU / ns;
            let pad = (seg * 0.05).min(0.012);
            for (s, good) in t.sector_map.iter().enumerate() {
                let color = match good {
                    // A bad track still has good sectors: show them green.
                    Some(true) if t.state == TrackState::Bad => theme::OK,
                    Some(true) => theme::state_color(t.state, dark),
                    Some(false) => theme::BAD,
                    None => theme::state_color(TrackState::Pending, dark),
                };
                let a0 = top + seg * s as f32 + pad;
                painter
                    .add(Shape::line(arc_points(c, r, a0, a0 + seg - 2.0 * pad), Stroke::new(width, color)));
            }
        }
    }

    // Active track: pulsing highlight.
    if let Some((cyl, h)) = map.active.filter(|(_, h)| *h == head) {
        if let Some(i) = cyls.iter().position(|&x| x == cyl) {
            let r = outer - ring * (i as f32 + 0.5);
            let t = ui.input(|i| i.time) as f32;
            let alpha = if busy { (150.0 + 100.0 * (t * 6.0).sin()) as u8 } else { 200 };
            painter.circle_stroke(c, r, Stroke::new((ring * 1.6).max(2.5), Color32::from_white_alpha(alpha)));
            let _ = h;
            // Label the head ring, so that it is not mistaken for a disk feature.
            let at = c + Vec2::angled(PI * 0.25) * r;
            let font = FontId::proportional((side * 0.03).clamp(9.0, 12.0));
            let text = format!("HEAD · C{cyl}");
            let galley = painter.layout_no_wrap(text.clone(), font.clone(), Color32::BLACK);
            let bg = egui::Rect::from_min_size(at + Vec2::new(4.0, 2.0), galley.size() + Vec2::new(6.0, 2.0));
            painter.rect_filled(bg, 3.0, Color32::from_white_alpha(220));
            painter.text(bg.center(), egui::Align2::CENTER_CENTER, text, font, Color32::BLACK);
        }
    }

    // Hub and index hole.
    painter.circle_filled(
        c,
        inner * 0.92,
        if dark { Color32::from_rgb(34, 37, 44) } else { Color32::from_gray(200) },
    );
    painter.circle_stroke(c, inner * 0.92, Stroke::new(1.5, Color32::from_gray(110)));
    painter.circle_filled(c, inner * 0.35, media);
    painter.circle_filled(c + Vec2::angled(top) * (inner * 0.7), 3.0, theme::ACCENT);
    painter.text(
        c + Vec2::new(0.0, inner * 0.62),
        egui::Align2::CENTER_CENTER,
        format!("SIDE {head}"),
        FontId::proportional((side * 0.035).clamp(10.0, 14.0)),
        if dark { Color32::from_gray(210) } else { Color32::from_gray(50) },
    );
    painter.circle_stroke(c, outer + 3.0, Stroke::new(1.5, Color32::from_gray(90)));

    // Hover: which cylinder/sector is under the pointer.
    if let Some(pos) = resp.hover_pos() {
        let d = pos.distance(c);
        if d >= inner && d <= outer {
            let i = (((outer - d) / ring) as usize).min(ncyl - 1);
            let cyl = cyls[i];
            let r = outer - ring * (i as f32 + 0.5);
            painter.circle_stroke(c, r, Stroke::new((ring * 1.3).max(2.0), Color32::from_white_alpha(120)));
            let t = map.get(cyl, head);
            let angle = ((pos - c).angle() - top).rem_euclid(TAU);
            let n = t.sector_map.len();
            if resp.clicked() && n > 0 {
                hit = Some((cyl, head, (((angle / TAU) * n as f32) as usize).min(n - 1)));
            }
            resp.on_hover_ui_at_pointer(|ui| {
                ui.label(RichText::new(format!("Cylinder {cyl} · Side {head}")).strong());
                ui.label(RichText::new(t.state.label()).color(theme::state_color(t.state, true)));
                if let Some((ok, total)) = t.sectors {
                    ui.label(format!("{ok}/{total} sectors decoded"));
                }
                if t.retries > 0 {
                    ui.label(format!("{} retry pass(es)", t.retries));
                }
                if !t.sector_map.is_empty() {
                    let s = ((angle / TAU) * t.sector_map.len() as f32) as usize;
                    let status = match t.sector_map.get(s).copied().flatten() {
                        Some(true) => "good",
                        Some(false) => "MISSING",
                        None => "n/a",
                    };
                    ui.label(format!("Logical sector {s}: {status}"));
                    let bad: Vec<String> = t
                        .sector_map
                        .iter()
                        .enumerate()
                        .filter(|(_, g)| **g == Some(false))
                        .map(|(i, _)| i.to_string())
                        .collect();
                    if !bad.is_empty() {
                        ui.label(RichText::new(format!("Missing: {}", bad.join(", "))).color(theme::BAD));
                    }
                    ui.label(RichText::new("Click to see the sector's bytes").small().color(theme::MUTED));
                }
            });
        }
    }
    hit
}

fn legend(ui: &mut Ui, map: &DiskMap, dark: bool) {
    let c = map.counts();
    let items = [
        (TrackState::Ok, "OK", c.ok),
        (TrackState::Recovered, "Recovered", c.recovered),
        (TrackState::Bad, "Bad", c.bad),
        (TrackState::Empty, "Empty", c.empty),
        (TrackState::Flux, "Flux", c.flux),
        (TrackState::Pending, "Pending", c.pending),
    ];
    ui.horizontal_wrapped(|ui| {
        for (state, label, n) in items {
            if n == 0 && matches!(state, TrackState::Empty | TrackState::Flux) {
                continue;
            }
            let (r, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), Sense::hover());
            ui.painter().circle_filled(r.center(), 5.5, theme::state_color(state, dark));
            ui.label(RichText::new(format!("{n} {label}")).strong());
            ui.add_space(6.0);
        }
        if let Some((good, total)) = map.found {
            let pct = if total > 0 { good as f32 * 100.0 / total as f32 } else { 0.0 };
            ui.separator();
            ui.label(RichText::new(format!("{good}/{total} sectors ({pct:.1}%)")).strong());
        }
    });
    ui.label(
        RichText::new(
            "Outer ring = cylinder 0. The white ring is the track under the head (the last one read \
             or written). Hover over a ring for its details.",
        )
        .small()
        .color(theme::MUTED),
    );
    if !map.geometry_known {
        ui.label(
            RichText::new("Geometry is provisional until gw reports the real track set.")
                .small()
                .color(theme::MUTED),
        );
    }
}
