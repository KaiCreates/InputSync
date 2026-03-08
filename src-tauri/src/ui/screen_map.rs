use eframe::egui;
use eframe::egui::{Color32, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};

use crate::state::{DeadCorners, DeadZone, EdgeTriggers};

const ACTIVE_COLOR: Color32 = Color32::from_rgb(0, 200, 200);
const INACTIVE_COLOR: Color32 = Color32::from_rgb(60, 60, 70);
const DEAD_CORNER_COLOR: Color32 = Color32::from_rgba_premultiplied(200, 50, 50, 120);
const DEAD_ZONE_COLOR: Color32 = Color32::from_rgba_premultiplied(200, 50, 50, 80);
const EDGE_THICKNESS: f32 = 10.0;

pub struct ScreenMapResponse {
    pub edge_triggers: EdgeTriggers,
}

/// Render an interactive mini-screen widget for edge trigger configuration.
/// Returns the (possibly updated) EdgeTriggers after processing clicks.
pub fn show(
    ui: &mut Ui,
    triggers: &EdgeTriggers,
    corners: &DeadCorners,
    dead_zones: &[DeadZone],
    size: Vec2,
) -> (Response, ScreenMapResponse) {
    let (resp, painter) = ui.allocate_painter(size, Sense::hover());
    let rect = resp.rect;

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(30, 30, 40));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0, Color32::from_rgb(80, 80, 90)), egui::StrokeKind::Outside);

    let mut new_triggers = triggers.clone();

    // Draw dead zones
    for dz in dead_zones {
        let dz_rect = Rect::from_min_size(
            Pos2::new(
                rect.min.x + dz.x_frac * rect.width(),
                rect.min.y + dz.y_frac * rect.height(),
            ),
            Vec2::new(dz.w_frac * rect.width(), dz.h_frac * rect.height()),
        );
        painter.rect_filled(dz_rect, 2.0, DEAD_ZONE_COLOR);
    }

    // Draw dead corners
    let cs = corners.size_px as f32 / 4.0; // scaled to widget
    if corners.top_left {
        painter.rect_filled(
            Rect::from_min_size(rect.min, Vec2::splat(cs)),
            0.0,
            DEAD_CORNER_COLOR,
        );
    }
    if corners.top_right {
        painter.rect_filled(
            Rect::from_min_size(Pos2::new(rect.max.x - cs, rect.min.y), Vec2::splat(cs)),
            0.0,
            DEAD_CORNER_COLOR,
        );
    }
    if corners.bottom_left {
        painter.rect_filled(
            Rect::from_min_size(Pos2::new(rect.min.x, rect.max.y - cs), Vec2::splat(cs)),
            0.0,
            DEAD_CORNER_COLOR,
        );
    }
    if corners.bottom_right {
        painter.rect_filled(
            Rect::from_min_size(rect.max - Vec2::splat(cs), Vec2::splat(cs)),
            0.0,
            DEAD_CORNER_COLOR,
        );
    }

    // Edge strips — clickable
    let edges = [
        (
            "top",
            Rect::from_min_size(rect.min, Vec2::new(rect.width(), EDGE_THICKNESS)),
            triggers.top,
        ),
        (
            "bottom",
            Rect::from_min_size(
                Pos2::new(rect.min.x, rect.max.y - EDGE_THICKNESS),
                Vec2::new(rect.width(), EDGE_THICKNESS),
            ),
            triggers.bottom,
        ),
        (
            "left",
            Rect::from_min_size(rect.min, Vec2::new(EDGE_THICKNESS, rect.height())),
            triggers.left,
        ),
        (
            "right",
            Rect::from_min_size(
                Pos2::new(rect.max.x - EDGE_THICKNESS, rect.min.y),
                Vec2::new(EDGE_THICKNESS, rect.height()),
            ),
            triggers.right,
        ),
    ];

    for (name, edge_rect, active) in &edges {
        let color = if *active { ACTIVE_COLOR } else { INACTIVE_COLOR };
        painter.rect_filled(*edge_rect, 2.0, color);

        let edge_resp = ui.interact(*edge_rect, ui.id().with(*name), Sense::click());
        if edge_resp.clicked() {
            match *name {
                "top" => new_triggers.top = !new_triggers.top,
                "bottom" => new_triggers.bottom = !new_triggers.bottom,
                "left" => new_triggers.left = !new_triggers.left,
                "right" => new_triggers.right = !new_triggers.right,
                _ => {}
            }
        }
        if edge_resp.hovered() {
            painter.rect_stroke(*edge_rect, 2.0, Stroke::new(1.5, Color32::WHITE), egui::StrokeKind::Outside);
        }
    }

    // Center label
    let center = rect.center();
    painter.text(
        center,
        egui::Align2::CENTER_CENTER,
        "Screen",
        egui::FontId::proportional(12.0),
        Color32::from_rgb(120, 120, 140),
    );

    (resp, ScreenMapResponse { edge_triggers: new_triggers })
}
