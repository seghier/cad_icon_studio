#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod boolean;
mod icons;
mod model;
mod render;

use boolean::{compute_boolean, divide_two_shapes, BooleanOp};
use eframe::egui::{
    self, Color32, Context, CursorIcon, FontId, Key, Modifiers, Pos2, Rect, RichText, Sense,
    Stroke, TextureHandle, TextureOptions, Ui, Vec2,
};
use icons::{draw_rotation_mouse_cursor, icon_button, VectorIcon};
use model::{
    build_preset, catmull_rom_to_cubic_cmds, dist, dist_to_segment, point_in_contours,
    sample_path_cmds_to_points, CadPreset, CadProject, FillRuleMode, ShapeKind, ThemeMode, VectorShape,
    CANVAS_HALF, CANVAS_SIZE, PREVIEW_SIZES,
};
use render::{
    configure_pro_studio_theme, draw_preview_background, export_shapes_to_svg,
    pixmap_to_color_image, render_shapes_to_pixmap, PreviewBgStyle,
};
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("CAD Icon Studio Pro — Vector Pathfinder, Shape Builder & Multi-Size Studio")
            .with_inner_size([1600.0, 950.0])
            .with_min_inner_size([1200.0, 740.0]),
        ..Default::default()
    };

    eframe::run_native(
        "CAD Icon Studio Pro",
        native_options,
        Box::new(|cc| Ok(Box::new(IconStudioApp::new(cc)))),
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StudioTool {
    MoveTransform,    // V
    NodeCornerEdit,   // A
    DrawNurbsCurve,   // N
    ShapeBuilderFill, // M
    CurveEdgePainter, // I
    AddThreePointArc,
    AddQuad,
    AddCircleArc,
    AddRegularPoly,
    AddPoly,
    AddBezier,
    AddLine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasViewMode {
    SideBySide,
    DarkOnly,
    LightOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewStripMode {
    Dark,
    Light,
    Both,
}

#[derive(Clone, Debug)]
enum DragAction {
    CornerPoint {
        shape_idx: usize,
        pt_idx: usize,
        origin_theme: ThemeMode,
    },
    MoveShape {
        shape_idx: usize,
        start_canvas_pos: [f32; 2],
        orig_shape: VectorShape,
        origin_theme: ThemeMode,
    },
    RotateShape {
        shape_idx: usize,
        pivot: [f32; 2],
        start_angle_deg: f32,
        current_delta_deg: f32,
        orig_shape: VectorShape,
        origin_theme: ThemeMode,
    },
    ScaleBoxHandle {
        shape_idx: usize,
        anchor: [f32; 2],
        start_pos: [f32; 2],
        scale_x: bool,
        scale_y: bool,
        orig_shape: VectorShape,
        origin_theme: ThemeMode,
    },
}

impl DragAction {
    fn origin_theme(&self) -> ThemeMode {
        match *self {
            DragAction::CornerPoint { origin_theme, .. } => origin_theme,
            DragAction::MoveShape { origin_theme, .. } => origin_theme,
            DragAction::RotateShape { origin_theme, .. } => origin_theme,
            DragAction::ScaleBoxHandle { origin_theme, .. } => origin_theme,
        }
    }
}

struct IconStudioApp {
    shapes: Vec<VectorShape>,
    selected_idx: Option<usize>,
    selected_node_idx: Option<usize>,
    boolean_partner_idx: Option<usize>,
    nurbs_points: Vec<[f32; 2]>,
    next_id: u64,

    undo_stack: Vec<Vec<VectorShape>>,
    redo_stack: Vec<Vec<VectorShape>>,

    active_tool: StudioTool,
    active_color_theme: ThemeMode,
    canvas_view_mode: CanvasViewMode,
    preview_strip_mode: PreviewStripMode,

    primary_fill_color: [u8; 4],
    primary_stroke_color: [u8; 4],
    keep_originals_on_boolean: bool,

    drag_action: Option<DragAction>,

    // Artboard View & Grid
    canvas_zoom: f32,
    show_grid: bool,
    snap_to_grid: bool,
    grid_step: f32,
    show_center_axes: bool,
    show_safe_margin: bool,
    show_bounding_gizmo: bool,
    show_corner_indices: bool,
    #[allow(dead_code)]
    canvas_bg_preview: PreviewBgStyle,

    // Textures (Separate for Dark & Light themes)
    dirty_render: bool,
    tex_512_dark: Option<TextureHandle>,
    tex_512_light: Option<TextureHandle>,
    tex_previews_dark: [Option<TextureHandle>; 4],
    tex_previews_light: [Option<TextureHandle>; 4],
    loupe_preview_idx: usize,
    show_loupe_pixel_grid: bool,

    status_msg: String,
    icon_export_name: String,
    current_project_path: Option<PathBuf>,
}

impl IconStudioApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_pro_studio_theme(&cc.egui_ctx);
        let initial = build_preset(CadPreset::ThreePointArc);
        Self {
            shapes: initial,
            selected_idx: Some(1),
            selected_node_idx: None,
            boolean_partner_idx: Some(0),
            nurbs_points: Vec::new(),
            next_id: 100,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            active_tool: StudioTool::MoveTransform,
            active_color_theme: ThemeMode::Dark,
            canvas_view_mode: CanvasViewMode::SideBySide,
            preview_strip_mode: PreviewStripMode::Both,
            primary_fill_color: [255, 127, 42, 200],
            primary_stroke_color: [238, 238, 240, 255],
            keep_originals_on_boolean: true,
            drag_action: None,
            canvas_zoom: 0.95,
            show_grid: true,
            snap_to_grid: false,
            grid_step: 16.0,
            show_center_axes: true,
            show_safe_margin: true,
            show_bounding_gizmo: true,
            show_corner_indices: false,
            canvas_bg_preview: PreviewBgStyle::CadDarkToolbar,
            dirty_render: true,
            tex_512_dark: None,
            tex_512_light: None,
            tex_previews_dark: [None, None, None, None],
            tex_previews_light: [None, None, None, None],
            loupe_preview_idx: 2,
            show_loupe_pixel_grid: true,
            status_msg: "Ready — Dual side-by-side Dark & Light canvas active! Shapes & edits sync live.".to_string(),
            icon_export_name: "cad_tool_icon".to_string(),
            current_project_path: None,
        }
    }

    fn save_undo(&mut self) {
        self.undo_stack.push(self.shapes.clone());
        if self.undo_stack.len() > 60 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
        self.dirty_render = true;
    }

    fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(self.shapes.clone());
            self.shapes = prev;
            self.selected_idx = self.shapes.len().checked_sub(1);
            self.dirty_render = true;
            self.status_msg = "Undo applied.".to_string();
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.shapes.clone());
            self.shapes = next;
            self.dirty_render = true;
            self.status_msg = "Redo applied.".to_string();
        }
    }

    fn snap_pt(&self, p: [f32; 2]) -> [f32; 2] {
        let s = if self.snap_to_grid && self.grid_step > 0.5 {
            self.grid_step
        } else {
            1.0
        };
        [
            ((p[0] / s).round() * s).clamp(0.0, CANVAS_SIZE),
            ((p[1] / s).round() * s).clamp(0.0, CANVAS_SIZE),
        ]
    }

    fn update_textures_if_dirty(&mut self, ctx: &Context) {
        if !self.dirty_render && self.tex_512_dark.is_some() && self.tex_512_light.is_some() {
            return;
        }
        self.dirty_render = false;

        // 1. Dark Theme Textures
        let pix512_dark = render_shapes_to_pixmap(&self.shapes, 512, None, ThemeMode::Dark);
        self.tex_512_dark = Some(ctx.load_texture(
            "canvas_512_dark",
            pixmap_to_color_image(&pix512_dark),
            TextureOptions::LINEAR,
        ));
        for (i, &sz) in PREVIEW_SIZES.iter().enumerate() {
            let pix = render_shapes_to_pixmap(&self.shapes, sz, None, ThemeMode::Dark);
            self.tex_previews_dark[i] = Some(ctx.load_texture(
                format!("preview_dark_{}", sz),
                pixmap_to_color_image(&pix),
                TextureOptions::NEAREST,
            ));
        }

        // 2. Light Theme Textures
        let pix512_light = render_shapes_to_pixmap(&self.shapes, 512, None, ThemeMode::Light);
        self.tex_512_light = Some(ctx.load_texture(
            "canvas_512_light",
            pixmap_to_color_image(&pix512_light),
            TextureOptions::LINEAR,
        ));
        for (i, &sz) in PREVIEW_SIZES.iter().enumerate() {
            let pix = render_shapes_to_pixmap(&self.shapes, sz, None, ThemeMode::Light);
            self.tex_previews_light[i] = Some(ctx.load_texture(
                format!("preview_light_{}", sz),
                pixmap_to_color_image(&pix),
                TextureOptions::NEAREST,
            ));
        }
    }

    // Layer Order Operations (Bring to Front, Up, Down, Send to Back, Duplicate, Delete)
    fn layer_bring_to_front(&mut self) {
        if let Some(i) = self.selected_idx.filter(|&i| i + 1 < self.shapes.len()) {
            self.save_undo();
            let s = self.shapes.remove(i);
            self.shapes.push(s);
            self.selected_idx = Some(self.shapes.len() - 1);
            self.status_msg = "Brought shape to Front.".to_string();
        }
    }

    fn layer_step_up(&mut self) {
        if let Some(i) = self.selected_idx.filter(|&i| i + 1 < self.shapes.len()) {
            self.save_undo();
            self.shapes.swap(i, i + 1);
            self.selected_idx = Some(i + 1);
            self.status_msg = "Moved shape Forward.".to_string();
        }
    }

    fn layer_step_down(&mut self) {
        if let Some(i) = self.selected_idx.filter(|&i| i > 0 && i < self.shapes.len()) {
            self.save_undo();
            self.shapes.swap(i, i - 1);
            self.selected_idx = Some(i - 1);
            self.status_msg = "Moved shape Backward.".to_string();
        }
    }

    fn layer_send_to_back(&mut self) {
        if let Some(i) = self.selected_idx.filter(|&i| i > 0 && i < self.shapes.len()) {
            self.save_undo();
            let s = self.shapes.remove(i);
            self.shapes.insert(0, s);
            self.selected_idx = Some(0);
            self.status_msg = "Sent shape to Back.".to_string();
        }
    }

    fn layer_duplicate(&mut self) {
        if let Some(i) = self.selected_idx.filter(|&i| i < self.shapes.len()) {
            self.save_undo();
            let mut copy = self.shapes[i].clone();
            copy.id = self.next_id;
            self.next_id += 1;
            copy.name = format!("{} Copy", copy.name);
            copy.translate(20.0, 20.0);
            self.shapes.push(copy);
            self.selected_idx = Some(self.shapes.len() - 1);
            self.status_msg = "Duplicated selected shape.".to_string();
        }
    }

    fn delete_selected_node(&mut self) -> bool {
        if let (Some(sel_idx), Some(node_idx)) = (self.selected_idx, self.selected_node_idx) {
            if sel_idx < self.shapes.len() {
                if self.shapes[sel_idx].locked {
                    self.status_msg = "Shape is locked! Unlock it in the Layers panel to edit.".to_string();
                    return false;
                }
                self.save_undo();
                let shape = &mut self.shapes[sel_idx];
                if let ShapeKind::ThreePointArc { .. } = shape.kind {
                    self.status_msg = "3-Point Arc requires 3 definition points (Start, Crest, End).".to_string();
                    return false;
                }
                if let ShapeKind::Quad { corner_radius, .. } = shape.kind {
                    shape.kind = ShapeKind::Poly {
                        closed: true,
                        corner_radius,
                    };
                }
                if shape.points.len() > 2 && node_idx < shape.points.len() {
                    shape.points.remove(node_idx);
                    shape.sync_edge_colors_len();
                    self.selected_node_idx = if node_idx < shape.points.len() {
                        Some(node_idx)
                    } else {
                        shape.points.len().checked_sub(1)
                    };
                    self.dirty_render = true;
                    self.status_msg = format!("Deleted node #{}. Remaining: {}", node_idx, shape.points.len());
                    return true;
                } else if shape.points.len() <= 2 {
                    self.layer_delete();
                    self.selected_node_idx = None;
                    return true;
                }
            }
        }
        false
    }

    fn layer_delete(&mut self) {
        if let Some(i) = self.selected_idx.filter(|&i| i < self.shapes.len()) {
            if self.shapes[i].locked {
                self.status_msg = "Shape is locked! Unlock it in the Layers panel to delete.".to_string();
                return;
            }
            self.save_undo();
            self.shapes.remove(i);
            self.selected_idx = self.shapes.len().checked_sub(1);
            self.selected_node_idx = None;
            self.dirty_render = true;
            self.status_msg = "Deleted shape.".to_string();
        }
    }

    fn fill_selected_closed_region(&mut self) {
        if let Some(i) = self.selected_idx.filter(|&i| i < self.shapes.len()) {
            self.save_undo();
            let col = self.primary_fill_color;
            self.shapes[i].enable_closed_region_fill(col);
            self.dirty_render = true;
            self.status_msg = format!("Filled closed region of '{}'!", self.shapes[i].name);
        }
    }

    fn resolve_boolean_pair(&self) -> Option<(usize, usize)> {
        if self.shapes.len() < 2 {
            return None;
        }
        if let (Some(a), Some(b)) = (self.selected_idx, self.boolean_partner_idx) {
            if a < self.shapes.len() && b < self.shapes.len() && a != b {
                return Some((a, b));
            }
        }
        let vis: Vec<usize> = self
            .shapes
            .iter()
            .enumerate()
            .filter(|(_, s)| s.visible)
            .map(|(i, _)| i)
            .collect();
        if vis.len() >= 2 {
            Some((vis[vis.len() - 2], vis[vis.len() - 1]))
        } else {
            Some((0, 1))
        }
    }

    fn execute_pathfinder(&mut self, op: BooleanOp) {
        let Some((idx_a, idx_b)) = self.resolve_boolean_pair() else {
            self.status_msg = "Need at least 2 shapes for Pathfinder Boolean operations.".to_string();
            return;
        };
        let contours_a = self.shapes[idx_a].to_contours();
        let contours_b = self.shapes[idx_b].to_contours();
        let result_contours = compute_boolean(&contours_a, &contours_b, op);
        if result_contours.is_empty() {
            self.status_msg = format!(
                "Pathfinder {:?} produced an empty region (ensure shapes A and B overlap!).",
                op
            );
            return;
        }

        self.save_undo();
        let id = self.next_id;
        self.next_id += 1;
        let op_name = match op {
            BooleanOp::Union => "Union (A + B)",
            BooleanOp::Subtract => "Subtract (A - B)",
            BooleanOp::Intersect => "Intersect (A & B)",
            BooleanOp::Xor => "Exclude (A ^ B)",
        };
        let mut new_shape = VectorShape::from_contours(id, op_name, result_contours);
        new_shape.fill_enabled = true;
        new_shape.fill_color = self.primary_fill_color;
        new_shape.stroke_enabled = true;
        new_shape.stroke_color = self.primary_stroke_color;
        new_shape.stroke_width = self.shapes[idx_a].stroke_width;

        if !self.keep_originals_on_boolean {
            let max_i = idx_a.max(idx_b);
            let min_i = idx_a.min(idx_b);
            self.shapes.remove(max_i);
            self.shapes.remove(min_i);
        }

        self.shapes.push(new_shape);
        self.selected_idx = Some(self.shapes.len() - 1);
        self.dirty_render = true;
        self.status_msg = format!("Created true vector {} shape!", op_name);
    }

    fn execute_pathfinder_divide(&mut self) {
        let Some((idx_a, idx_b)) = self.resolve_boolean_pair() else {
            return;
        };
        let parts = divide_two_shapes(&self.shapes[idx_a].to_contours(), &self.shapes[idx_b].to_contours());
        if parts.is_empty() {
            self.status_msg = "Shapes do not overlap to divide.".to_string();
            return;
        }
        self.save_undo();
        let palette = [
            [255, 127, 42, 210],
            [56, 182, 255, 170],
            [46, 204, 113, 170],
        ];
        if !self.keep_originals_on_boolean {
            let max_i = idx_a.max(idx_b);
            let min_i = idx_a.min(idx_b);
            self.shapes.remove(max_i);
            self.shapes.remove(min_i);
        }
        for (i, (label, contours)) in parts.into_iter().enumerate() {
            let id = self.next_id;
            self.next_id += 1;
            let mut s = VectorShape::from_contours(id, label, contours);
            s.fill_enabled = true;
            s.fill_color = palette[i % palette.len()];
            s.stroke_color = self.primary_stroke_color;
            self.shapes.push(s);
        }
        self.selected_idx = Some(self.shapes.len() - 1);
        self.dirty_render = true;
        self.status_msg = "Divided overlapping shapes into independent editable regions!".to_string();
    }

    fn find_hit_shape(&self, cpt: [f32; 2]) -> Option<usize> {
        let mut best_hit = None;
        let mut best_stroke_dist = f32::MAX;

        // Check in reverse z-order (topmost first)
        for i in (0..self.shapes.len()).rev() {
            let shape = &self.shapes[i];
            if !shape.visible || shape.locked {
                continue;
            }
            let stroke_tol = (shape.stroke_width * 0.5 + 10.0).max(14.0);
            let d = shape.distance_to_curve(cpt);
            if d <= stroke_tol {
                if d < best_stroke_dist {
                    best_stroke_dist = d;
                    best_hit = Some(i);
                }
            } else if shape.fill_enabled && shape.is_closed() {
                let contours = shape.to_contours();
                if point_in_contours(cpt, &contours) && best_hit.is_none() {
                    best_hit = Some(i);
                }
            }
        }
        best_hit
    }

    fn compute_region_at_point(&self, pt: [f32; 2]) -> Option<(String, Vec<Vec<[f32; 2]>>)> {
        let hits: Vec<usize> = (0..self.shapes.len())
            .filter(|&i| self.shapes[i].visible && point_in_contours(pt, &self.shapes[i].to_contours()))
            .collect();

        if hits.len() >= 2 {
            let idx_a = hits[hits.len() - 2];
            let idx_b = hits[hits.len() - 1];
            let inter = compute_boolean(
                &self.shapes[idx_a].to_contours(),
                &self.shapes[idx_b].to_contours(),
                BooleanOp::Intersect,
            );
            if !inter.is_empty() {
                return Some((
                    format!("Intersection (#{} & #{})", self.shapes[idx_a].id, self.shapes[idx_b].id),
                    inter,
                ));
            }
        } else if hits.len() == 1 {
            let idx_a = hits[0];
            let mut ca = self.shapes[idx_a].to_contours();
            for (other_i, other_s) in self.shapes.iter().enumerate() {
                if other_i != idx_a && other_s.visible {
                    let sub = compute_boolean(&ca, &other_s.to_contours(), BooleanOp::Subtract);
                    if !sub.is_empty() && point_in_contours(pt, &sub) {
                        ca = sub;
                    }
                }
            }
            return Some((format!("Filled Region (#{})", self.shapes[idx_a].id), ca));
        }
        None
    }

    fn spawn_shape(&mut self, tool: StudioTool) {
        self.save_undo();
        let id = self.next_id;
        self.next_id += 1;
        let (cx, cy) = (CANVAS_HALF, CANVAS_HALF);
        let mut s = match tool {
            StudioTool::AddThreePointArc => {
                let mut arc = VectorShape::default_cad_style(
                    id,
                    &format!("3-Point Arc {}", id),
                    ShapeKind::ThreePointArc { close_chord: false },
                    vec![
                        [cx - 150.0, cy + 70.0],
                        [cx, cy - 80.0],
                        [cx + 150.0, cy + 70.0],
                    ],
                );
                arc.stroke_width = 24.0;
                arc.stroke_color = [56, 182, 255, 255];
                arc.show_vertex_nodes = true;
                arc.vertex_node_radius = 28.0;
                arc
            }
            StudioTool::AddQuad => VectorShape::default_cad_style(
                id,
                &format!("Quad {}", id),
                ShapeKind::Quad {
                    lock_rect: false,
                    corner_radius: 0.0,
                },
                vec![
                    [cx - 112.0, cy - 80.0],
                    [cx + 112.0, cy - 80.0],
                    [cx + 112.0, cy + 80.0],
                    [cx - 112.0, cy + 80.0],
                ],
            ),
            StudioTool::AddCircleArc => VectorShape::default_cad_style(
                id,
                &format!("Circle {}", id),
                ShapeKind::CircleArc {
                    lock_circle: true,
                    start_deg: 0.0,
                    sweep_deg: 360.0,
                    close_pie: false,
                },
                vec![[cx, cy], [cx + 128.0, cy], [cx, cy + 128.0]],
            ),
            StudioTool::AddRegularPoly => VectorShape::default_cad_style(
                id,
                &format!("Star {}", id),
                ShapeKind::RegularPoly {
                    sides: 6,
                    inner_ratio: 1.0,
                    corner_radius: 0.0,
                },
                vec![[cx, cy], [cx, cy - 128.0]],
            ),
            StudioTool::AddPoly => VectorShape::default_cad_style(
                id,
                &format!("Polygon {}", id),
                ShapeKind::Poly {
                    closed: true,
                    corner_radius: 0.0,
                },
                vec![
                    [cx, cy - 112.0],
                    [cx + 112.0, cy - 16.0],
                    [cx + 64.0, cy + 112.0],
                    [cx - 64.0, cy + 112.0],
                    [cx - 112.0, cy - 16.0],
                ],
            ),
            StudioTool::AddBezier => VectorShape::default_cad_style(
                id,
                &format!("Bezier {}", id),
                ShapeKind::CubicBezier { closed: false },
                vec![
                    [cx - 128.0, cy + 96.0],
                    [cx - 64.0, cy - 128.0],
                    [cx + 64.0, cy + 128.0],
                    [cx + 128.0, cy - 96.0],
                ],
            ),
            _ => VectorShape::default_cad_style(
                id,
                &format!("Line {}", id),
                ShapeKind::Line {
                    arrow_start: false,
                    arrow_end: true,
                    arrow_size: 56.0,
                },
                vec![[cx - 128.0, cy + 96.0], [cx + 128.0, cy - 96.0]],
            ),
        };
        s.fill_color = self.primary_fill_color;
        s.stroke_color = self.primary_stroke_color;
        self.shapes.push(s);
        self.selected_idx = Some(self.shapes.len() - 1);
        self.active_tool = StudioTool::MoveTransform;
        self.dirty_render = true;
    }

    fn batch_export_both_themes(&mut self) {
        if let Some(folder) = rfd::FileDialog::new()
            .set_title("Select Folder to Export Dark & Light Theme Packs (SVG + PNGs)")
            .pick_folder()
        {
            let base = if self.icon_export_name.trim().is_empty() {
                "cad_icon"
            } else {
                self.icon_export_name.trim()
            };

            // 1. Dark Theme Export
            let dark_base = format!("{}_dark", base);
            let dark_svg: PathBuf = folder.join(format!("{}.svg", dark_base));
            let _ = std::fs::write(&dark_svg, export_shapes_to_svg(&self.shapes, ThemeMode::Dark));
            for &sz in &[512u32, 64, 48, 32, 24] {
                let p = folder.join(format!("{}_{}x{}.png", dark_base, sz, sz));
                let _ = render_shapes_to_pixmap(&self.shapes, sz, None, ThemeMode::Dark).save_png(p);
            }

            // 2. Light Theme Export
            let light_base = format!("{}_light", base);
            let light_svg: PathBuf = folder.join(format!("{}.svg", light_base));
            let _ = std::fs::write(&light_svg, export_shapes_to_svg(&self.shapes, ThemeMode::Light));
            for &sz in &[512u32, 64, 48, 32, 24] {
                let p = folder.join(format!("{}_{}x{}.png", light_base, sz, sz));
                let _ = render_shapes_to_pixmap(&self.shapes, sz, None, ThemeMode::Light).save_png(p);
            }

            self.status_msg = format!("Exported Full Dual Theme Pack (Dark & Light SVG + 512..24px PNGs) to {}", folder.display());
        }
    }

    fn export_single_theme_pack(&mut self, theme: ThemeMode) {
        if let Some(folder) = rfd::FileDialog::new()
            .set_title(&format!("Select Folder to Export {} Pack (SVG + PNGs)", theme.label()))
            .pick_folder()
        {
            let base = if self.icon_export_name.trim().is_empty() {
                "cad_icon"
            } else {
                self.icon_export_name.trim()
            };
            let file_base = format!("{}_{}", base, theme.suffix());
            let svg_path: PathBuf = folder.join(format!("{}.svg", file_base));
            let _ = std::fs::write(&svg_path, export_shapes_to_svg(&self.shapes, theme));
            for &sz in &[512u32, 64, 48, 32, 24] {
                let p = folder.join(format!("{}_{}x{}.png", file_base, sz, sz));
                let _ = render_shapes_to_pixmap(&self.shapes, sz, None, theme).save_png(p);
            }
            self.status_msg = format!("Exported {} pack (SVG + 512..24px PNGs) to {}", theme.label(), folder.display());
        }
    }

    #[allow(dead_code)]
    fn batch_export_all(&mut self) {
        self.batch_export_both_themes();
    }

    fn save_project_to_path(&mut self, path: &PathBuf) -> bool {
        let proj = CadProject {
            name: self.icon_export_name.clone(),
            canvas_size: CANVAS_SIZE,
            shapes: self.shapes.clone(),
            primary_fill_color: Some(self.primary_fill_color),
            primary_stroke_color: Some(self.primary_stroke_color),
            grid_step: Some(self.grid_step),
            snap_to_grid: Some(self.snap_to_grid),
        };
        match proj.to_json_pretty() {
            Ok(json) => match std::fs::write(path, json) {
                Ok(_) => {
                    self.current_project_path = Some(path.clone());
                    let filename = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("project.json");
                    self.status_msg = format!(
                        "Project saved successfully to '{}' ({} shapes).",
                        filename,
                        self.shapes.len()
                    );
                    true
                }
                Err(e) => {
                    self.status_msg = format!("Error writing project file: {}", e);
                    false
                }
            },
            Err(e) => {
                self.status_msg = format!("Error serializing project: {}", e);
                false
            }
        }
    }

    fn save_project_as(&mut self) {
        let default_name = if self.icon_export_name.trim().is_empty() {
            "cad_project.json".to_string()
        } else {
            format!("{}.json", self.icon_export_name.trim())
        };

        if let Some(path) = rfd::FileDialog::new()
            .set_title("Save CAD Icon Studio Project (.json)")
            .add_filter("CAD Project (*.json, *.cadicon)", &["json", "cadicon"])
            .set_file_name(&default_name)
            .save_file()
        {
            self.save_project_to_path(&path);
        }
    }

    fn save_project_quick(&mut self) {
        if let Some(path) = self.current_project_path.clone() {
            self.save_project_to_path(&path);
        } else {
            self.save_project_as();
        }
    }

    fn load_project_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Open CAD Icon Studio Project (.json)")
            .add_filter("CAD Project (*.json, *.cadicon)", &["json", "cadicon"])
            .pick_file()
        {
            match std::fs::read_to_string(&path) {
                Ok(content) => match CadProject::from_json_str(&content) {
                    Ok(proj) => {
                        self.save_undo();
                        self.shapes = proj.shapes;
                        self.icon_export_name = proj.name;
                        if let Some(col) = proj.primary_fill_color {
                            self.primary_fill_color = col;
                        }
                        if let Some(col) = proj.primary_stroke_color {
                            self.primary_stroke_color = col;
                        }
                        if let Some(gs) = proj.grid_step {
                            self.grid_step = gs;
                        }
                        if let Some(stg) = proj.snap_to_grid {
                            self.snap_to_grid = stg;
                        }
                        self.current_project_path = Some(path.clone());
                        self.selected_idx = self.shapes.len().checked_sub(1);
                        self.selected_node_idx = None;
                        self.boolean_partner_idx = None;
                        self.next_id = self.shapes.iter().map(|s| s.id).max().unwrap_or(0) + 1;
                        self.dirty_render = true;
                        let filename = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("project.json");
                        self.status_msg = format!(
                            "Loaded project '{}' with {} shapes!",
                            filename,
                            self.shapes.len()
                        );
                    }
                    Err(e) => {
                        self.status_msg = format!("Failed to parse project file: {}", e);
                    }
                },
                Err(e) => {
                    self.status_msg = format!("Failed to read file: {}", e);
                }
            }
        }
    }
}

impl eframe::App for IconStudioApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        if !ctx.wants_keyboard_input() {
            ctx.input(|i| {
                if i.key_pressed(Key::V) {
                    self.active_tool = StudioTool::MoveTransform;
                } else if i.key_pressed(Key::A) {
                    self.active_tool = StudioTool::NodeCornerEdit;
                } else if i.key_pressed(Key::M) {
                    self.active_tool = StudioTool::ShapeBuilderFill;
                } else if i.key_pressed(Key::I) {
                    self.active_tool = StudioTool::CurveEdgePainter;
                }
            });
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Z)) {
            self.undo();
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Y)) {
            self.redo();
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::S)) {
            self.save_project_quick();
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::O)) {
            self.load_project_dialog();
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::N)) {
            self.active_tool = StudioTool::DrawNurbsCurve;
            self.nurbs_points.clear();
            self.status_msg = "NURBS Spline Tool (N): Click canvas to place points. Enter or Double-Click to finish, Esc to cancel.".to_string();
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) {
            if self.active_tool == StudioTool::DrawNurbsCurve && self.nurbs_points.len() >= 2 {
                self.save_undo();
                let id = self.next_id;
                self.next_id += 1;
                let pts = std::mem::take(&mut self.nurbs_points);
                let mut s = VectorShape::default_cad_style(
                    id,
                    &format!("NURBS Curve {}", id),
                    ShapeKind::NurbsCurve { closed: false },
                    pts,
                );
                s.stroke_color = self.primary_stroke_color;
                s.stroke_width = 20.0;
                s.show_vertex_nodes = true;
                s.vertex_node_radius = 24.0;
                self.shapes.push(s);
                self.selected_idx = Some(self.shapes.len() - 1);
                self.active_tool = StudioTool::NodeCornerEdit;
                self.dirty_render = true;
                self.status_msg = format!("Created NURBS curve with {} points! Direct Node Tool active.", self.shapes.last().unwrap().points.len());
            }
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Delete))
            || ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Backspace))
        {
            if self.active_tool == StudioTool::NodeCornerEdit && self.selected_node_idx.is_some() {
                self.delete_selected_node();
            } else {
                self.layer_delete();
            }
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            if self.active_tool == StudioTool::DrawNurbsCurve {
                self.nurbs_points.clear();
                self.active_tool = StudioTool::MoveTransform;
                self.status_msg = "Cancelled NURBS drawing.".to_string();
            } else {
                self.selected_idx = None;
                self.selected_node_idx = None;
                self.boolean_partner_idx = None;
            }
        }

        // Enforce invariant: hidden, locked, or out-of-range shapes cannot remain selected
        if let Some(i) = self.selected_idx {
            if i >= self.shapes.len() || !self.shapes[i].visible || self.shapes[i].locked {
                self.selected_idx = None;
                self.selected_node_idx = None;
            }
        }
        if let Some(i) = self.boolean_partner_idx {
            if i >= self.shapes.len() || !self.shapes[i].visible || self.shapes[i].locked {
                self.boolean_partner_idx = None;
            }
        }
        if let (Some(sel), Some(node_i)) = (self.selected_idx, self.selected_node_idx) {
            if sel >= self.shapes.len() || node_i >= self.shapes[sel].points.len() {
                self.selected_node_idx = None;
            }
        }

        self.update_textures_if_dirty(ctx);

        egui::TopBottomPanel::top("top_context_bar")
            .exact_height(46.0)
            .show(ctx, |ui| {
                self.draw_top_context_bar(ui);
            });

        egui::TopBottomPanel::bottom("bottom_status_bar")
            .exact_height(26.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(
                        RichText::new("CAD ICON STUDIO PRO")
                            .strong()
                            .color(Color32::from_rgb(255, 138, 48)),
                    );
                    ui.separator();
                    ui.label(&self.status_msg);
                });
            });

        egui::SidePanel::left("left_tool_dock")
            .exact_width(58.0)
            .resizable(false)
            .show(ctx, |ui| {
                self.draw_left_tool_dock(ui);
            });

        egui::SidePanel::right("right_studio_dock")
            .default_width(390.0)
            .min_width(350.0)
            .show(ctx, |ui| {
                self.draw_right_studio_dock(ui);
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::from_rgb(24, 24, 27)))
            .show(ctx, |ui| {
                self.draw_center_artboard(ui);
            });
    }
}

impl IconStudioApp {
    fn draw_left_tool_dock(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(6.0);
            let tools = [
                (
                    StudioTool::MoveTransform,
                    VectorIcon::ToolMove,
                    "Move & Transform Tool (V) — Click & drag anywhere on a shape to move, scale, or rotate",
                ),
                (
                    StudioTool::NodeCornerEdit,
                    VectorIcon::ToolNode,
                    "Direct Corner / Node Tool (A) — Drag individual corners or double-click an edge to insert a corner",
                ),
                (
                    StudioTool::DrawNurbsCurve,
                    VectorIcon::ToolNurbs,
                    "NURBS Spline Tool (N) — Click canvas to insert points, Enter/Esc or double-click to finish",
                ),
                (
                    StudioTool::ShapeBuilderFill,
                    VectorIcon::ToolBucketFill,
                    "Bucket Fill Closed Region / Intersection (M) — Click inside any closed shape or overlap (A & B) to fill it!",
                ),
                (
                    StudioTool::CurveEdgePainter,
                    VectorIcon::ToolEdgePaint,
                    "Curve & Edge Color Painter (I) — Click any curve or edge to paint it",
                ),
            ];
            for (t, ico, tip) in tools {
                if icon_button(ui, ico, "", Vec2::new(42.0, 35.0), self.active_tool == t, tip).clicked() {
                    self.active_tool = t;
                    if t == StudioTool::NodeCornerEdit {
                        if let Some(sel) = self.selected_idx {
                            if sel < self.shapes.len() && self.shapes[sel].visible {
                                self.shapes[sel].show_vertex_nodes = true;
                                self.dirty_render = true;
                            }
                        }
                        self.status_msg = "Direct Node Tool (A): Corner nodes active! Drag nodes to move, double-click edge to insert, Del to delete.".to_string();
                    } else if t == StudioTool::DrawNurbsCurve {
                        self.nurbs_points.clear();
                        self.status_msg = "NURBS Spline Tool (N): Click anywhere on canvas to place control points. Enter or Double-Click to finish, Esc to cancel.".to_string();
                    } else if t == StudioTool::MoveTransform {
                        self.selected_node_idx = None;
                    }
                    if let Some(sel) = self.selected_idx {
                        if sel >= self.shapes.len() || !self.shapes[sel].visible || self.shapes[sel].locked {
                            self.selected_idx = None;
                            self.selected_node_idx = None;
                        }
                    }
                }
            }

            ui.separator();

            let add_tools = [
                (StudioTool::AddThreePointArc, VectorIcon::ShapeArc, "Add 3-Point CAD Arc (Start, Mid, End)"),
                (StudioTool::AddCircleArc, VectorIcon::ShapeCircleArc, "Add Circle / CAD Arc"),
                (StudioTool::AddQuad, VectorIcon::ShapeQuad, "Add Quad / Rectangle"),
                (StudioTool::AddRegularPoly, VectorIcon::ShapeStar, "Add Star / Hexagon"),
                (StudioTool::AddPoly, VectorIcon::ShapePoly, "Add Multi-Corner Polygon"),
                (StudioTool::AddBezier, VectorIcon::ShapeBezier, "Add Cubic Bezier Curve"),
                (StudioTool::AddLine, VectorIcon::ShapeLineArrow, "Add Line / Extrude Arrow"),
            ];
            for (t, ico, tip) in add_tools {
                if icon_button(ui, ico, "", Vec2::new(42.0, 31.0), false, tip).clicked() {
                    self.spawn_shape(t);
                }
            }

            ui.separator();
            ui.label(RichText::new("Fill/Strk").size(10.0));
            if ui
                .color_edit_button_srgba_unmultiplied(&mut self.primary_fill_color)
                .on_hover_text("Primary Fill Color (Active Theme)")
                .changed()
            {
                if let Some(s_i) = self.selected_idx.filter(|&idx| idx < self.shapes.len()) {
                    if self.shapes[s_i].fill_enabled {
                        match self.active_color_theme {
                            ThemeMode::Dark => self.shapes[s_i].fill_color = self.primary_fill_color,
                            ThemeMode::Light => self.shapes[s_i].light_fill_color = Some(self.primary_fill_color),
                        }
                        self.dirty_render = true;
                    }
                }
            }
            if ui
                .color_edit_button_srgba_unmultiplied(&mut self.primary_stroke_color)
                .on_hover_text("Primary Curve / Stroke Color (Active Theme)")
                .changed()
            {
                if let Some(s_i) = self.selected_idx.filter(|&idx| idx < self.shapes.len()) {
                    match self.active_color_theme {
                        ThemeMode::Dark => self.shapes[s_i].stroke_color = self.primary_stroke_color,
                        ThemeMode::Light => self.shapes[s_i].light_stroke_color = Some(self.primary_stroke_color),
                    }
                    self.dirty_render = true;
                }
            }
            if ui
                .small_button("Swap")
                .on_hover_text("Swap Fill & Stroke Colors")
                .clicked()
            {
                std::mem::swap(&mut self.primary_fill_color, &mut self.primary_stroke_color);
                if let Some(idx) = self.selected_idx.filter(|&i| i < self.shapes.len()) {
                    self.save_undo();
                    self.shapes[idx].fill_color = self.primary_fill_color;
                    self.shapes[idx].stroke_color = self.primary_stroke_color;
                }
            }
        });
    }

    fn draw_top_context_bar(&mut self, ui: &mut Ui) {
        ui.horizontal_centered(|ui| {
            egui::ComboBox::from_id_salt("presets_combo")
                .selected_text("Templates")
                .width(100.0)
                .show_ui(ui, |ui| {
                    if ui.button("Boolean Intersect Venn").clicked() {
                        self.save_undo();
                        self.shapes = build_preset(CadPreset::BooleanIntersectLens);
                        self.selected_idx = Some(2);
                        self.boolean_partner_idx = Some(0);
                    }
                    if ui.button("Extrude Push/Pull").clicked() {
                        self.save_undo();
                        self.shapes = build_preset(CadPreset::ExtrudePushPull);
                        self.selected_idx = Some(2);
                    }
                    if ui.button("3-Point Arc + Nodes").clicked() {
                        self.save_undo();
                        self.shapes = build_preset(CadPreset::ThreePointArc);
                        self.selected_idx = Some(1);
                    }
                    if ui.button("Isometric Cube").clicked() {
                        self.save_undo();
                        self.shapes = build_preset(CadPreset::IsometricCube);
                        self.selected_idx = Some(1);
                    }
                    if ui.button("Multi-Color Edges Quad").clicked() {
                        self.save_undo();
                        self.shapes = build_preset(CadPreset::MultiColorEdges);
                        self.selected_idx = Some(0);
                    }
                    if ui.button("Clear Blank Canvas").clicked() {
                        self.save_undo();
                        self.shapes.clear();
                        self.selected_idx = None;
                    }
                });

            if ui
                .add_enabled(!self.undo_stack.is_empty(), egui::Button::new("Undo"))
                .clicked()
            {
                self.undo();
            }
            if ui
                .add_enabled(!self.redo_stack.is_empty(), egui::Button::new("Redo"))
                .clicked()
            {
                self.redo();
            }

            ui.separator();
            if icon_button(ui, VectorIcon::OpenProject, "Load JSON", Vec2::new(98.0, 28.0), false, "Open CAD Icon Studio Project (Ctrl+O)").clicked() {
                self.load_project_dialog();
            }
            if icon_button(ui, VectorIcon::SaveProject, "Save JSON", Vec2::new(98.0, 28.0), false, "Save CAD Icon Studio Project (Ctrl+S)").clicked() {
                self.save_project_quick();
            }

            ui.separator();

            // PATHFINDER BOOLEAN BAR WITH VECTOR ICONS
            ui.label(
                RichText::new("Boolean:")
                    .strong()
                    .color(Color32::from_rgb(255, 165, 90)),
            );
            if icon_button(ui, VectorIcon::BoolUnion, "Add", Vec2::new(56.0, 28.0), false, "Boolean Union (A + B)").clicked() {
                self.execute_pathfinder(BooleanOp::Union);
            }
            if icon_button(ui, VectorIcon::BoolSubtract, "Sub", Vec2::new(56.0, 28.0), false, "Boolean Subtract (A - B)").clicked() {
                self.execute_pathfinder(BooleanOp::Subtract);
            }
            if icon_button(ui, VectorIcon::BoolIntersect, "Inter", Vec2::new(62.0, 28.0), false, "Boolean Intersect (A & B)").clicked() {
                self.execute_pathfinder(BooleanOp::Intersect);
            }
            if icon_button(ui, VectorIcon::BoolXor, "Xor", Vec2::new(54.0, 28.0), false, "Boolean Exclude (A ^ B)").clicked() {
                self.execute_pathfinder(BooleanOp::Xor);
            }
            if icon_button(ui, VectorIcon::BoolDivide, "Div", Vec2::new(54.0, 28.0), false, "Divide overlapping shapes into separate parts").clicked() {
                self.execute_pathfinder_divide();
            }
            ui.checkbox(&mut self.keep_originals_on_boolean, "Keep");

            ui.separator();

            // FILL CLOSED REGION QUICK BUTTON
            if icon_button(
                ui,
                VectorIcon::FillRegion,
                "Fill Region",
                Vec2::new(92.0, 28.0),
                false,
                "Closes and fills the selected shape/curve with the active Fill Color",
            )
            .clicked()
            {
                self.fill_selected_closed_region();
            }

            ui.separator();

            // LAYER ORDER BUTTONS WITH VECTOR ICONS
            ui.label(RichText::new("Order:").strong());
            if icon_button(ui, VectorIcon::OrderFront, "", Vec2::new(28.0, 28.0), false, "Bring to Front").clicked() {
                self.layer_bring_to_front();
            }
            if icon_button(ui, VectorIcon::OrderUp, "", Vec2::new(28.0, 28.0), false, "Bring Forward (Up)").clicked() {
                self.layer_step_up();
            }
            if icon_button(ui, VectorIcon::OrderDown, "", Vec2::new(28.0, 28.0), false, "Send Backward (Down)").clicked() {
                self.layer_step_down();
            }
            if icon_button(ui, VectorIcon::OrderBack, "", Vec2::new(28.0, 28.0), false, "Send to Back").clicked() {
                self.layer_send_to_back();
            }
            if icon_button(ui, VectorIcon::Duplicate, "", Vec2::new(28.0, 28.0), false, "Duplicate Shape").clicked() {
                self.layer_duplicate();
            }
            if icon_button(ui, VectorIcon::Trash, "", Vec2::new(28.0, 28.0), false, "Delete Shape (Del)").clicked() {
                self.layer_delete();
            }

            ui.separator();

            // ROTATE & FLIP BUTTONS WITH VECTOR ICONS
            if icon_button(ui, VectorIcon::RotateCCW, "-45", Vec2::new(52.0, 28.0), false, "Rotate -45 deg").clicked() {
                if let Some(i) = self.selected_idx.filter(|&i| i < self.shapes.len()) {
                    self.save_undo();
                    let c = self.shapes[i].center();
                    self.shapes[i].rotate(-45.0, c);
                }
            }
            if icon_button(ui, VectorIcon::RotateCW, "+45", Vec2::new(52.0, 28.0), false, "Rotate +45 deg").clicked() {
                if let Some(i) = self.selected_idx.filter(|&i| i < self.shapes.len()) {
                    self.save_undo();
                    let c = self.shapes[i].center();
                    self.shapes[i].rotate(45.0, c);
                }
            }
            if icon_button(ui, VectorIcon::FlipH, "", Vec2::new(28.0, 28.0), false, "Flip Horizontal").clicked() {
                if let Some(i) = self.selected_idx.filter(|&i| i < self.shapes.len()) {
                    self.save_undo();
                    let c = self.shapes[i].center();
                    self.shapes[i].mirror(true, c);
                }
            }
            if icon_button(ui, VectorIcon::FlipV, "", Vec2::new(28.0, 28.0), false, "Flip Vertical").clicked() {
                if let Some(i) = self.selected_idx.filter(|&i| i < self.shapes.len()) {
                    self.save_undo();
                    let c = self.shapes[i].center();
                    self.shapes[i].mirror(false, c);
                }
            }
        });
    }

    fn draw_right_studio_dock(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            // 1. ALWAYS-VISIBLE LAYERS & ORDER PANEL
            ui.label(RichText::new("Shapes & Layers Order (Click = Select A, 'B' = Boolean Partner)").strong());
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    if icon_button(ui, VectorIcon::OrderFront, "Front", Vec2::new(58.0, 24.0), false, "Bring to Front").clicked() {
                        self.layer_bring_to_front();
                    }
                    if icon_button(ui, VectorIcon::OrderUp, "Up", Vec2::new(46.0, 24.0), false, "Move Up").clicked() {
                        self.layer_step_up();
                    }
                    if icon_button(ui, VectorIcon::OrderDown, "Down", Vec2::new(58.0, 24.0), false, "Move Down").clicked() {
                        self.layer_step_down();
                    }
                    if icon_button(ui, VectorIcon::OrderBack, "Back", Vec2::new(54.0, 24.0), false, "Send to Back").clicked() {
                        self.layer_send_to_back();
                    }
                    if icon_button(ui, VectorIcon::Duplicate, "Dup", Vec2::new(50.0, 24.0), false, "Duplicate").clicked() {
                        self.layer_duplicate();
                    }
                    if icon_button(ui, VectorIcon::Trash, "Del", Vec2::new(48.0, 24.0), false, "Delete").clicked() {
                        self.layer_delete();
                    }
                });
                ui.separator();

                for i in (0..self.shapes.len()).rev() {
                    let is_a = self.selected_idx == Some(i);
                    let is_b = self.boolean_partner_idx == Some(i);
                    let is_locked = self.shapes[i].locked;
                    ui.horizontal(|ui| {
                        if ui.checkbox(&mut self.shapes[i].visible, "").changed() {
                            if !self.shapes[i].visible {
                                if self.selected_idx == Some(i) {
                                    self.selected_idx = None;
                                }
                                if self.boolean_partner_idx == Some(i) {
                                    self.boolean_partner_idx = None;
                                }
                            }
                            self.dirty_render = true;
                        }

                        // Lock layer toggle button next to checkbox
                        let lock_ico = if is_locked {
                            VectorIcon::LockClosed
                        } else {
                            VectorIcon::LockOpen
                        };
                        if icon_button(
                            ui,
                            lock_ico,
                            "",
                            Vec2::new(22.0, 20.0),
                            is_locked,
                            if is_locked {
                                "Layer is Locked — click to Unlock"
                            } else {
                                "Lock Layer (prevents canvas selection & dragging)"
                            },
                        )
                        .clicked()
                        {
                            self.shapes[i].locked = !is_locked;
                            if self.shapes[i].locked {
                                if self.selected_idx == Some(i) {
                                    self.selected_idx = None;
                                    self.selected_node_idx = None;
                                }
                                if self.boolean_partner_idx == Some(i) {
                                    self.boolean_partner_idx = None;
                                }
                            }
                        }

                        if ui
                            .selectable_label(is_b, RichText::new("[B]").strong())
                            .on_hover_text("Click to set as Boolean Partner Shape B")
                            .clicked()
                        {
                            if self.shapes[i].visible && !self.shapes[i].locked {
                                self.boolean_partner_idx = Some(i);
                            }
                        }
                        let label = format!(
                            "#{} {}{}{}",
                            self.shapes[i].id,
                            self.shapes[i].name,
                            if is_locked { " 🔒" } else { "" },
                            if is_a { "  [Selected A]" } else { "" }
                        );
                        if ui.selectable_label(is_a, label).clicked() {
                            if self.shapes[i].visible && !self.shapes[i].locked {
                                self.selected_idx = Some(i);
                            } else if self.shapes[i].locked {
                                self.status_msg = format!("Shape #{} is locked! Click 🔒 to unlock.", self.shapes[i].id);
                            }
                        }
                    });
                }
            });

            ui.add_space(6.0);

            // 2. FILL CLOSED REGION, SWATCHES & STROKE STUDIO (WITH THEME OVERRIDES)
            if let Some(sel_idx) = self.selected_idx.filter(|&i| i < self.shapes.len()) {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    // Theme Switcher & Status Row
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Theme Color Target:").strong());
                        if ui
                            .selectable_label(self.active_color_theme == ThemeMode::Dark, "🌙 Dark Theme")
                            .on_hover_text("Edit dark-mode icon colors")
                            .clicked()
                        {
                            self.active_color_theme = ThemeMode::Dark;
                        }
                        if ui
                            .selectable_label(self.active_color_theme == ThemeMode::Light, "☀️ Light Theme")
                            .on_hover_text("Edit light-mode icon colors (overrides auto-invert)")
                            .clicked()
                        {
                            self.active_color_theme = ThemeMode::Light;
                        }
                    });

                    let has_light_ov = self.shapes[sel_idx].has_light_overrides();
                    if self.active_color_theme == ThemeMode::Light {
                        ui.horizontal(|ui| {
                            if has_light_ov {
                                ui.label(
                                    RichText::new("🎨 Custom Light Colors Active")
                                        .size(11.0)
                                        .color(Color32::from_rgb(255, 140, 40)),
                                );
                                if ui
                                    .button(RichText::new("↺ Reset to Auto-Invert").size(11.0))
                                    .on_hover_text("Clear custom overrides; smart auto-inversion will be used")
                                    .clicked()
                                {
                                    self.save_undo();
                                    self.shapes[sel_idx].clear_light_overrides();
                                    self.dirty_render = true;
                                    self.status_msg = "Reset Light Theme colors to smart auto-inversion.".to_string();
                                }
                            } else {
                                ui.label(
                                    RichText::new("⚡ Smart Auto-Inverted from Dark Theme")
                                        .size(11.0)
                                        .italics()
                                        .color(Color32::LIGHT_GRAY),
                                );
                            }
                        });
                    }

                    ui.separator();

                    ui.label(RichText::new(format!(
                        "CAD Swatches ({} Mode — L-Click: Fill, R-Click: Stroke):",
                        self.active_color_theme.label()
                    )).strong());

                    let swatches: &[(&str, [u8; 4])] = if self.active_color_theme == ThemeMode::Dark {
                        &[
                            ("CAD Orange", [255, 127, 42, 255]),
                            ("Orange Fill", [255, 127, 42, 180]),
                            ("CAD White", [238, 238, 242, 255]),
                            ("Vector Cyan", [56, 182, 255, 255]),
                            ("Cyan Fill", [56, 182, 255, 150]),
                            ("Axis Red", [235, 77, 75, 255]),
                            ("Axis Green", [46, 204, 113, 255]),
                            ("Gold", [241, 196, 15, 255]),
                        ]
                    } else {
                        &[
                            ("CAD Orange", [255, 127, 42, 255]),
                            ("Charcoal Line", [26, 26, 30, 255]),
                            ("Medium Gray", [90, 90, 100, 255]),
                            ("Soft Blue Fill", [56, 140, 240, 140]),
                            ("Vibrant Blue", [0, 120, 215, 255]),
                            ("Soft Orange Fill", [255, 127, 42, 140]),
                            ("CAD Red", [215, 45, 40, 255]),
                            ("CAD Green", [35, 160, 85, 255]),
                        ]
                    };

                    ui.horizontal_wrapped(|ui| {
                        for &(name, col) in swatches {
                            let c32 = Color32::from_rgba_unmultiplied(col[0], col[1], col[2], col[3]);
                            let (r, resp) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::click());
                            ui.painter().rect_filled(r, 3.0, c32);
                            ui.painter().rect_stroke(r, 3.0, Stroke::new(1.0, Color32::GRAY));
                            let resp = resp.on_hover_text(name);
                            if resp.clicked() {
                                self.save_undo();
                                match self.active_color_theme {
                                    ThemeMode::Dark => {
                                        self.primary_fill_color = col;
                                        self.shapes[sel_idx].enable_closed_region_fill(col);
                                    }
                                    ThemeMode::Light => {
                                        self.shapes[sel_idx].light_fill_color = Some(col);
                                        self.shapes[sel_idx].fill_enabled = true;
                                    }
                                }
                                self.dirty_render = true;
                            }
                            if resp.secondary_clicked() {
                                self.save_undo();
                                match self.active_color_theme {
                                    ThemeMode::Dark => {
                                        self.primary_stroke_color = col;
                                        self.shapes[sel_idx].stroke_color = col;
                                        self.shapes[sel_idx].stroke_enabled = true;
                                    }
                                    ThemeMode::Light => {
                                        self.shapes[sel_idx].light_stroke_color = Some(col);
                                        self.shapes[sel_idx].stroke_enabled = true;
                                    }
                                }
                                self.dirty_render = true;
                            }
                        }
                    });

                    ui.separator();

                    // Dedicated Fill Closed Region Row + Button
                    let mut trigger_fill_region = false;
                    {
                        let theme = self.active_color_theme;
                        let shape = &mut self.shapes[sel_idx];
                        ui.horizontal(|ui| {
                            if ui
                                .checkbox(&mut shape.fill_enabled, RichText::new("Closed Fill:").strong())
                                .changed()
                            {
                                self.dirty_render = true;
                            }

                            match theme {
                                ThemeMode::Dark => {
                                    if ui
                                        .color_edit_button_srgba_unmultiplied(&mut shape.fill_color)
                                        .changed()
                                    {
                                        self.primary_fill_color = shape.fill_color;
                                        shape.fill_enabled = true;
                                        self.dirty_render = true;
                                    }
                                }
                                ThemeMode::Light => {
                                    let mut cur = shape.effective_fill_color(ThemeMode::Light);
                                    if ui.color_edit_button_srgba_unmultiplied(&mut cur).changed() {
                                        shape.light_fill_color = Some(cur);
                                        shape.fill_enabled = true;
                                        self.dirty_render = true;
                                    }
                                }
                            }

                            if icon_button(
                                ui,
                                VectorIcon::FillRegion,
                                "Fill Closed Region",
                                Vec2::new(132.0, 24.0),
                                shape.fill_enabled,
                                "One-click: closes and fills the interior of this shape/arc/curve",
                            )
                            .clicked()
                            {
                                trigger_fill_region = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Fill Rule:");
                            if ui
                                .selectable_value(&mut shape.fill_rule, FillRuleMode::EvenOdd, "Even-Odd")
                                .changed()
                            {
                                self.dirty_render = true;
                            }
                            if ui
                                .selectable_value(&mut shape.fill_rule, FillRuleMode::Winding, "Winding")
                                .changed()
                            {
                                self.dirty_render = true;
                            }
                            if ui
                                .checkbox(&mut shape.close_path_for_fill, "Auto-Close Curve")
                                .changed()
                            {
                                self.dirty_render = true;
                            }
                        });
                    }
                    if trigger_fill_region {
                        self.fill_selected_closed_region();
                    }

                    ui.separator();

                    let mut convert_to_free = false;
                    {
                        let theme = self.active_color_theme;
                        let shape = &mut self.shapes[sel_idx];
                        ui.horizontal(|ui| {
                            if ui
                                .checkbox(&mut shape.stroke_enabled, RichText::new("Curve / Stroke:").strong())
                                .changed()
                            {
                                self.dirty_render = true;
                            }

                            match theme {
                                ThemeMode::Dark => {
                                    if ui
                                        .color_edit_button_srgba_unmultiplied(&mut shape.stroke_color)
                                        .changed()
                                    {
                                        self.primary_stroke_color = shape.stroke_color;
                                        self.dirty_render = true;
                                    }
                                }
                                ThemeMode::Light => {
                                    let mut cur = shape.effective_stroke_color(ThemeMode::Light);
                                    if ui.color_edit_button_srgba_unmultiplied(&mut cur).changed() {
                                        shape.light_stroke_color = Some(cur);
                                        self.dirty_render = true;
                                    }
                                }
                            }

                            if ui.checkbox(&mut shape.dashed, "Dashed").changed() {
                                self.dirty_render = true;
                            }
                        });

                        if ui
                            .add(egui::Slider::new(&mut shape.stroke_width, 1.0..=64.0).text("Stroke Width"))
                            .changed()
                        {
                            self.dirty_render = true;
                        }

                        let seg_count = shape.edge_segments().len();
                        if seg_count > 1 {
                            if ui
                                .checkbox(&mut shape.per_edge_colors_enabled, "Per-Edge Individual Colors")
                                .changed()
                            {
                                shape.sync_edge_colors_len();
                                self.dirty_render = true;
                            }
                            if shape.per_edge_colors_enabled {
                                shape.sync_edge_colors_len();
                                ui.horizontal_wrapped(|ui| {
                                    for e_i in 0..seg_count {
                                        ui.label(format!("E#{}:", e_i));
                                        match theme {
                                            ThemeMode::Dark => {
                                                if ui
                                                    .color_edit_button_srgba_unmultiplied(&mut shape.edge_colors[e_i])
                                                    .changed()
                                                {
                                                    self.dirty_render = true;
                                                }
                                            }
                                            ThemeMode::Light => {
                                                let mut eff_edges = shape.effective_edge_colors(ThemeMode::Light);
                                                if ui
                                                    .color_edit_button_srgba_unmultiplied(&mut eff_edges[e_i])
                                                    .changed()
                                                {
                                                    shape.light_edge_colors = Some(eff_edges);
                                                    self.dirty_render = true;
                                                }
                                            }
                                        }
                                    }
                                });
                            }
                        }

                        ui.separator();
                        ui.horizontal(|ui| {
                            if ui
                                .checkbox(&mut shape.show_vertex_nodes, RichText::new("CAD Corner Nodes").strong())
                                .changed()
                            {
                                self.dirty_render = true;
                            }
                            if shape.show_vertex_nodes {
                                if ui
                                    .add(egui::DragValue::new(&mut shape.vertex_node_radius).speed(0.5).prefix("R:"))
                                    .changed()
                                {
                                    self.dirty_render = true;
                                }

                                match theme {
                                    ThemeMode::Dark => {
                                        if ui
                                            .color_edit_button_srgba_unmultiplied(&mut shape.vertex_node_fill)
                                            .changed()
                                        {
                                            self.dirty_render = true;
                                        }
                                    }
                                    ThemeMode::Light => {
                                        let mut cur_vn = shape.effective_vertex_node_fill(ThemeMode::Light);
                                        if ui
                                            .color_edit_button_srgba_unmultiplied(&mut cur_vn)
                                            .changed()
                                        {
                                            shape.light_vertex_node_fill = Some(cur_vn);
                                            self.dirty_render = true;
                                        }
                                    }
                                }
                            }
                        });

                        ui.separator();
                        let mut arc_angles_changed = false;
                        match &mut shape.kind {
                            ShapeKind::Quad { lock_rect, corner_radius } => {
                                ui.checkbox(lock_rect, "Lock Orthogonal Box");
                                if ui.add(egui::Slider::new(corner_radius, 0.0..=160.0).text("Fillet")).changed() {
                                    self.dirty_render = true;
                                }
                            }
                            ShapeKind::Poly { closed, corner_radius } => {
                                if ui.checkbox(closed, "Closed Polygon").changed() {
                                    self.dirty_render = true;
                                }
                                if ui.add(egui::Slider::new(corner_radius, 0.0..=160.0).text("Fillet")).changed() {
                                    self.dirty_render = true;
                                }
                            }
                            ShapeKind::CompoundPoly { corner_radius, .. } => {
                                if ui.add(egui::Slider::new(corner_radius, 0.0..=120.0).text("Fillet")).changed() {
                                    self.dirty_render = true;
                                }
                            }
                            ShapeKind::CircleArc { lock_circle, start_deg, sweep_deg, close_pie } => {
                                ui.checkbox(lock_circle, "Lock 1:1 Circle");
                                if ui.add(egui::Slider::new(start_deg, 0.0..=360.0).text("Start deg")).changed() {
                                    arc_angles_changed = true;
                                }
                                if ui.add(egui::Slider::new(sweep_deg, -360.0..=360.0).text("Sweep deg")).changed() {
                                    arc_angles_changed = true;
                                }
                                if ui.checkbox(close_pie, "Close Pie Sector").changed() {
                                    self.dirty_render = true;
                                }
                                if ui.button("Convert Arc/Circle to Editable Corners").clicked() {
                                    convert_to_free = true;
                                }
                            }
                            ShapeKind::RegularPoly { sides, inner_ratio, corner_radius } => {
                                if ui.add(egui::Slider::new(sides, 3..=16).text("Sides")).changed() {
                                    self.dirty_render = true;
                                }
                                if ui.add(egui::Slider::new(inner_ratio, 0.15..=1.0).text("Star Ratio")).changed() {
                                    self.dirty_render = true;
                                }
                                if ui.add(egui::Slider::new(corner_radius, 0.0..=100.0).text("Fillet")).changed() {
                                    self.dirty_render = true;
                                }
                                if ui.button("Convert to Editable Corners").clicked() {
                                    convert_to_free = true;
                                }
                            }
                            ShapeKind::Line { arrow_start, arrow_end, arrow_size } => {
                                ui.horizontal(|ui| {
                                    if ui.checkbox(arrow_start, "Start Arrow").changed() {
                                        self.dirty_render = true;
                                    }
                                    if ui.checkbox(arrow_end, "End Arrow").changed() {
                                        self.dirty_render = true;
                                    }
                                });
                                if ui.add(egui::Slider::new(arrow_size, 16.0..=140.0).text("Arrow Size")).changed() {
                                    self.dirty_render = true;
                                }
                            }
                            ShapeKind::CubicBezier { closed } => {
                                if ui.checkbox(closed, "Closed Bezier").changed() {
                                    self.dirty_render = true;
                                }
                            }
                            ShapeKind::ThreePointArc { close_chord } => {
                                if ui.checkbox(close_chord, "Close Chord Line").changed() {
                                    self.dirty_render = true;
                                }
                                ui.label(RichText::new("CAD 3-Point Arc (#0: Start, #1: Crest, #2: End)").italics());
                            }
                            ShapeKind::NurbsCurve { closed } => {
                                if ui.checkbox(closed, "Closed NURBS Curve").changed() {
                                    self.dirty_render = true;
                                }
                                ui.label(format!("NURBS Control Points: {}", shape.points.len()));
                            }
                        }
                        if arc_angles_changed {
                            shape.sync_arc_handles_to_angles();
                            self.dirty_render = true;
                        }
                    }
                    if convert_to_free {
                        self.save_undo();
                        self.shapes[sel_idx].convert_to_free_corners();
                        self.dirty_render = true;
                    }
                });
            }

            ui.add_space(6.0);

            // 3. LIVE MULTI-SIZE PREVIEWS (64x64 to 24x24 px) + PIXEL LOUPE + EXPORT
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Live Toolbar Previews:").strong());
                    if ui.selectable_label(self.preview_strip_mode == PreviewStripMode::Both, "🌓 Both").clicked() {
                        self.preview_strip_mode = PreviewStripMode::Both;
                    }
                    if ui.selectable_label(self.preview_strip_mode == PreviewStripMode::Dark, "🌙 Dark").clicked() {
                        self.preview_strip_mode = PreviewStripMode::Dark;
                    }
                    if ui.selectable_label(self.preview_strip_mode == PreviewStripMode::Light, "☀️ Light").clicked() {
                        self.preview_strip_mode = PreviewStripMode::Light;
                    }
                });

                let show_dark = self.preview_strip_mode == PreviewStripMode::Dark || self.preview_strip_mode == PreviewStripMode::Both;
                let show_light = self.preview_strip_mode == PreviewStripMode::Light || self.preview_strip_mode == PreviewStripMode::Both;

                if show_dark {
                    ui.label(RichText::new("🌙 Dark Toolbar (#252526):").size(11.0).color(Color32::from_rgb(255, 160, 50)));
                    ui.horizontal(|ui| {
                        for (i, &sz) in PREVIEW_SIZES.iter().enumerate() {
                            ui.vertical(|ui| {
                                let box_sz = Vec2::splat(sz as f32 + 10.0);
                                let (rect, resp) = ui.allocate_exact_size(box_sz, Sense::click());
                                draw_preview_background(ui.painter(), rect, PreviewBgStyle::CadDarkToolbar, 4.0);
                                if self.loupe_preview_idx == i {
                                    ui.painter().rect_stroke(rect, 4.0, Stroke::new(1.5, Color32::from_rgb(255, 138, 48)));
                                }
                                let ico = Rect::from_center_size(rect.center(), Vec2::splat(sz as f32));
                                if let Some(tex) = &self.tex_previews_dark[i] {
                                    ui.painter().image(
                                        tex.id(),
                                        ico,
                                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                        Color32::WHITE,
                                    );
                                }
                                if resp.clicked() {
                                    self.loupe_preview_idx = i;
                                }
                                ui.label(RichText::new(format!("{}px", sz)).size(10.0));
                            });
                        }
                    });
                }

                if show_light {
                    ui.label(RichText::new("☀️ Light Toolbar (#F5F5F7):").size(11.0).color(Color32::from_rgb(255, 200, 60)));
                    ui.horizontal(|ui| {
                        for (i, &sz) in PREVIEW_SIZES.iter().enumerate() {
                            ui.vertical(|ui| {
                                let box_sz = Vec2::splat(sz as f32 + 10.0);
                                let (rect, resp) = ui.allocate_exact_size(box_sz, Sense::click());
                                draw_preview_background(ui.painter(), rect, PreviewBgStyle::CadLightToolbar, 4.0);
                                if self.loupe_preview_idx == i {
                                    ui.painter().rect_stroke(rect, 4.0, Stroke::new(1.5, Color32::from_rgb(255, 138, 48)));
                                }
                                let ico = Rect::from_center_size(rect.center(), Vec2::splat(sz as f32));
                                if let Some(tex) = &self.tex_previews_light[i] {
                                    ui.painter().image(
                                        tex.id(),
                                        ico,
                                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                        Color32::WHITE,
                                    );
                                }
                                if resp.clicked() {
                                    self.loupe_preview_idx = i;
                                }
                                ui.label(RichText::new(format!("{}px", sz)).size(10.0));
                            });
                        }
                    });
                }

                let active_sz = PREVIEW_SIZES[self.loupe_preview_idx];
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Pixel Loupe ({}x{}):", active_sz, active_sz)).strong());
                    ui.checkbox(&mut self.show_loupe_pixel_grid, "1px Grid");
                });

                let side = 168.0;
                ui.horizontal(|ui| {
                    if show_dark {
                        ui.vertical(|ui| {
                            let (l_rect, _) = ui.allocate_exact_size(Vec2::splat(side), Sense::hover());
                            draw_preview_background(ui.painter(), l_rect, PreviewBgStyle::CadDarkToolbar, 4.0);
                            if let Some(tex) = &self.tex_previews_dark[self.loupe_preview_idx] {
                                ui.painter().image(
                                    tex.id(),
                                    l_rect,
                                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                    Color32::WHITE,
                                );
                            }
                            if self.show_loupe_pixel_grid {
                                let step = side / (active_sz as f32);
                                let gs = Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 22));
                                for k in 1..active_sz {
                                    let off = k as f32 * step;
                                    ui.painter().line_segment(
                                        [Pos2::new(l_rect.left() + off, l_rect.top()), Pos2::new(l_rect.left() + off, l_rect.bottom())],
                                        gs,
                                    );
                                    ui.painter().line_segment(
                                        [Pos2::new(l_rect.left(), l_rect.top() + off), Pos2::new(l_rect.right(), l_rect.top() + off)],
                                        gs,
                                    );
                                }
                            }
                            ui.label(RichText::new("Dark 8x Loupe").size(10.0));
                        });
                    }

                    if show_light {
                        ui.vertical(|ui| {
                            let (l_rect, _) = ui.allocate_exact_size(Vec2::splat(side), Sense::hover());
                            draw_preview_background(ui.painter(), l_rect, PreviewBgStyle::CadLightToolbar, 4.0);
                            if let Some(tex) = &self.tex_previews_light[self.loupe_preview_idx] {
                                ui.painter().image(
                                    tex.id(),
                                    l_rect,
                                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                    Color32::WHITE,
                                );
                            }
                            if self.show_loupe_pixel_grid {
                                let step = side / (active_sz as f32);
                                let gs = Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 0, 0, 25));
                                for k in 1..active_sz {
                                    let off = k as f32 * step;
                                    ui.painter().line_segment(
                                        [Pos2::new(l_rect.left() + off, l_rect.top()), Pos2::new(l_rect.left() + off, l_rect.bottom())],
                                        gs,
                                    );
                                    ui.painter().line_segment(
                                        [Pos2::new(l_rect.left(), l_rect.top() + off), Pos2::new(l_rect.right(), l_rect.top() + off)],
                                        gs,
                                    );
                                }
                            }
                            ui.label(RichText::new("Light 8x Loupe").size(10.0));
                        });
                    }
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.icon_export_name);
                });

                ui.add_space(3.0);
                ui.horizontal(|ui| {
                    if icon_button(ui, VectorIcon::SaveProject, "Save", Vec2::new(76.0, 25.0), false, "Save project to JSON (Ctrl+S)").clicked() {
                        self.save_project_quick();
                    }
                    if icon_button(ui, VectorIcon::OpenProject, "Load", Vec2::new(76.0, 25.0), false, "Open / Load project from JSON (Ctrl+O)").clicked() {
                        self.load_project_dialog();
                    }
                    if ui.button("Save As...").on_hover_text("Save project with new filename").clicked() {
                        self.save_project_as();
                    }
                });

                ui.separator();
                ui.add_space(2.0);
                if ui
                    .button(RichText::new("🚀 Batch Export Both Themes (Dark & Light SVG + PNGs)").strong())
                    .on_hover_text("Exports SVG and 512, 64, 48, 32, 24px PNGs for BOTH Dark and Light themes!")
                    .clicked()
                {
                    self.batch_export_both_themes();
                }

                ui.horizontal(|ui| {
                    if ui.button("🌙 Export Dark Pack").clicked() {
                        self.export_single_theme_pack(ThemeMode::Dark);
                    }
                    if ui.button("☀️ Export Light Pack").clicked() {
                        self.export_single_theme_pack(ThemeMode::Light);
                    }
                });
            });
        });
    }

    // ========================================================================
    // CENTER ARTBOARD (DUAL SIDE-BY-SIDE OR SINGLE THEME)
    // ========================================================================

    fn draw_center_artboard(&mut self, ui: &mut Ui) {
        egui::Frame::none()
            .fill(Color32::from_rgb(33, 33, 37))
            .inner_margin(4.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.show_grid, "Grid");
                    ui.checkbox(&mut self.snap_to_grid, "Snap to Grid");
                    egui::ComboBox::from_id_salt("grid_step")
                        .selected_text(format!("{} px", self.grid_step as i32))
                        .width(70.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.grid_step, 8.0, "8 px");
                            ui.selectable_value(&mut self.grid_step, 16.0, "16 px");
                            ui.selectable_value(&mut self.grid_step, 32.0, "32 px");
                        });
                    ui.checkbox(&mut self.show_center_axes, "Axes");
                    ui.checkbox(&mut self.show_safe_margin, "Margin");
                    ui.checkbox(&mut self.show_bounding_gizmo, "Transform Box");
                    ui.checkbox(&mut self.show_corner_indices, "Node #");
                    ui.separator();
                    ui.label("Zoom:");
                    ui.add(egui::Slider::new(&mut self.canvas_zoom, 0.5..=1.6).show_value(false));

                    ui.separator();
                    ui.label(RichText::new("Canvas:").strong());
                    if ui
                        .selectable_label(self.canvas_view_mode == CanvasViewMode::SideBySide, "🌓 Dual Side-by-Side")
                        .on_hover_text("Display Dark and Light theme canvases side by side")
                        .clicked()
                    {
                        self.canvas_view_mode = CanvasViewMode::SideBySide;
                    }
                    if ui
                        .selectable_label(self.canvas_view_mode == CanvasViewMode::DarkOnly, "🌙 Dark")
                        .on_hover_text("Display Dark theme canvas only")
                        .clicked()
                    {
                        self.canvas_view_mode = CanvasViewMode::DarkOnly;
                    }
                    if ui
                        .selectable_label(self.canvas_view_mode == CanvasViewMode::LightOnly, "☀️ Light")
                        .on_hover_text("Display Light theme canvas only")
                        .clicked()
                    {
                        self.canvas_view_mode = CanvasViewMode::LightOnly;
                    }
                });
            });

        let avail = ui.available_rect_before_wrap();
        let spacing = 28.0;
        let title_h = 24.0;

        let mut artboards: Vec<(ThemeMode, Rect, &'static str)> = Vec::new();
        let scale: f32;
        let dark_rect_store: Rect;
        let light_rect_store: Rect;

        match self.canvas_view_mode {
            CanvasViewMode::SideBySide => {
                let max_w = (avail.width() - spacing - 24.0) * 0.5;
                let max_h = avail.height() - title_h - 36.0;
                let display_side = (CANVAS_SIZE * self.canvas_zoom).min(max_w).min(max_h).max(180.0);
                scale = display_side / CANVAS_SIZE;
                let total_w = display_side * 2.0 + spacing;
                let start_x = (avail.center().x - total_w * 0.5).max(avail.left() + 12.0);
                let y_top = avail.top() + title_h + 10.0 + ((avail.height() - title_h - 10.0 - display_side) * 0.5).max(0.0);
                let d_rect = Rect::from_min_size(Pos2::new(start_x, y_top), Vec2::splat(display_side));
                let l_rect = Rect::from_min_size(Pos2::new(start_x + display_side + spacing, y_top), Vec2::splat(display_side));
                dark_rect_store = d_rect;
                light_rect_store = l_rect;
                artboards.push((ThemeMode::Dark, d_rect, "🌙 Dark Theme (#252526)"));
                artboards.push((ThemeMode::Light, l_rect, "☀️ Light Theme (#F5F5F7)"));
            }
            CanvasViewMode::DarkOnly => {
                let max_w = avail.width() - 32.0;
                let max_h = avail.height() - title_h - 36.0;
                let display_side = (CANVAS_SIZE * self.canvas_zoom).min(max_w).min(max_h).max(200.0);
                scale = display_side / CANVAS_SIZE;
                let y_top = avail.top() + title_h + 10.0 + ((avail.height() - title_h - 10.0 - display_side) * 0.5).max(0.0);
                let d_rect = Rect::from_min_size(Pos2::new(avail.center().x - display_side * 0.5, y_top), Vec2::splat(display_side));
                dark_rect_store = d_rect;
                light_rect_store = d_rect;
                artboards.push((ThemeMode::Dark, d_rect, "🌙 Dark Theme (#252526)"));
            }
            CanvasViewMode::LightOnly => {
                let max_w = avail.width() - 32.0;
                let max_h = avail.height() - title_h - 36.0;
                let display_side = (CANVAS_SIZE * self.canvas_zoom).min(max_w).min(max_h).max(200.0);
                scale = display_side / CANVAS_SIZE;
                let y_top = avail.top() + title_h + 10.0 + ((avail.height() - title_h - 10.0 - display_side) * 0.5).max(0.0);
                let l_rect = Rect::from_min_size(Pos2::new(avail.center().x - display_side * 0.5, y_top), Vec2::splat(display_side));
                dark_rect_store = l_rect;
                light_rect_store = l_rect;
                artboards.push((ThemeMode::Light, l_rect, "☀️ Light Theme (#F5F5F7)"));
            }
        }

        let (_, resp) = ui.allocate_exact_size(avail.size(), Sense::click_and_drag());
        let painter = ui.painter_at(avail);

        let get_rect_for_theme = |t: ThemeMode| -> Rect {
            match t {
                ThemeMode::Dark => dark_rect_store,
                ThemeMode::Light => light_rect_store,
            }
        };

        let to_screen_theme = |p: [f32; 2], art_rect: Rect| -> Pos2 {
            Pos2::new(art_rect.left() + p[0] * scale, art_rect.top() + p[1] * scale)
        };

        let to_canvas_theme = |pos: Pos2, art_rect: Rect| -> [f32; 2] {
            [
                ((pos.x - art_rect.left()) / scale).clamp(0.0, CANVAS_SIZE),
                ((pos.y - art_rect.top()) / scale).clamp(0.0, CANVAS_SIZE),
            ]
        };

        let which_canvas_at = |pos: Pos2| -> Option<(ThemeMode, Rect)> {
            for &(theme, art_rect, _) in &artboards {
                if art_rect.contains(pos) {
                    return Some((theme, art_rect));
                }
            }
            None
        };

        // Render Artboards
        for &(theme, artboard_rect, label) in &artboards {
            let is_active_theme = self.active_color_theme == theme;

            // Header Banner
            let header_rect = Rect::from_min_max(
                Pos2::new(artboard_rect.left(), artboard_rect.top() - 25.0),
                Pos2::new(artboard_rect.right(), artboard_rect.top() - 5.0),
            );
            let header_bg = if is_active_theme {
                Color32::from_rgb(45, 45, 54)
            } else {
                Color32::from_rgb(28, 28, 32)
            };
            painter.rect_filled(header_rect, 4.0, header_bg);
            let header_stroke = if is_active_theme {
                Stroke::new(1.0, Color32::from_rgb(255, 127, 42))
            } else {
                Stroke::new(1.0, Color32::from_rgb(55, 55, 62))
            };
            painter.rect_stroke(header_rect, 4.0, header_stroke);

            let header_text = if is_active_theme {
                format!("{}  •  [ACTIVE COLOR TARGET]", label)
            } else {
                label.to_string()
            };
            painter.text(
                header_rect.center(),
                egui::Align2::CENTER_CENTER,
                header_text,
                FontId::proportional(11.5),
                if is_active_theme { Color32::from_rgb(255, 175, 75) } else { Color32::LIGHT_GRAY },
            );

            // Drop Shadow
            painter.rect_filled(
                artboard_rect.translate(Vec2::new(5.0, 7.0)),
                4.0,
                Color32::from_rgba_unmultiplied(0, 0, 0, 115),
            );

            // Canvas Background
            match theme {
                ThemeMode::Dark => draw_preview_background(&painter, artboard_rect, PreviewBgStyle::CadDarkToolbar, 0.0),
                ThemeMode::Light => draw_preview_background(&painter, artboard_rect, PreviewBgStyle::CadLightToolbar, 0.0),
            }

            // Rulers
            let ruler_bg = match theme {
                ThemeMode::Dark => Color32::from_rgb(32, 32, 36),
                ThemeMode::Light => Color32::from_rgb(228, 228, 234),
            };
            let ruler_fg = match theme {
                ThemeMode::Dark => Color32::LIGHT_GRAY,
                ThemeMode::Light => Color32::DARK_GRAY,
            };
            let top_ruler = Rect::from_min_max(
                Pos2::new(artboard_rect.left(), artboard_rect.top() - 18.0),
                Pos2::new(artboard_rect.right(), artboard_rect.top()),
            );
            let left_ruler = Rect::from_min_max(
                Pos2::new(artboard_rect.left() - 18.0, artboard_rect.top()),
                Pos2::new(artboard_rect.left(), artboard_rect.bottom()),
            );
            painter.rect_filled(top_ruler, 0.0, ruler_bg);
            painter.rect_filled(left_ruler, 0.0, ruler_bg);
            for tick in (0..=512).step_by(64) {
                let sx = artboard_rect.left() + (tick as f32) * scale;
                let sy = artboard_rect.top() + (tick as f32) * scale;
                painter.line_segment(
                    [Pos2::new(sx, top_ruler.bottom() - 5.0), Pos2::new(sx, top_ruler.bottom())],
                    Stroke::new(1.0, Color32::GRAY),
                );
                painter.text(
                    Pos2::new(sx + 2.0, top_ruler.top() + 2.0),
                    egui::Align2::LEFT_TOP,
                    format!("{}", tick),
                    FontId::proportional(9.0),
                    ruler_fg,
                );
                painter.line_segment(
                    [Pos2::new(left_ruler.right() - 5.0, sy), Pos2::new(left_ruler.right(), sy)],
                    Stroke::new(1.0, Color32::GRAY),
                );
            }

            // Grid & Guides
            if self.show_grid && self.grid_step >= 4.0 {
                let gs = match theme {
                    ThemeMode::Dark => Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 14)),
                    ThemeMode::Light => Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 0, 0, 22)),
                };
                let mut g = self.grid_step;
                while g < CANVAS_SIZE {
                    painter.line_segment(
                        [to_screen_theme([g, 0.0], artboard_rect), to_screen_theme([g, CANVAS_SIZE], artboard_rect)],
                        gs,
                    );
                    painter.line_segment(
                        [to_screen_theme([0.0, g], artboard_rect), to_screen_theme([CANVAS_SIZE, g], artboard_rect)],
                        gs,
                    );
                    g += self.grid_step;
                }
            }

            if self.show_safe_margin {
                let m_col = match theme {
                    ThemeMode::Dark => Color32::from_rgba_unmultiplied(56, 182, 255, 45),
                    ThemeMode::Light => Color32::from_rgba_unmultiplied(0, 120, 215, 65),
                };
                painter.rect_stroke(
                    Rect::from_min_max(
                        to_screen_theme([32.0, 32.0], artboard_rect),
                        to_screen_theme([480.0, 480.0], artboard_rect),
                    ),
                    0.0,
                    Stroke::new(1.0, m_col),
                );
            }

            if self.show_center_axes {
                let ax_g = match theme {
                    ThemeMode::Dark => Color32::from_rgba_unmultiplied(46, 204, 113, 85),
                    ThemeMode::Light => Color32::from_rgba_unmultiplied(39, 174, 96, 120),
                };
                let ax_r = match theme {
                    ThemeMode::Dark => Color32::from_rgba_unmultiplied(235, 77, 75, 85),
                    ThemeMode::Light => Color32::from_rgba_unmultiplied(192, 57, 43, 120),
                };
                painter.line_segment(
                    [
                        to_screen_theme([CANVAS_HALF, 0.0], artboard_rect),
                        to_screen_theme([CANVAS_HALF, CANVAS_SIZE], artboard_rect),
                    ],
                    Stroke::new(1.0, ax_g),
                );
                painter.line_segment(
                    [
                        to_screen_theme([0.0, CANVAS_HALF], artboard_rect),
                        to_screen_theme([CANVAS_SIZE, CANVAS_HALF], artboard_rect),
                    ],
                    Stroke::new(1.0, ax_r),
                );
            }

            // Rendered Geometry Texture
            match theme {
                ThemeMode::Dark => {
                    if let Some(tex) = &self.tex_512_dark {
                        painter.image(
                            tex.id(),
                            artboard_rect,
                            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                }
                ThemeMode::Light => {
                    if let Some(tex) = &self.tex_512_light {
                        painter.image(
                            tex.id(),
                            artboard_rect,
                            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                }
            }

            // Outer Border
            let border_stroke = if is_active_theme {
                Stroke::new(2.0, Color32::from_rgb(255, 127, 42))
            } else {
                match theme {
                    ThemeMode::Dark => Stroke::new(1.2, Color32::from_rgb(90, 90, 98)),
                    ThemeMode::Light => Stroke::new(1.2, Color32::from_rgb(185, 185, 195)),
                }
            };
            painter.rect_stroke(artboard_rect, 0.0, border_stroke);
        }

        // Draw Interactive Overlays (Handles, Bounding Boxes, NURBS Preview, etc.) on Artboards
        let is_node_tool = self.active_tool == StudioTool::NodeCornerEdit;
        let mut rot_handles_by_theme: Vec<(ThemeMode, Pos2)> = Vec::new();
        let mut scale_handles_all: Vec<(ThemeMode, Pos2, [f32; 2], [f32; 2], bool, bool)> = Vec::new();

        for &(theme, artboard_rect, _) in &artboards {
            // Live NURBS Drawing Preview
            if self.active_tool == StudioTool::DrawNurbsCurve {
                let hover_cpt = resp.hover_pos().and_then(|hp| {
                    if artboard_rect.contains(hp) {
                        Some(to_canvas_theme(hp, artboard_rect))
                    } else {
                        None
                    }
                });
                let snapped_hover = hover_cpt.map(|c| {
                    if self.snap_to_grid {
                        let g = self.grid_step;
                        [(c[0] / g).round() * g, (c[1] / g).round() * g]
                    } else {
                        c
                    }
                });

                if !self.nurbs_points.is_empty() {
                    let mut preview_pts = self.nurbs_points.clone();
                    if let Some(h) = snapped_hover {
                        preview_pts.push(h);
                    }

                    if preview_pts.len() >= 2 {
                        let preview_cmds = catmull_rom_to_cubic_cmds(&preview_pts, false, false);
                        let sampled = sample_path_cmds_to_points(&preview_cmds);
                        let screen_pts: Vec<Pos2> = sampled
                            .iter()
                            .map(|&p| to_screen_theme(p, artboard_rect))
                            .collect();
                        painter.add(egui::Shape::line(
                            screen_pts,
                            Stroke::new(3.0, Color32::from_rgb(56, 220, 255)),
                        ));
                    }

                    for (idx, &pt) in self.nurbs_points.iter().enumerate() {
                        let sp = to_screen_theme(pt, artboard_rect);
                        painter.circle_filled(sp, 7.5, Color32::from_rgb(255, 127, 42));
                        painter.circle_stroke(sp, 7.5, Stroke::new(2.0, Color32::WHITE));
                        painter.text(
                            sp + Vec2::new(8.0, -8.0),
                            egui::Align2::LEFT_BOTTOM,
                            format!("#{}", idx),
                            FontId::proportional(11.0),
                            Color32::YELLOW,
                        );
                    }

                    if let (Some(last_pt), Some(h)) = (self.nurbs_points.last(), snapped_hover) {
                        painter.line_segment(
                            [to_screen_theme(*last_pt, artboard_rect), to_screen_theme(h, artboard_rect)],
                            Stroke::new(1.5, Color32::from_rgba_unmultiplied(255, 255, 255, 140)),
                        );
                    }
                }

                if let Some(h) = snapped_hover {
                    let shp = to_screen_theme(h, artboard_rect);
                    painter.circle_filled(shp, 5.0, Color32::from_rgb(56, 182, 255));
                    painter.circle_stroke(shp, 5.0, Stroke::new(1.5, Color32::WHITE));
                }
            }

            // Shape Builder Hover Highlight
            if self.active_tool == StudioTool::ShapeBuilderFill {
                if let Some(hover_pos) = resp.hover_pos() {
                    if artboard_rect.contains(hover_pos) {
                        let cpt = to_canvas_theme(hover_pos, artboard_rect);
                        if let Some((_name, region_contours)) = self.compute_region_at_point(cpt) {
                            for c in &region_contours {
                                let screen_pts: Vec<Pos2> = c.iter().map(|&p| to_screen_theme(p, artboard_rect)).collect();
                                painter.add(egui::Shape::closed_line(
                                    screen_pts,
                                    Stroke::new(3.0, Color32::from_rgb(56, 220, 255)),
                                ));
                            }
                        }
                    }
                }
            }

            // Selection Box & Handles
            if let Some(sel_idx) = self.selected_idx.filter(|&i| i < self.shapes.len() && self.shapes[i].visible && !self.shapes[i].locked) {
                let shape = &self.shapes[sel_idx];
                let (bmin, bmax) = shape.bounding_box();
                let box_rect = Rect::from_min_max(
                    to_screen_theme(bmin, artboard_rect),
                    to_screen_theme(bmax, artboard_rect),
                );

                if let ShapeKind::CubicBezier { .. } = shape.kind {
                    if shape.points.len() >= 4 {
                        let s = Stroke::new(1.5, Color32::from_rgb(56, 182, 255));
                        painter.line_segment([to_screen_theme(shape.points[0], artboard_rect), to_screen_theme(shape.points[1], artboard_rect)], s);
                        painter.line_segment([to_screen_theme(shape.points[3], artboard_rect), to_screen_theme(shape.points[2], artboard_rect)], s);
                    }
                }

                let show_box = self.show_bounding_gizmo && !is_node_tool;

                if show_box {
                    let box_stroke_col = match theme {
                        ThemeMode::Dark => Color32::from_rgba_unmultiplied(56, 182, 255, 180),
                        ThemeMode::Light => Color32::from_rgba_unmultiplied(0, 120, 215, 200),
                    };
                    painter.rect_stroke(box_rect, 0.0, Stroke::new(1.2, box_stroke_col));

                    let top_mid = Pos2::new(box_rect.center().x, box_rect.top());
                    let rot_pos = Pos2::new(box_rect.center().x, box_rect.top() - 28.0);
                    painter.line_segment([top_mid, rot_pos], Stroke::new(1.5, box_stroke_col));
                    painter.circle_filled(rot_pos, 7.5, Color32::from_rgb(46, 204, 113));
                    painter.circle_stroke(rot_pos, 7.5, Stroke::new(1.5, Color32::WHITE));
                    rot_handles_by_theme.push((theme, rot_pos));

                    let cx = (bmin[0] + bmax[0]) * 0.5;
                    let cy = (bmin[1] + bmax[1]) * 0.5;
                    let specs = [
                        ([bmin[0], bmin[1]], [bmax[0], bmax[1]], true, true),
                        ([bmax[0], bmin[1]], [bmin[0], bmax[1]], true, true),
                        ([bmax[0], bmax[1]], [bmin[0], bmin[1]], true, true),
                        ([bmin[0], bmax[1]], [bmax[0], bmin[1]], true, true),
                        ([cx, bmin[1]], [cx, bmax[1]], false, true),
                        ([cx, bmax[1]], [cx, bmin[1]], false, true),
                        ([bmin[0], cy], [bmax[0], cy], true, false),
                        ([bmax[0], cy], [bmin[0], cy], true, false),
                    ];
                    for (h_pt, anchor, sx, sy) in specs {
                        let sp = to_screen_theme(h_pt, artboard_rect);
                        painter.rect_filled(Rect::from_center_size(sp, Vec2::splat(7.5)), 1.0, Color32::WHITE);
                        painter.rect_stroke(
                            Rect::from_center_size(sp, Vec2::splat(7.5)),
                            1.0,
                            Stroke::new(1.2, Color32::from_rgb(30, 30, 35)),
                        );
                        scale_handles_all.push((theme, sp, h_pt, anchor, sx, sy));
                    }
                }

                // Draw Draggable Corner / Control Handles
                let hover_pos = resp.hover_pos();
                for (pt_i, &pt) in shape.points.iter().enumerate() {
                    let sp = to_screen_theme(pt, artboard_rect);
                    let is_node_selected = self.selected_node_idx == Some(pt_i);
                    let is_hovered = hover_pos.map(|hp| hp.distance(sp) <= 12.0).unwrap_or(false);

                    let radius = if is_node_selected {
                        9.0
                    } else if is_hovered || is_node_tool {
                        7.5
                    } else {
                        6.5
                    };

                    let fill_col = if is_node_selected {
                        Color32::WHITE
                    } else if is_hovered {
                        Color32::from_rgb(56, 182, 255)
                    } else {
                        Color32::from_rgb(255, 127, 42)
                    };

                    let stroke_col = if is_node_selected {
                        Color32::from_rgb(56, 182, 255)
                    } else {
                        match theme {
                            ThemeMode::Dark => Color32::WHITE,
                            ThemeMode::Light => Color32::from_rgb(30, 30, 35),
                        }
                    };

                    if is_node_selected {
                        painter.circle_stroke(sp, radius + 3.5, Stroke::new(2.0, Color32::from_rgb(255, 127, 42)));
                    }

                    painter.circle_filled(sp, radius, fill_col);
                    painter.circle_stroke(sp, radius, Stroke::new(2.0, stroke_col));

                    if self.show_corner_indices || is_node_tool {
                        let text_col = if is_node_selected {
                            Color32::from_rgb(56, 220, 255)
                        } else {
                            match theme {
                                ThemeMode::Dark => Color32::YELLOW,
                                ThemeMode::Light => Color32::from_rgb(180, 100, 0),
                            }
                        };
                        painter.text(
                            sp + Vec2::new(9.0, -9.0),
                            egui::Align2::LEFT_BOTTOM,
                            format!("#{}", pt_i),
                            FontId::proportional(11.5),
                            text_col,
                        );
                    }
                }
            }
        }

        // Hover Cursors (Including Rotation Gizmo)
        let mut draw_rot_cursor: Option<(Pos2, Option<f32>)> = None;
        if let Some(DragAction::RotateShape { current_delta_deg, .. }) = &self.drag_action {
            if let Some(p) = resp.interact_pointer_pos() {
                ui.ctx().set_cursor_icon(CursorIcon::None);
                draw_rot_cursor = Some((p, Some(*current_delta_deg)));
            }
        } else if let Some(hover_pos) = resp.hover_pos() {
            if rot_handles_by_theme.iter().any(|(_, rot_sp)| rot_sp.distance(hover_pos) <= 13.0) {
                ui.ctx().set_cursor_icon(CursorIcon::None);
                draw_rot_cursor = Some((hover_pos, None));
            }
            if draw_rot_cursor.is_none() {
                if let Some((hit_theme, hit_rect)) = which_canvas_at(hover_pos) {
                    if let Some(sel_idx) = self.selected_idx.filter(|&i| i < self.shapes.len() && self.shapes[i].visible && !self.shapes[i].locked) {
                        let hit_cpt = to_canvas_theme(hover_pos, hit_rect);
                        if self.shapes[sel_idx]
                            .points
                            .iter()
                            .any(|&pt| to_screen_theme(pt, hit_rect).distance(hover_pos) <= 11.0)
                        {
                            ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
                        } else if scale_handles_all
                            .iter()
                            .any(|(t, sp, ..)| *t == hit_theme && sp.distance(hover_pos) <= 10.0)
                        {
                            ui.ctx().set_cursor_icon(CursorIcon::ResizeNwSe);
                        } else if self.shapes[sel_idx].hit_test(hit_cpt) {
                            ui.ctx().set_cursor_icon(CursorIcon::Grab);
                        }
                    }
                }
            }
        }

        // Double-Click: Add Corner Node or Finish NURBS
        if resp.double_clicked() {
            if let Some(pos) = resp.interact_pointer_pos() {
                if let Some((theme, rect)) = which_canvas_at(pos) {
                    self.active_color_theme = theme;
                    let cpt = to_canvas_theme(pos, rect);

                    if self.active_tool == StudioTool::DrawNurbsCurve && self.nurbs_points.len() >= 2 {
                        self.save_undo();
                        let id = self.next_id;
                        self.next_id += 1;
                        let pts = std::mem::take(&mut self.nurbs_points);
                        let mut s = VectorShape::default_cad_style(
                            id,
                            &format!("NURBS Curve {}", id),
                            ShapeKind::NurbsCurve { closed: false },
                            pts,
                        );
                        s.stroke_color = self.primary_stroke_color;
                        s.stroke_width = 20.0;
                        s.show_vertex_nodes = true;
                        s.vertex_node_radius = 24.0;
                        self.shapes.push(s);
                        self.selected_idx = Some(self.shapes.len() - 1);
                        self.active_tool = StudioTool::NodeCornerEdit;
                        self.dirty_render = true;
                        self.status_msg = "Created NURBS curve! Direct Node Tool active.".to_string();
                        return;
                    }

                    let target_idx = self.find_hit_shape(cpt).or(self.selected_idx);
                    if let Some(sel_idx) = target_idx {
                        if sel_idx < self.shapes.len() && self.shapes[sel_idx].visible {
                            self.save_undo();
                            self.selected_idx = Some(sel_idx);
                            self.active_tool = StudioTool::NodeCornerEdit;
                            let shape = &mut self.shapes[sel_idx];
                            match &mut shape.kind {
                                ShapeKind::Line { .. } => {
                                    let pts = shape.points.clone();
                                    if pts.len() >= 2 {
                                        shape.kind = ShapeKind::Poly { closed: false, corner_radius: 0.0 };
                                        shape.points = vec![pts[0], cpt, pts[1]];
                                        shape.sync_edge_colors_len();
                                        self.selected_node_idx = Some(1);
                                        self.dirty_render = true;
                                        self.status_msg = format!("Inserted new corner node on line at ({:.0}, {:.0})!", cpt[0], cpt[1]);
                                    }
                                }
                                ShapeKind::Quad { corner_radius, .. } => {
                                    let cr = *corner_radius;
                                    shape.kind = ShapeKind::Poly { closed: true, corner_radius: cr };
                                    let n = shape.points.len();
                                    let mut best_edge = 0;
                                    let mut best_d = f32::MAX;
                                    for i in 0..n {
                                        let d = dist_to_segment(cpt, shape.points[i], shape.points[(i + 1) % n]);
                                        if d < best_d {
                                            best_d = d;
                                            best_edge = i;
                                        }
                                    }
                                    shape.points.insert(best_edge + 1, cpt);
                                    shape.sync_edge_colors_len();
                                    self.selected_node_idx = Some(best_edge + 1);
                                    self.dirty_render = true;
                                    self.status_msg = format!("Inserted new corner node at ({:.0}, {:.0})!", cpt[0], cpt[1]);
                                }
                                ShapeKind::ThreePointArc { .. } | ShapeKind::CircleArc { .. } => {
                                    let mut best_i = 0;
                                    let mut best_d = f32::MAX;
                                    for (i, &pt) in shape.points.iter().enumerate() {
                                        let d = dist(cpt, pt);
                                        if d < best_d {
                                            best_d = d;
                                            best_i = i;
                                        }
                                    }
                                    self.selected_node_idx = Some(best_i);
                                    self.dirty_render = true;
                                    self.status_msg = format!("CAD Arc Node #{} selected — drag to reshape arc!", best_i);
                                }
                                ShapeKind::NurbsCurve { closed } => {
                                    let n = shape.points.len();
                                    let edge_count = if *closed { n } else { n.saturating_sub(1) };
                                    let mut best_edge = 0;
                                    let mut best_d = f32::MAX;
                                    for i in 0..edge_count {
                                        let next_i = (i + 1) % n;
                                        let d = dist_to_segment(cpt, shape.points[i], shape.points[next_i]);
                                        if d < best_d {
                                            best_d = d;
                                            best_edge = i;
                                        }
                                    }
                                    shape.points.insert(best_edge + 1, cpt);
                                    shape.sync_edge_colors_len();
                                    self.selected_node_idx = Some(best_edge + 1);
                                    self.dirty_render = true;
                                    self.status_msg = format!("Inserted NURBS point at ({:.0}, {:.0})!", cpt[0], cpt[1]);
                                }
                                ShapeKind::RegularPoly { .. } => {
                                    shape.convert_to_free_corners();
                                    let n = shape.points.len();
                                    let closed = shape.is_closed();
                                    let edge_count = if closed { n } else { n.saturating_sub(1) };
                                    let mut best_edge = 0;
                                    let mut best_d = f32::MAX;
                                    for i in 0..edge_count {
                                        let next_i = (i + 1) % n;
                                        let d = dist_to_segment(cpt, shape.points[i], shape.points[next_i]);
                                        if d < best_d {
                                            best_d = d;
                                            best_edge = i;
                                        }
                                    }
                                    shape.points.insert(best_edge + 1, cpt);
                                    shape.sync_edge_colors_len();
                                    self.selected_node_idx = Some(best_edge + 1);
                                    self.dirty_render = true;
                                    self.status_msg = format!("Converted to editable corners and inserted node at ({:.0}, {:.0})!", cpt[0], cpt[1]);
                                }
                                ShapeKind::Poly { closed, .. } => {
                                    let n = shape.points.len();
                                    let edge_count = if *closed { n } else { n.saturating_sub(1) };
                                    let mut best_edge = 0;
                                    let mut best_d = f32::MAX;
                                    for i in 0..edge_count {
                                        let next_i = (i + 1) % n;
                                        let d = dist_to_segment(cpt, shape.points[i], shape.points[next_i]);
                                        if d < best_d {
                                            best_d = d;
                                            best_edge = i;
                                        }
                                    }
                                    shape.points.insert(best_edge + 1, cpt);
                                    shape.sync_edge_colors_len();
                                    self.selected_node_idx = Some(best_edge + 1);
                                    self.dirty_render = true;
                                    self.status_msg = format!("Inserted new corner node at ({:.0}, {:.0})!", cpt[0], cpt[1]);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        // Single Click: Select, Bucket Fill Region, or Edge Paint
        if resp.clicked() {
            if let Some(pos) = resp.interact_pointer_pos() {
                if let Some((theme, rect)) = which_canvas_at(pos) {
                    self.active_color_theme = theme;
                    let cpt = to_canvas_theme(pos, rect);

                    match self.active_tool {
                        StudioTool::DrawNurbsCurve => {
                            let pt = if self.snap_to_grid {
                                let g = self.grid_step;
                                [(cpt[0] / g).round() * g, (cpt[1] / g).round() * g]
                            } else {
                                cpt
                            };
                            self.nurbs_points.push(pt);
                            self.status_msg = format!(
                                "NURBS Point #{} placed at ({:.0}, {:.0}). Click next point, or Double-Click / Enter to finish.",
                                self.nurbs_points.len(),
                                pt[0],
                                pt[1]
                            );
                        }
                        StudioTool::ShapeBuilderFill => {
                            if let Some((label, contours)) = self.compute_region_at_point(cpt) {
                                self.save_undo();
                                let id = self.next_id;
                                self.next_id += 1;
                                let mut s = VectorShape::from_contours(id, &label, contours);
                                s.fill_enabled = true;
                                match theme {
                                    ThemeMode::Dark => {
                                        s.fill_color = self.primary_fill_color;
                                        s.stroke_color = self.primary_stroke_color;
                                    }
                                    ThemeMode::Light => {
                                        s.light_fill_color = Some(self.primary_fill_color);
                                        s.light_stroke_color = Some(self.primary_stroke_color);
                                    }
                                }
                                s.stroke_enabled = true;
                                self.shapes.push(s);
                                self.selected_idx = Some(self.shapes.len() - 1);
                                self.status_msg = format!("Filled closed region '{}'!", label);
                            } else if let Some(sel) = self.selected_idx.filter(|&i| i < self.shapes.len() && self.shapes[i].visible) {
                                if self.shapes[sel].hit_test(cpt) {
                                    self.fill_selected_closed_region();
                                }
                            }
                        }
                        StudioTool::CurveEdgePainter => {
                            let mut painted = false;
                            for s_i in (0..self.shapes.len()).rev() {
                                if !self.shapes[s_i].visible {
                                    continue;
                                }
                                let segs = self.shapes[s_i].edge_segments();
                                if let Some((e_i, _)) = segs
                                    .iter()
                                    .enumerate()
                                    .find(|(_, (a, b))| dist_to_segment(cpt, *a, *b) < 18.0)
                                {
                                    self.save_undo();
                                    self.selected_idx = Some(s_i);
                                    self.shapes[s_i].per_edge_colors_enabled = true;
                                    self.shapes[s_i].sync_edge_colors_len();
                                    match theme {
                                        ThemeMode::Dark => self.shapes[s_i].edge_colors[e_i] = self.primary_fill_color,
                                        ThemeMode::Light => {
                                            let mut ec = self.shapes[s_i].effective_edge_colors(ThemeMode::Light);
                                            ec[e_i] = self.primary_fill_color;
                                            self.shapes[s_i].light_edge_colors = Some(ec);
                                        }
                                    }
                                    painted = true;
                                    break;
                                } else if self.shapes[s_i].hit_test(cpt) {
                                    self.save_undo();
                                    self.selected_idx = Some(s_i);
                                    match theme {
                                        ThemeMode::Dark => self.shapes[s_i].stroke_color = self.primary_fill_color,
                                        ThemeMode::Light => self.shapes[s_i].light_stroke_color = Some(self.primary_fill_color),
                                    }
                                    painted = true;
                                    break;
                                }
                            }
                            if !painted {
                                self.selected_idx = None;
                            }
                        }
                        _ => {
                            let shift = ui.input(|i| i.modifiers.shift);
                            let mut clicked_handle = None;
                            if let Some(sel_idx) = self.selected_idx.filter(|&i| i < self.shapes.len() && self.shapes[i].visible && !self.shapes[i].locked) {
                                for (pt_i, &pt) in self.shapes[sel_idx].points.iter().enumerate() {
                                    if to_screen_theme(pt, rect).distance(pos) <= 12.0 {
                                        clicked_handle = Some(pt_i);
                                        break;
                                    }
                                }
                            }

                            if let Some(node_i) = clicked_handle {
                                self.selected_node_idx = Some(node_i);
                                self.status_msg = format!("Selected Corner Node #{} — Drag to move, Del to delete", node_i);
                            } else if let Some(hit_i) = self.find_hit_shape(cpt) {
                                if shift {
                                    self.boolean_partner_idx = Some(hit_i);
                                    self.status_msg = format!("Set #{} as Boolean Partner Shape B!", self.shapes[hit_i].id);
                                } else {
                                    self.selected_idx = Some(hit_i);
                                    self.selected_node_idx = None;
                                    self.status_msg = format!("Selected shape #{} ({})", self.shapes[hit_i].id, self.shapes[hit_i].name);
                                }
                            } else if !shift {
                                self.selected_idx = None;
                                self.selected_node_idx = None;
                                self.boolean_partner_idx = None;
                            }
                        }
                    }
                }
            }
        }

        // Drag Start
        if resp.drag_started() {
            if let Some(pos) = resp.interact_pointer_pos() {
                if let Some((theme, rect)) = which_canvas_at(pos) {
                    self.active_color_theme = theme;
                    let cpt = to_canvas_theme(pos, rect);

                    if let Some(sel_idx) = self.selected_idx.filter(|&i| i < self.shapes.len() && self.shapes[i].visible && !self.shapes[i].locked) {
                        // 1. Rotation Handle
                        if rot_handles_by_theme.iter().any(|(t, sp)| *t == theme && sp.distance(pos) <= 13.0) {
                            self.save_undo();
                            let piv = self.shapes[sel_idx].center();
                            let ang = (cpt[1] - piv[1]).atan2(cpt[0] - piv[0]).to_degrees();
                            self.drag_action = Some(DragAction::RotateShape {
                                shape_idx: sel_idx,
                                pivot: piv,
                                start_angle_deg: ang,
                                current_delta_deg: 0.0,
                                orig_shape: self.shapes[sel_idx].clone(),
                                origin_theme: theme,
                            });
                            return;
                        }

                        // 2. Corner Handles
                        for (pt_i, &pt) in self.shapes[sel_idx].points.iter().enumerate() {
                            if to_screen_theme(pt, rect).distance(pos) <= 12.0 {
                                self.save_undo();
                                self.selected_node_idx = Some(pt_i);
                                self.drag_action = Some(DragAction::CornerPoint {
                                    shape_idx: sel_idx,
                                    pt_idx: pt_i,
                                    origin_theme: theme,
                                });
                                return;
                            }
                        }

                        // 3. Scale Handles
                        for (t, sp, h_pt, anchor, sx, sy) in &scale_handles_all {
                            if *t == theme && sp.distance(pos) <= 10.0 {
                                self.save_undo();
                                self.drag_action = Some(DragAction::ScaleBoxHandle {
                                    shape_idx: sel_idx,
                                    anchor: *anchor,
                                    start_pos: *h_pt,
                                    scale_x: *sx,
                                    scale_y: *sy,
                                    orig_shape: self.shapes[sel_idx].clone(),
                                    origin_theme: theme,
                                });
                                return;
                            }
                        }

                        // 4. Move Selected Shape
                        if self.shapes[sel_idx].hit_test(cpt) {
                            self.save_undo();
                            self.drag_action = Some(DragAction::MoveShape {
                                shape_idx: sel_idx,
                                start_canvas_pos: cpt,
                                orig_shape: self.shapes[sel_idx].clone(),
                                origin_theme: theme,
                            });
                            return;
                        }
                    }

                    // 5. Hit Test Other Shapes
                    if let Some(i) = self.find_hit_shape(cpt) {
                        self.selected_idx = Some(i);
                        self.save_undo();
                        self.drag_action = Some(DragAction::MoveShape {
                            shape_idx: i,
                            start_canvas_pos: cpt,
                            orig_shape: self.shapes[i].clone(),
                            origin_theme: theme,
                        });
                    } else {
                        self.selected_idx = None;
                    }
                }
            }
        }

        // Active Dragging
        if resp.dragged() {
            if let (Some(pos), Some(action)) = (resp.interact_pointer_pos(), self.drag_action.clone()) {
                let origin_theme = action.origin_theme();
                let art_rect = get_rect_for_theme(origin_theme);
                let raw_c = to_canvas_theme(pos, art_rect);
                let snapped_c = self.snap_pt(raw_c);

                match action {
                    DragAction::CornerPoint { shape_idx, pt_idx, .. } => {
                        ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
                        let shape = &mut self.shapes[shape_idx];
                        match &mut shape.kind {
                            ShapeKind::Quad { lock_rect: true, .. } if shape.points.len() == 4 => {
                                shape.points[pt_idx] = snapped_c;
                                match pt_idx {
                                    0 => {
                                        shape.points[1][1] = snapped_c[1];
                                        shape.points[3][0] = snapped_c[0];
                                    }
                                    1 => {
                                        shape.points[0][1] = snapped_c[1];
                                        shape.points[2][0] = snapped_c[0];
                                    }
                                    2 => {
                                        shape.points[3][1] = snapped_c[1];
                                        shape.points[1][0] = snapped_c[0];
                                    }
                                    3 => {
                                        shape.points[2][1] = snapped_c[1];
                                        shape.points[0][0] = snapped_c[0];
                                    }
                                    _ => {}
                                }
                            }
                            ShapeKind::CircleArc {
                                lock_circle,
                                start_deg,
                                sweep_deg,
                                ..
                            } if shape.points.len() >= 3 => {
                                if pt_idx == 0 {
                                    let dx = snapped_c[0] - shape.points[0][0];
                                    let dy = snapped_c[1] - shape.points[0][1];
                                    shape.translate(dx, dy);
                                } else {
                                    shape.points[pt_idx] = snapped_c;
                                    let c = shape.points[0];
                                    let r = dist(c, snapped_c).max(4.0);
                                    let ang = (snapped_c[1] - c[1])
                                        .atan2(snapped_c[0] - c[0])
                                        .to_degrees()
                                        .rem_euclid(360.0);
                                    if pt_idx == 1 && sweep_deg.abs() < 359.5 {
                                        *start_deg = ang;
                                    } else if pt_idx == 2 && sweep_deg.abs() < 359.5 {
                                        *sweep_deg = (ang - *start_deg).rem_euclid(360.0);
                                    }
                                    if *lock_circle {
                                        let a0 = start_deg.to_radians();
                                        let a1 = if sweep_deg.abs() >= 359.5 {
                                            (*start_deg + 90.0).to_radians()
                                        } else {
                                            (*start_deg + *sweep_deg).to_radians()
                                        };
                                        shape.points[1] = [c[0] + r * a0.cos(), c[1] + r * a0.sin()];
                                        shape.points[2] = [c[0] + r * a1.cos(), c[1] + r * a1.sin()];
                                    }
                                }
                            }
                            _ => {
                                if pt_idx < shape.points.len() {
                                    shape.points[pt_idx] = snapped_c;
                                }
                            }
                        }
                        self.dirty_render = true;
                    }
                    DragAction::MoveShape {
                        shape_idx,
                        start_canvas_pos,
                        orig_shape,
                        ..
                    } => {
                        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
                        let mut dx = raw_c[0] - start_canvas_pos[0];
                        let mut dy = raw_c[1] - start_canvas_pos[1];
                        if self.snap_to_grid && self.grid_step > 0.5 {
                            dx = (dx / self.grid_step).round() * self.grid_step;
                            dy = (dy / self.grid_step).round() * self.grid_step;
                        }
                        let mut moved = orig_shape;
                        moved.translate(dx, dy);
                        self.shapes[shape_idx] = moved;
                        self.dirty_render = true;
                    }
                    DragAction::RotateShape {
                        shape_idx,
                        pivot,
                        start_angle_deg,
                        orig_shape,
                        origin_theme,
                        ..
                    } => {
                        ui.ctx().set_cursor_icon(CursorIcon::None);
                        let cur_ang = (raw_c[1] - pivot[1]).atan2(raw_c[0] - pivot[0]).to_degrees();
                        let mut delta = cur_ang - start_angle_deg;
                        if delta > 180.0 {
                            delta -= 360.0;
                        } else if delta < -180.0 {
                            delta += 360.0;
                        }
                        if ui.input(|i| i.modifiers.shift) {
                            delta = (delta / 15.0).round() * 15.0;
                        }
                        let mut rotated = orig_shape.clone();
                        rotated.rotate(delta, pivot);
                        self.shapes[shape_idx] = rotated;
                        self.drag_action = Some(DragAction::RotateShape {
                            shape_idx,
                            pivot,
                            start_angle_deg,
                            current_delta_deg: delta,
                            orig_shape,
                            origin_theme,
                        });
                        self.dirty_render = true;
                    }
                    DragAction::ScaleBoxHandle {
                        shape_idx,
                        anchor,
                        start_pos,
                        scale_x,
                        scale_y,
                        orig_shape,
                        ..
                    } => {
                        ui.ctx().set_cursor_icon(CursorIcon::ResizeNwSe);
                        let sx = if scale_x && (start_pos[0] - anchor[0]).abs() > 2.0 {
                            ((snapped_c[0] - anchor[0]) / (start_pos[0] - anchor[0])).clamp(0.08, 10.0)
                        } else {
                            1.0
                        };
                        let sy = if scale_y && (start_pos[1] - anchor[1]).abs() > 2.0 {
                            ((snapped_c[1] - anchor[1]) / (start_pos[1] - anchor[1])).clamp(0.08, 10.0)
                        } else {
                            1.0
                        };
                        let mut scaled = orig_shape;
                        scaled.scale(sx, sy, anchor, false);
                        self.shapes[shape_idx] = scaled;
                        self.dirty_render = true;
                    }
                }
            }
        }

        if resp.drag_stopped() {
            self.drag_action = None;
        }

        // Draw Custom Curved Rotation Mouse Cursor
        if let Some((cursor_pos, opt_deg)) = draw_rot_cursor {
            draw_rotation_mouse_cursor(&painter, cursor_pos, opt_deg);
        }
    }
}
