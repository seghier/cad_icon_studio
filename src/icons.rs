use eframe::egui::{self, Color32, Painter, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};
use std::f32::consts::{PI, TAU};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VectorIcon {
    ToolMove,
    ToolNode,
    ToolBucketFill,
    ToolEdgePaint,
    ShapeQuad,
    ShapeCircleArc,
    ShapeStar,
    ShapePoly,
    ShapeBezier,
    ShapeLineArrow,
    ShapeArc,
    ToolNurbs,
    LockClosed,
    LockOpen,
    BoolUnion,
    BoolSubtract,
    BoolIntersect,
    BoolXor,
    BoolDivide,
    OrderFront,
    OrderUp,
    OrderDown,
    OrderBack,
    Duplicate,
    Trash,
    RotateCCW,
    RotateCW,
    FlipH,
    FlipV,
    FillRegion,
    SaveProject,
    OpenProject,
}

/// Draws a button with a custom vector-painted icon (and optional ASCII text) so no '□' glyphs ever occur.
pub fn icon_button(
    ui: &mut Ui,
    icon: VectorIcon,
    label: &str,
    size: Vec2,
    active: bool,
    tooltip: &str,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let bg = if active {
            Color32::from_rgb(230, 110, 30)
        } else if resp.is_pointer_button_down_on() {
            Color32::from_rgb(85, 85, 95)
        } else if resp.hovered() {
            Color32::from_rgb(68, 68, 76)
        } else {
            Color32::from_rgb(48, 48, 54)
        };
        let border = if active || resp.hovered() {
            Color32::from_rgb(255, 145, 60)
        } else {
            Color32::from_rgb(64, 64, 72)
        };
        let painter = ui.painter();
        painter.rect_filled(rect, 4.0, bg);
        painter.rect_stroke(rect, 4.0, Stroke::new(1.0, border));

        let fg = if active {
            Color32::WHITE
        } else {
            Color32::from_rgb(235, 235, 240)
        };
        let accent = if active {
            Color32::WHITE
        } else {
            Color32::from_rgb(255, 138, 48)
        };

        if label.is_empty() {
            let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(size.y.min(size.x) - 10.0));
            draw_vector_icon(painter, icon_rect, icon, fg, accent);
        } else {
            let icon_side = (size.y - 8.0).clamp(12.0, 18.0);
            let icon_rect = Rect::from_min_size(
                Pos2::new(rect.left() + 6.0, rect.center().y - icon_side * 0.5),
                Vec2::splat(icon_side),
            );
            draw_vector_icon(painter, icon_rect, icon, fg, accent);
            painter.text(
                Pos2::new(icon_rect.right() + 5.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                label,
                egui::FontId::proportional(12.5),
                fg,
            );
        }
    }
    resp.on_hover_text(tooltip)
}

pub fn draw_vector_icon(
    p: &Painter,
    r: Rect,
    icon: VectorIcon,
    fg: Color32,
    accent: Color32,
) {
    let c = r.center();
    let w = r.width();
    let h = r.height();
    let s = Stroke::new(1.6, fg);
    let sa = Stroke::new(1.8, accent);

    match icon {
        VectorIcon::ToolMove => {
            // High-precision Selection & Transform Cursor Arrow + 4-way Move Cross badge
            // Draw clean arrow head with 2 strictly convex triangles (zero jagged artifacts!)
            let tip = Pos2::new(r.left() + 3.0, r.top() + 2.0);
            let left_wing = Pos2::new(r.left() + 3.0, r.top() + h * 0.72);
            let notch = Pos2::new(r.left() + w * 0.40, r.top() + h * 0.54);
            let right_wing = Pos2::new(r.left() + w * 0.70, r.top() + h * 0.54);

            // Stem quad
            let stem_w = 2.4;
            let stem_start = Pos2::new(r.left() + w * 0.34, r.top() + h * 0.52);
            let stem_end = Pos2::new(r.left() + w * 0.60, r.top() + h * 0.88);
            let dx = stem_end.x - stem_start.x;
            let dy = stem_end.y - stem_start.y;
            let len = (dx * dx + dy * dy).sqrt().max(1.0);
            let nx = -dy / len * stem_w * 0.5;
            let ny = dx / len * stem_w * 0.5;

            let stem_pts = vec![
                Pos2::new(stem_start.x + nx, stem_start.y + ny),
                Pos2::new(stem_start.x - nx, stem_start.y - ny),
                Pos2::new(stem_end.x - nx, stem_end.y - ny),
                Pos2::new(stem_end.x + nx, stem_end.y + ny),
            ];
            p.add(egui::Shape::convex_polygon(stem_pts, fg, Stroke::NONE));

            // Head triangles (strictly convex, 100% anti-aliased)
            let tri1 = vec![tip, left_wing, notch];
            let tri2 = vec![tip, notch, right_wing];
            p.add(egui::Shape::convex_polygon(tri1, fg, Stroke::NONE));
            p.add(egui::Shape::convex_polygon(tri2, fg, Stroke::NONE));

            // Mini 4-way move cross badge in bottom-right corner in accent color
            let mx = r.right() - 4.5;
            let my = r.bottom() - 4.5;
            let m_arm = 3.5;
            p.line_segment([Pos2::new(mx - m_arm, my), Pos2::new(mx + m_arm, my)], Stroke::new(1.4, accent));
            p.line_segment([Pos2::new(mx, my - m_arm), Pos2::new(mx, my + m_arm)], Stroke::new(1.4, accent));
        }
        VectorIcon::ToolNode => {
            p.line_segment([r.left_bottom(), r.right_top()], s);
            let d = 3.5;
            let dia = vec![
                Pos2::new(c.x, c.y - d),
                Pos2::new(c.x + d, c.y),
                Pos2::new(c.x, c.y + d),
                Pos2::new(c.x - d, c.y),
            ];
            p.add(egui::Shape::convex_polygon(dia, accent, Stroke::new(1.2, fg)));
        }
        VectorIcon::ToolBucketFill | VectorIcon::FillRegion => {
            // Paint bucket diamond + droplet
            let pts = vec![
                Pos2::new(c.x - 2.0, r.top() + 2.0),
                Pos2::new(r.right() - 3.0, c.y),
                Pos2::new(c.x - 1.0, r.bottom() - 3.0),
                Pos2::new(r.left() + 2.0, c.y + 1.0),
            ];
            p.add(egui::Shape::convex_polygon(pts, accent, s));
            p.circle_filled(Pos2::new(r.right() - 2.0, r.bottom() - 3.0), 2.5, accent);
        }
        VectorIcon::ToolEdgePaint => {
            p.line_segment([Pos2::new(r.left() + 2.0, r.bottom() - 2.0), Pos2::new(r.right() - 2.0, r.top() + 2.0)], sa);
            p.circle_filled(Pos2::new(r.left() + 3.0, r.bottom() - 3.0), 2.8, fg);
            p.circle_filled(Pos2::new(r.right() - 3.0, r.top() + 3.0), 2.8, fg);
        }
        VectorIcon::ShapeQuad => {
            p.rect_stroke(r.shrink(2.0), 1.5, s);
            p.circle_filled(r.shrink(2.0).left_top(), 2.2, accent);
            p.circle_filled(r.shrink(2.0).right_bottom(), 2.2, accent);
        }
        VectorIcon::ShapeCircleArc => {
            p.circle_stroke(c, w * 0.42, s);
            p.circle_filled(Pos2::new(c.x + w * 0.42, c.y), 2.2, accent);
        }
        VectorIcon::ShapeStar => {
            let mut pts = Vec::new();
            for i in 0..10 {
                let a = -PI * 0.5 + (i as f32) * TAU / 10.0;
                let rad = if i % 2 == 0 { w * 0.46 } else { w * 0.20 };
                pts.push(Pos2::new(c.x + rad * a.cos(), c.y + rad * a.sin()));
            }
            p.add(egui::Shape::closed_line(pts, s));
        }
        VectorIcon::ShapePoly => {
            let pts = vec![
                Pos2::new(c.x, r.top() + 2.0),
                Pos2::new(r.right() - 2.0, c.y - 1.0),
                Pos2::new(r.right() - 4.0, r.bottom() - 2.0),
                Pos2::new(r.left() + 4.0, r.bottom() - 2.0),
                Pos2::new(r.left() + 2.0, c.y - 1.0),
            ];
            p.add(egui::Shape::closed_line(pts, s));
        }
        VectorIcon::ShapeBezier => {
            let p0 = Pos2::new(r.left() + 2.0, r.bottom() - 3.0);
            let p1 = Pos2::new(c.x - 2.0, r.top() + 1.0);
            let p2 = Pos2::new(c.x + 2.0, r.bottom() - 1.0);
            let p3 = Pos2::new(r.right() - 2.0, r.top() + 3.0);
            let mut curve = Vec::new();
            for i in 0..=12 {
                let t = i as f32 / 12.0;
                let mt = 1.0 - t;
                curve.push(Pos2::new(
                    mt.powi(3) * p0.x + 3.0 * mt * mt * t * p1.x + 3.0 * mt * t * t * p2.x + t.powi(3) * p3.x,
                    mt.powi(3) * p0.y + 3.0 * mt * mt * t * p1.y + 3.0 * mt * t * t * p2.y + t.powi(3) * p3.y,
                ));
            }
            p.add(egui::Shape::line(curve, sa));
            p.circle_filled(p0, 2.2, fg);
            p.circle_filled(p3, 2.2, fg);
        }
        VectorIcon::ShapeLineArrow => {
            let a = Pos2::new(r.left() + 2.0, r.bottom() - 2.0);
            let b = Pos2::new(r.right() - 2.0, r.top() + 2.0);
            p.line_segment([a, b], s);
            p.line_segment([b, Pos2::new(b.x - 6.0, b.y + 1.0)], sa);
            p.line_segment([b, Pos2::new(b.x - 1.0, b.y + 6.0)], sa);
        }
        VectorIcon::ShapeArc => {
            // 3-point CAD arc with Start, Mid/Crest, and End nodes
            let p0 = Pos2::new(r.left() + 2.0, r.bottom() - 3.0);
            let p1 = Pos2::new(c.x, r.top() + 3.0);
            let p2 = Pos2::new(r.right() - 2.0, r.bottom() - 3.0);
            let mut curve = Vec::new();
            for i in 0..=12 {
                let t = i as f32 / 12.0;
                let mt = 1.0 - t;
                curve.push(Pos2::new(
                    mt * mt * p0.x + 2.0 * mt * t * p1.x + t * t * p2.x,
                    mt * mt * p0.y + 2.0 * mt * t * p1.y + t * t * p2.y,
                ));
            }
            p.add(egui::Shape::line(curve, sa));
            p.circle_filled(p0, 2.4, fg);
            p.circle_filled(Pos2::new(p1.x, p1.y + (p0.y - p1.y) * 0.25), 2.6, accent);
            p.circle_filled(p2, 2.4, fg);
        }
        VectorIcon::ToolNurbs => {
            // Smooth spline with control points and pen tip
            let p0 = Pos2::new(r.left() + 3.0, r.bottom() - 4.0);
            let p1 = Pos2::new(c.x - 3.0, r.bottom() - 8.0);
            let p2 = Pos2::new(c.x + 3.0, r.top() + 8.0);
            let p3 = Pos2::new(r.right() - 3.0, r.top() + 4.0);
            let mut curve = Vec::new();
            for i in 0..=12 {
                let t = i as f32 / 12.0;
                let mt = 1.0 - t;
                curve.push(Pos2::new(
                    mt.powi(3) * p0.x + 3.0 * mt * mt * t * p1.x + 3.0 * mt * t * t * p2.x + t.powi(3) * p3.x,
                    mt.powi(3) * p0.y + 3.0 * mt * mt * t * p1.y + 3.0 * mt * t * t * p2.y + t.powi(3) * p3.y,
                ));
            }
            p.add(egui::Shape::line(curve, sa));
            p.circle_filled(p0, 2.0, accent);
            p.circle_filled(Pos2::new(c.x - 1.5, c.y + 2.0), 2.0, fg);
            p.circle_filled(Pos2::new(c.x + 1.5, c.y - 2.0), 2.0, fg);
            p.circle_filled(p3, 2.0, accent);
        }
        VectorIcon::LockClosed => {
            let body = Rect::from_center_size(c + Vec2::new(0.0, 2.5), Vec2::new(11.0, 8.5));
            p.rect_filled(body, 1.5, accent);
            let shackle_c = Pos2::new(c.x, c.y - 2.0);
            p.circle_stroke(shackle_c, 3.6, Stroke::new(1.4, fg));
            p.circle_filled(Pos2::new(c.x, c.y + 2.0), 1.2, Color32::from_rgb(30, 30, 35));
        }
        VectorIcon::LockOpen => {
            let body = Rect::from_center_size(c + Vec2::new(0.0, 2.5), Vec2::new(11.0, 8.5));
            p.rect_stroke(body, 1.5, Stroke::new(1.2, Color32::from_rgb(140, 140, 150)));
            let shackle_c = Pos2::new(c.x + 1.8, c.y - 3.5);
            p.circle_stroke(shackle_c, 3.6, Stroke::new(1.3, Color32::from_rgb(170, 170, 180)));
        }
        VectorIcon::BoolUnion => {
            let r1 = Rect::from_min_size(r.left_top() + Vec2::splat(1.0), Vec2::splat(w * 0.62));
            let r2 = Rect::from_min_size(r.left_top() + Vec2::splat(w * 0.32), Vec2::splat(w * 0.62));
            p.rect_filled(r1, 1.0, accent);
            p.rect_filled(r2, 1.0, accent);
        }
        VectorIcon::BoolSubtract => {
            let r1 = Rect::from_min_size(r.left_top() + Vec2::splat(1.0), Vec2::splat(w * 0.62));
            let r2 = Rect::from_min_size(r.left_top() + Vec2::splat(w * 0.32), Vec2::splat(w * 0.62));
            p.rect_filled(r1, 1.0, accent);
            p.rect_filled(r2, 1.0, Color32::from_rgb(40, 40, 46));
            p.rect_stroke(r2, 1.0, Stroke::new(1.2, fg));
        }
        VectorIcon::BoolIntersect => {
            let r1 = Rect::from_min_size(r.left_top() + Vec2::splat(1.0), Vec2::splat(w * 0.62));
            let r2 = Rect::from_min_size(r.left_top() + Vec2::splat(w * 0.32), Vec2::splat(w * 0.62));
            p.rect_stroke(r1, 1.0, Stroke::new(1.0, fg));
            p.rect_stroke(r2, 1.0, Stroke::new(1.0, fg));
            p.rect_filled(r1.intersect(r2), 0.5, accent);
        }
        VectorIcon::BoolXor => {
            let r1 = Rect::from_min_size(r.left_top() + Vec2::splat(1.0), Vec2::splat(w * 0.62));
            let r2 = Rect::from_min_size(r.left_top() + Vec2::splat(w * 0.32), Vec2::splat(w * 0.62));
            p.rect_filled(r1, 1.0, accent);
            p.rect_filled(r2, 1.0, accent);
            p.rect_filled(r1.intersect(r2), 0.5, Color32::from_rgb(35, 35, 40));
        }
        VectorIcon::BoolDivide => {
            let r1 = Rect::from_min_size(r.left_top() + Vec2::splat(1.0), Vec2::splat(w * 0.62));
            let r2 = Rect::from_min_size(r.left_top() + Vec2::splat(w * 0.32), Vec2::splat(w * 0.62));
            p.rect_stroke(r1, 1.0, s);
            p.rect_stroke(r2, 1.0, sa);
        }
        VectorIcon::OrderFront => {
            p.line_segment([Pos2::new(r.left() + 2.0, r.top() + 2.0), Pos2::new(r.right() - 2.0, r.top() + 2.0)], sa);
            draw_arrow_up(p, c + Vec2::new(0.0, 2.0), w * 0.35, s);
        }
        VectorIcon::OrderUp => {
            draw_arrow_up(p, c, w * 0.38, s);
        }
        VectorIcon::OrderDown => {
            draw_arrow_down(p, c, w * 0.38, s);
        }
        VectorIcon::OrderBack => {
            p.line_segment([Pos2::new(r.left() + 2.0, r.bottom() - 2.0), Pos2::new(r.right() - 2.0, r.bottom() - 2.0)], sa);
            draw_arrow_down(p, c - Vec2::new(0.0, 2.0), w * 0.35, s);
        }
        VectorIcon::Duplicate => {
            let r1 = Rect::from_min_size(r.left_top() + Vec2::splat(1.0), Vec2::splat(w * 0.62));
            let r2 = Rect::from_min_size(r.left_top() + Vec2::splat(w * 0.30), Vec2::splat(w * 0.62));
            p.rect_stroke(r1, 1.0, s);
            p.rect_filled(r2, 1.0, accent);
        }
        VectorIcon::Trash => {
            p.line_segment([r.left_top() + Vec2::splat(3.0), r.right_bottom() - Vec2::splat(3.0)], sa);
            p.line_segment([r.right_top() + Vec2::new(-3.0, 3.0), r.left_bottom() + Vec2::new(3.0, -3.0)], sa);
        }
        VectorIcon::RotateCCW | VectorIcon::RotateCW => {
            let cw = icon == VectorIcon::RotateCW;
            draw_curved_rotation_icon(p, c, w * 0.38, cw, s, accent);
        }
        VectorIcon::FlipH => {
            p.line_segment([Pos2::new(c.x, r.top() + 1.0), Pos2::new(c.x, r.bottom() - 1.0)], sa);
            p.line_segment([Pos2::new(r.left() + 2.0, c.y), Pos2::new(r.right() - 2.0, c.y)], s);
        }
        VectorIcon::FlipV => {
            p.line_segment([Pos2::new(r.left() + 1.0, c.y), Pos2::new(r.right() - 1.0, c.y)], sa);
            p.line_segment([Pos2::new(c.x, r.top() + 2.0), Pos2::new(c.x, r.bottom() - 2.0)], s);
        }
        VectorIcon::SaveProject => {
            let body = Rect::from_center_size(c, Vec2::new(w * 0.82, h * 0.82));
            p.rect_stroke(body, 2.0, s);
            let shutter = Rect::from_min_max(
                Pos2::new(body.left() + 2.5, body.top()),
                Pos2::new(body.right() - 2.5, body.top() + body.height() * 0.38),
            );
            p.rect_filled(shutter, 1.0, accent);
            let lbl = Rect::from_min_max(
                Pos2::new(body.left() + 2.5, body.bottom() - body.height() * 0.40),
                Pos2::new(body.right() - 2.5, body.bottom() - 1.5),
            );
            p.rect_filled(lbl, 1.0, fg);
        }
        VectorIcon::OpenProject => {
            let back = Rect::from_min_max(
                Pos2::new(r.left() + 1.0, r.top() + 3.0),
                Pos2::new(r.right() - 1.0, r.bottom() - 2.0),
            );
            p.rect_stroke(back, 2.0, s);
            let tab = Rect::from_min_max(
                Pos2::new(back.left(), back.top() - 3.0),
                Pos2::new(back.left() + back.width() * 0.45, back.top()),
            );
            p.rect_filled(tab, 1.0, accent);
            p.line_segment([Pos2::new(back.left(), back.center().y), Pos2::new(back.right() + 1.5, back.center().y - 2.0)], sa);
        }
    }
}

fn draw_arrow_up(p: &Painter, c: Pos2, rad: f32, s: Stroke) {
    let top = Pos2::new(c.x, c.y - rad);
    p.line_segment([Pos2::new(c.x, c.y + rad), top], s);
    p.line_segment([top, Pos2::new(c.x - rad * 0.75, c.y - rad * 0.15)], s);
    p.line_segment([top, Pos2::new(c.x + rad * 0.75, c.y - rad * 0.15)], s);
}

fn draw_arrow_down(p: &Painter, c: Pos2, rad: f32, s: Stroke) {
    let bot = Pos2::new(c.x, c.y + rad);
    p.line_segment([Pos2::new(c.x, c.y - rad), bot], s);
    p.line_segment([bot, Pos2::new(c.x - rad * 0.75, c.y + rad * 0.15)], s);
    p.line_segment([bot, Pos2::new(c.x + rad * 0.75, c.y + rad * 0.15)], s);
}

pub fn draw_curved_rotation_icon(
    p: &Painter,
    center: Pos2,
    radius: f32,
    clockwise: bool,
    stroke: Stroke,
    head_color: Color32,
) {
    let mut pts = Vec::new();
    let start_a = -PI * 0.85;
    let end_a = PI * 0.45;
    for i in 0..=14 {
        let t = i as f32 / 14.0;
        let a = start_a + (end_a - start_a) * t;
        let x = if clockwise {
            center.x + radius * a.cos()
        } else {
            center.x - radius * a.cos()
        };
        let y = center.y + radius * a.sin();
        pts.push(Pos2::new(x, y));
    }
    if let Some(&tip) = pts.last() {
        p.add(egui::Shape::line(pts, stroke));
        p.circle_filled(tip, 2.8, head_color);
    }
}

/// Draws a custom high-visibility curved Rotation Mouse Cursor at `pos` when hovering/dragging the rotation gizmo
pub fn draw_rotation_mouse_cursor(p: &Painter, pos: Pos2, angle_badge: Option<f32>) {
    p.circle_filled(pos, 13.0, Color32::from_rgba_unmultiplied(24, 24, 28, 210));
    p.circle_stroke(pos, 13.0, Stroke::new(1.5, Color32::from_rgb(46, 204, 113)));
    draw_curved_rotation_icon(
        p,
        pos,
        7.5,
        true,
        Stroke::new(2.2, Color32::WHITE),
        Color32::from_rgb(46, 204, 113),
    );
    if let Some(deg) = angle_badge {
        let badge_rect = Rect::from_min_size(pos + Vec2::new(16.0, -14.0), Vec2::new(58.0, 20.0));
        p.rect_filled(badge_rect, 4.0, Color32::from_rgb(20, 20, 24));
        p.rect_stroke(badge_rect, 4.0, Stroke::new(1.0, Color32::from_rgb(46, 204, 113)));
        p.text(
            badge_rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("{:+.1}°", deg),
            egui::FontId::proportional(11.5),
            Color32::from_rgb(46, 204, 113),
        );
    }
}
