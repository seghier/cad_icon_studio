use crate::model::{PathCmd, ThemeMode, VectorShape, CANVAS_SIZE};
use eframe::egui::{self, Color32, ColorImage, Context, Painter, Pos2, Rect, Rounding, Stroke, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum PreviewBgStyle {
    CadDarkToolbar,
    CadLightToolbar,
    CadHoverButton,
    CadActiveButton,
    Checkerboard,
}

pub fn cmds_to_tiny_skia_path(cmds: &[PathCmd]) -> Option<tiny_skia::Path> {
    let mut pb = tiny_skia::PathBuilder::new();
    for cmd in cmds {
        match *cmd {
            PathCmd::MoveTo(x, y) => pb.move_to(x, y),
            PathCmd::LineTo(x, y) => pb.line_to(x, y),
            PathCmd::QuadTo(cx, cy, x, y) => pb.quad_to(cx, cy, x, y),
            PathCmd::CubicTo(c1x, c1y, c2x, c2y, x, y) => pb.cubic_to(c1x, c1y, c2x, c2y, x, y),
            PathCmd::Close => pb.close(),
        }
    }
    pb.finish()
}

pub fn cmds_to_svg_d(cmds: &[PathCmd]) -> String {
    let mut out = String::new();
    for cmd in cmds {
        match *cmd {
            PathCmd::MoveTo(x, y) => out.push_str(&format!("M {:.2} {:.2} ", x, y)),
            PathCmd::LineTo(x, y) => out.push_str(&format!("L {:.2} {:.2} ", x, y)),
            PathCmd::QuadTo(cx, cy, x, y) => {
                out.push_str(&format!("Q {:.2} {:.2} {:.2} {:.2} ", cx, cy, x, y))
            }
            PathCmd::CubicTo(c1x, c1y, c2x, c2y, x, y) => out.push_str(&format!(
                "C {:.2} {:.2} {:.2} {:.2} {:.2} {:.2} ",
                c1x, c1y, c2x, c2y, x, y
            )),
            PathCmd::Close => out.push_str("Z "),
        }
    }
    out.trim().to_string()
}

pub fn render_shapes_to_pixmap(
    shapes: &[VectorShape],
    target_size: u32,
    bg_color: Option<[u8; 4]>,
    theme: ThemeMode,
) -> tiny_skia::Pixmap {
    let mut pixmap = tiny_skia::Pixmap::new(target_size, target_size).unwrap();
    if let Some([r, g, b, a]) = bg_color {
        pixmap.fill(tiny_skia::Color::from_rgba8(r, g, b, a));
    } else {
        pixmap.fill(tiny_skia::Color::TRANSPARENT);
    }

    let scale = (target_size as f32) / CANVAS_SIZE;
    let transform = tiny_skia::Transform::from_scale(scale, scale);

    for shape in shapes {
        if !shape.visible {
            continue;
        }

        let fill_c = shape.effective_fill_color(theme);
        let stroke_c = shape.effective_stroke_color(theme);
        let edge_colors = shape.effective_edge_colors(theme);

        // 1. Fill Path
        if shape.fill_enabled && fill_c[3] > 0 {
            let fill_cmds = shape.to_path_cmds(shape.close_path_for_fill);
            if let Some(fill_path) = cmds_to_tiny_skia_path(&fill_cmds) {
                let mut paint = tiny_skia::Paint::default();
                paint.set_color_rgba8(fill_c[0], fill_c[1], fill_c[2], fill_c[3]);
                paint.anti_alias = true;
                pixmap.fill_path(
                    &fill_path,
                    &paint,
                    shape.fill_rule.to_skia(),
                    transform,
                    None,
                );
            }
        }

        // 2. Stroke Path (Unified or Per-Edge Colored)
        if shape.stroke_enabled && shape.stroke_width > 0.0 {
            let mut stroke = tiny_skia::Stroke {
                width: shape.stroke_width,
                line_cap: shape.line_cap.to_skia(),
                line_join: shape.line_join.to_skia(),
                ..Default::default()
            };
            if shape.dashed {
                stroke.dash = tiny_skia::StrokeDash::new(
                    vec![shape.dash_len.max(2.0), shape.gap_len.max(2.0)],
                    0.0,
                );
            }

            let segs = shape.edge_segments();
            if shape.per_edge_colors_enabled && !segs.is_empty() {
                for (e_idx, (p0, p1)) in segs.iter().enumerate() {
                    let col = edge_colors.get(e_idx).copied().unwrap_or(stroke_c);
                    if col[3] == 0 {
                        continue;
                    }
                    let mut pb = tiny_skia::PathBuilder::new();
                    pb.move_to(p0[0], p0[1]);
                    pb.line_to(p1[0], p1[1]);
                    if let Some(seg_path) = pb.finish() {
                        let mut paint = tiny_skia::Paint::default();
                        paint.set_color_rgba8(col[0], col[1], col[2], col[3]);
                        paint.anti_alias = true;
                        pixmap.stroke_path(&seg_path, &paint, &stroke, transform, None);
                    }
                }
            } else if stroke_c[3] > 0 {
                let stroke_cmds = shape.to_path_cmds(false);
                if let Some(stroke_path) = cmds_to_tiny_skia_path(&stroke_cmds) {
                    let mut paint = tiny_skia::Paint::default();
                    paint.set_color_rgba8(stroke_c[0], stroke_c[1], stroke_c[2], stroke_c[3]);
                    paint.anti_alias = true;
                    pixmap.stroke_path(&stroke_path, &paint, &stroke, transform, None);
                }
            }
        }

        // 3. CAD Vertex Nodes
        if shape.show_vertex_nodes && shape.vertex_node_radius > 0.5 {
            let fc = shape.effective_vertex_node_fill(theme);
            let sc = shape.effective_vertex_node_stroke(theme);
            for pt in shape.cad_node_positions() {
                let mut pb = tiny_skia::PathBuilder::new();
                pb.push_circle(pt[0], pt[1], shape.vertex_node_radius);
                if let Some(node_path) = pb.finish() {
                    let mut fill_paint = tiny_skia::Paint::default();
                    fill_paint.set_color_rgba8(fc[0], fc[1], fc[2], fc[3]);
                    fill_paint.anti_alias = true;
                    pixmap.fill_path(
                        &node_path,
                        &fill_paint,
                        tiny_skia::FillRule::Winding,
                        transform,
                        None,
                    );

                    if shape.vertex_node_stroke_width > 0.1 && sc[3] > 0 {
                        let mut stroke_paint = tiny_skia::Paint::default();
                        stroke_paint.set_color_rgba8(sc[0], sc[1], sc[2], sc[3]);
                        stroke_paint.anti_alias = true;
                        let node_stroke = tiny_skia::Stroke {
                            width: shape.vertex_node_stroke_width,
                            ..Default::default()
                        };
                        pixmap.stroke_path(&node_path, &stroke_paint, &node_stroke, transform, None);
                    }
                }
            }
        }
    }

    pixmap
}

pub fn pixmap_to_color_image(pixmap: &tiny_skia::Pixmap) -> ColorImage {
    let pixels: Vec<Color32> = pixmap
        .pixels()
        .iter()
        .map(|p| Color32::from_rgba_premultiplied(p.red(), p.green(), p.blue(), p.alpha()))
        .collect();
    ColorImage {
        size: [pixmap.width() as usize, pixmap.height() as usize],
        pixels,
    }
}

pub fn export_shapes_to_svg(shapes: &[VectorShape], theme: ThemeMode) -> String {
    let mut svg = String::from(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 512 512\" width=\"512\" height=\"512\">\n",
    );

    for shape in shapes {
        if !shape.visible {
            continue;
        }

        let fill_c = shape.effective_fill_color(theme);
        let stroke_c = shape.effective_stroke_color(theme);
        let edge_colors = shape.effective_edge_colors(theme);

        if shape.fill_enabled && fill_c[3] > 0 {
            let d = cmds_to_svg_d(&shape.to_path_cmds(shape.close_path_for_fill));
            if !d.is_empty() {
                svg.push_str(&format!(
                    "  <path d=\"{}\" fill=\"{}\" fill-opacity=\"{:.3}\" fill-rule=\"{}\" stroke=\"none\" />\n",
                    d,
                    rgb_hex(fill_c),
                    fill_c[3] as f32 / 255.0,
                    shape.fill_rule.to_svg()
                ));
            }
        }

        if shape.stroke_enabled && shape.stroke_width > 0.0 {
            let dash = if shape.dashed {
                format!(
                    " stroke-dasharray=\"{:.1} {:.1}\"",
                    shape.dash_len.max(2.0),
                    shape.gap_len.max(2.0)
                )
            } else {
                String::new()
            };
            let segs = shape.edge_segments();
            if shape.per_edge_colors_enabled && !segs.is_empty() {
                for (i, (p0, p1)) in segs.iter().enumerate() {
                    let c = edge_colors.get(i).copied().unwrap_or(stroke_c);
                    svg.push_str(&format!(
                        "  <path d=\"M {:.2} {:.2} L {:.2} {:.2}\" fill=\"none\" stroke=\"{}\" stroke-opacity=\"{:.3}\" stroke-width=\"{:.2}\" stroke-linecap=\"{}\"{} />\n",
                        p0[0], p0[1], p1[0], p1[1], rgb_hex(c), c[3] as f32 / 255.0, shape.stroke_width, shape.line_cap.to_svg(), dash
                    ));
                }
            } else if stroke_c[3] > 0 {
                let d = cmds_to_svg_d(&shape.to_path_cmds(false));
                if !d.is_empty() {
                    svg.push_str(&format!(
                        "  <path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-opacity=\"{:.3}\" stroke-width=\"{:.2}\" stroke-linecap=\"{}\" stroke-linejoin=\"{}\"{} />\n",
                        d, rgb_hex(stroke_c), stroke_c[3] as f32 / 255.0, shape.stroke_width, shape.line_cap.to_svg(), shape.line_join.to_svg(), dash
                    ));
                }
            }
        }

        if shape.show_vertex_nodes && shape.vertex_node_radius > 0.5 {
            let fc = shape.effective_vertex_node_fill(theme);
            let sc = shape.effective_vertex_node_stroke(theme);
            for pt in shape.cad_node_positions() {
                svg.push_str(&format!(
                    "  <circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" fill=\"{}\" fill-opacity=\"{:.3}\" stroke=\"{}\" stroke-opacity=\"{:.3}\" stroke-width=\"{:.2}\" />\n",
                    pt[0], pt[1], shape.vertex_node_radius, rgb_hex(fc), fc[3] as f32 / 255.0, rgb_hex(sc), sc[3] as f32 / 255.0, shape.vertex_node_stroke_width
                ));
            }
        }
    }

    svg.push_str("</svg>\n");
    svg
}

pub fn rgb_hex(c: [u8; 4]) -> String {
    format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}

pub fn draw_preview_background(
    painter: &Painter,
    rect: Rect,
    style: PreviewBgStyle,
    rounding: f32,
) {
    match style {
        PreviewBgStyle::CadDarkToolbar => {
            painter.rect_filled(rect, rounding, Color32::from_rgb(37, 37, 38));
        }
        PreviewBgStyle::CadLightToolbar => {
            painter.rect_filled(rect, rounding, Color32::from_rgb(245, 245, 247));
            painter.rect_stroke(rect, rounding, Stroke::new(1.0, Color32::from_rgb(215, 215, 222)));
        }
        PreviewBgStyle::CadHoverButton => {
            painter.rect_filled(rect, rounding, Color32::from_rgb(62, 62, 66));
            painter.rect_stroke(
                rect,
                rounding,
                Stroke::new(1.0, Color32::from_rgb(95, 95, 102)),
            );
        }
        PreviewBgStyle::CadActiveButton => {
            painter.rect_filled(rect, rounding, Color32::from_rgb(78, 52, 34));
            painter.rect_stroke(
                rect,
                rounding,
                Stroke::new(1.5, Color32::from_rgb(255, 127, 42)),
            );
        }
        PreviewBgStyle::Checkerboard => {
            painter.rect_filled(rect, rounding, Color32::from_rgb(44, 44, 48));
            let cell = 8.0;
            let cols = (rect.width() / cell).ceil() as i32;
            let rows = (rect.height() / cell).ceil() as i32;
            for r in 0..rows {
                for c in 0..cols {
                    if (r + c) % 2 == 0 {
                        let min = Pos2::new(rect.left() + c as f32 * cell, rect.top() + r as f32 * cell);
                        let max = Pos2::new(
                            (min.x + cell).min(rect.right()),
                            (min.y + cell).min(rect.bottom()),
                        );
                        painter.rect_filled(
                            Rect::from_min_max(min, max),
                            0.0,
                            Color32::from_rgb(64, 64, 70),
                        );
                    }
                }
            }
        }
    }
}

pub fn configure_pro_studio_theme(ctx: &Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = Color32::from_rgb(38, 38, 41); // Affinity Dark Studio Panel
    visuals.window_fill = Color32::from_rgb(32, 32, 35);
    visuals.faint_bg_color = Color32::from_rgb(28, 28, 31);
    visuals.extreme_bg_color = Color32::from_rgb(20, 20, 22);
    visuals.selection.bg_fill = Color32::from_rgb(215, 105, 30);
    visuals.selection.stroke = Stroke::new(1.0, Color32::from_rgb(255, 175, 100));
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(44, 44, 48);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(56, 56, 62));
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(52, 52, 57);
    visuals.widgets.inactive.rounding = Rounding::same(4.0);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(68, 68, 75);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(255, 138, 48));
    visuals.widgets.hovered.rounding = Rounding::same(4.0);
    visuals.widgets.active.bg_fill = Color32::from_rgb(255, 127, 42);
    visuals.widgets.active.rounding = Rounding::same(4.0);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(6.0, 5.0);
    style.spacing.button_padding = Vec2::new(7.0, 4.0);
    ctx.set_style(style);
}
