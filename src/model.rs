use crate::boolean::{compute_boolean, BooleanOp};
use serde::{Deserialize, Serialize};
use std::f32::consts::{PI, TAU};

pub const CANVAS_SIZE: f32 = 512.0;
pub const CANVAS_HALF: f32 = 256.0;
pub const PREVIEW_SIZES: [u32; 4] = [64, 48, 32, 24];

#[derive(Clone, Debug)]
pub enum PathCmd {
    MoveTo(f32, f32),
    LineTo(f32, f32),
    QuadTo(f32, f32, f32, f32),
    CubicTo(f32, f32, f32, f32, f32, f32),
    Close,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapStyle {
    Round,
    Butt,
    Square,
}

impl CapStyle {
    pub fn to_skia(self) -> tiny_skia::LineCap {
        match self {
            CapStyle::Round => tiny_skia::LineCap::Round,
            CapStyle::Butt => tiny_skia::LineCap::Butt,
            CapStyle::Square => tiny_skia::LineCap::Square,
        }
    }
    pub fn to_svg(self) -> &'static str {
        match self {
            CapStyle::Round => "round",
            CapStyle::Butt => "butt",
            CapStyle::Square => "square",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum JoinStyle {
    Round,
    Miter,
    Bevel,
}

impl JoinStyle {
    pub fn to_skia(self) -> tiny_skia::LineJoin {
        match self {
            JoinStyle::Round => tiny_skia::LineJoin::Round,
            JoinStyle::Miter => tiny_skia::LineJoin::Miter,
            JoinStyle::Bevel => tiny_skia::LineJoin::Bevel,
        }
    }
    pub fn to_svg(self) -> &'static str {
        match self {
            JoinStyle::Round => "round",
            JoinStyle::Miter => "miter",
            JoinStyle::Bevel => "bevel",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FillRuleMode {
    Winding,
    EvenOdd,
}

impl FillRuleMode {
    pub fn to_skia(self) -> tiny_skia::FillRule {
        match self {
            FillRuleMode::Winding => tiny_skia::FillRule::Winding,
            FillRuleMode::EvenOdd => tiny_skia::FillRule::EvenOdd,
        }
    }
    pub fn to_svg(self) -> &'static str {
        match self {
            FillRuleMode::Winding => "nonzero",
            FillRuleMode::EvenOdd => "evenodd",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeMode {
    Dark,
    Light,
}

impl ThemeMode {
    pub fn label(self) -> &'static str {
        match self {
            ThemeMode::Dark => "Dark Theme",
            ThemeMode::Light => "Light Theme",
        }
    }
    pub fn suffix(self) -> &'static str {
        match self {
            ThemeMode::Dark => "dark",
            ThemeMode::Light => "light",
        }
    }
}

pub fn invert_color_for_light_theme(c: [u8; 4]) -> [u8; 4] {
    let [r, g, b, a] = c;
    if a == 0 {
        return c;
    }
    let lum = 0.299 * (r as f32) + 0.587 * (g as f32) + 0.114 * (b as f32);
    let max_c = r.max(g).max(b) as f32;
    let min_c = r.min(g).min(b) as f32;
    let sat = if max_c > 0.0 { (max_c - min_c) / max_c } else { 0.0 };

    if sat > 0.32 {
        // High-saturation accent color (e.g. CAD orange #FF7F2A, cyan, etc.)
        if lum > 175.0 {
            let f = 0.78;
            [((r as f32) * f) as u8, ((g as f32) * f) as u8, ((b as f32) * f) as u8, a]
        } else {
            c
        }
    } else {
        // Grayscale / low-saturation tone
        if lum >= 130.0 {
            // Light in dark theme -> dark charcoal in light theme (e.g. white #EEEEF0 -> #222226)
            let val = (255.0 - lum).max(24.0) as u8;
            [val, val, ((val as f32) * 1.08).min(255.0) as u8, a]
        } else {
            // Dark in dark theme (e.g. dark outer ring 30,30,32) -> keep
            c
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ShapeKind {
    Quad {
        lock_rect: bool,
        corner_radius: f32,
    },
    Poly {
        closed: bool,
        corner_radius: f32,
    },
    CompoundPoly {
        contours: Vec<Vec<[f32; 2]>>,
        corner_radius: f32,
    },
    Line {
        arrow_start: bool,
        arrow_end: bool,
        arrow_size: f32,
    },
    CircleArc {
        lock_circle: bool,
        start_deg: f32,
        sweep_deg: f32,
        close_pie: bool,
    },
    CubicBezier {
        closed: bool,
    },
    RegularPoly {
        sides: usize,
        inner_ratio: f32,
        corner_radius: f32,
    },
    ThreePointArc {
        close_chord: bool,
    },
    NurbsCurve {
        closed: bool,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VectorShape {
    pub id: u64,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub kind: ShapeKind,
    pub points: Vec<[f32; 2]>,

    pub fill_enabled: bool,
    pub fill_color: [u8; 4],
    #[serde(default)]
    pub light_fill_color: Option<[u8; 4]>,
    pub fill_rule: FillRuleMode,
    pub close_path_for_fill: bool,

    pub stroke_enabled: bool,
    pub stroke_color: [u8; 4],
    #[serde(default)]
    pub light_stroke_color: Option<[u8; 4]>,
    pub stroke_width: f32,
    pub line_cap: CapStyle,
    pub line_join: JoinStyle,
    pub dashed: bool,
    pub dash_len: f32,
    pub gap_len: f32,

    pub per_edge_colors_enabled: bool,
    pub edge_colors: Vec<[u8; 4]>,
    #[serde(default)]
    pub light_edge_colors: Option<Vec<[u8; 4]>>,

    pub show_vertex_nodes: bool,
    pub vertex_node_radius: f32,
    pub vertex_node_fill: [u8; 4],
    #[serde(default)]
    pub light_vertex_node_fill: Option<[u8; 4]>,
    pub vertex_node_stroke: [u8; 4],
    #[serde(default)]
    pub light_vertex_node_stroke: Option<[u8; 4]>,
    pub vertex_node_stroke_width: f32,
}

impl VectorShape {
    pub fn default_cad_style(
        id: u64,
        name: &str,
        kind: ShapeKind,
        points: Vec<[f32; 2]>,
    ) -> Self {
        let edge_count = points.len().max(4);
        let mut s = Self {
            id,
            name: name.to_string(),
            visible: true,
            locked: false,
            kind,
            points,
            fill_enabled: false,
            fill_color: [255, 127, 42, 175],
            light_fill_color: None,
            fill_rule: FillRuleMode::EvenOdd,
            close_path_for_fill: true,
            stroke_enabled: true,
            stroke_color: [238, 238, 240, 255],
            light_stroke_color: None,
            stroke_width: 20.0,
            line_cap: CapStyle::Round,
            line_join: JoinStyle::Round,
            dashed: false,
            dash_len: 26.0,
            gap_len: 20.0,
            per_edge_colors_enabled: false,
            edge_colors: vec![[255, 127, 42, 255]; edge_count],
            light_edge_colors: None,
            show_vertex_nodes: false,
            vertex_node_radius: 24.0,
            vertex_node_fill: [255, 127, 42, 255],
            light_vertex_node_fill: None,
            vertex_node_stroke: [30, 30, 32, 255],
            light_vertex_node_stroke: None,
            vertex_node_stroke_width: 10.0,
        };
        s.sync_arc_handles_to_angles();
        s
    }

    pub fn effective_stroke_color(&self, theme: ThemeMode) -> [u8; 4] {
        match theme {
            ThemeMode::Dark => self.stroke_color,
            ThemeMode::Light => self
                .light_stroke_color
                .unwrap_or_else(|| invert_color_for_light_theme(self.stroke_color)),
        }
    }

    pub fn effective_fill_color(&self, theme: ThemeMode) -> [u8; 4] {
        match theme {
            ThemeMode::Dark => self.fill_color,
            ThemeMode::Light => self
                .light_fill_color
                .unwrap_or_else(|| invert_color_for_light_theme(self.fill_color)),
        }
    }

    pub fn effective_edge_colors(&self, theme: ThemeMode) -> Vec<[u8; 4]> {
        match theme {
            ThemeMode::Dark => self.edge_colors.clone(),
            ThemeMode::Light => {
                if let Some(ref l_edges) = self.light_edge_colors {
                    l_edges.clone()
                } else {
                    self.edge_colors
                        .iter()
                        .map(|&c| invert_color_for_light_theme(c))
                        .collect()
                }
            }
        }
    }

    pub fn effective_vertex_node_fill(&self, theme: ThemeMode) -> [u8; 4] {
        match theme {
            ThemeMode::Dark => self.vertex_node_fill,
            ThemeMode::Light => self
                .light_vertex_node_fill
                .unwrap_or_else(|| invert_color_for_light_theme(self.vertex_node_fill)),
        }
    }

    pub fn effective_vertex_node_stroke(&self, theme: ThemeMode) -> [u8; 4] {
        match theme {
            ThemeMode::Dark => self.vertex_node_stroke,
            ThemeMode::Light => self
                .light_vertex_node_stroke
                .unwrap_or_else(|| invert_color_for_light_theme(self.vertex_node_stroke)),
        }
    }

    pub fn has_light_overrides(&self) -> bool {
        self.light_stroke_color.is_some()
            || self.light_fill_color.is_some()
            || self.light_edge_colors.is_some()
            || self.light_vertex_node_fill.is_some()
            || self.light_vertex_node_stroke.is_some()
    }

    pub fn clear_light_overrides(&mut self) {
        self.light_stroke_color = None;
        self.light_fill_color = None;
        self.light_edge_colors = None;
        self.light_vertex_node_fill = None;
        self.light_vertex_node_stroke = None;
    }

    /// Keeps `points[1]` and `points[2]` of a `CircleArc` sitting directly on the start and end of the arc!
    pub fn sync_arc_handles_to_angles(&mut self) {
        if let ShapeKind::CircleArc {
            start_deg,
            sweep_deg,
            ..
        } = self.kind
        {
            if self.points.len() >= 3 {
                let c = self.points[0];
                let rx = dist(c, self.points[1]).max(4.0);
                let ry = dist(c, self.points[2]).max(4.0);
                let a0 = start_deg.to_radians();
                let a1 = if sweep_deg.abs() >= 359.5 {
                    (start_deg + 90.0).to_radians()
                } else {
                    (start_deg + sweep_deg).to_radians()
                };
                self.points[1] = [c[0] + rx * a0.cos(), c[1] + rx * a0.sin()];
                self.points[2] = [c[0] + ry * a1.cos(), c[1] + ry * a1.sin()];
            }
        }
    }

    /// Enables fill and ensures open paths/curves/arcs close their region cleanly
    pub fn enable_closed_region_fill(&mut self, color: [u8; 4]) {
        self.fill_enabled = true;
        self.fill_color = color;
        self.close_path_for_fill = true;
        match &mut self.kind {
            ShapeKind::Poly { closed, .. } | ShapeKind::CubicBezier { closed } => {
                *closed = true;
            }
            _ => {}
        }
    }

    pub fn from_contours(id: u64, name: &str, mut contours: Vec<Vec<[f32; 2]>>) -> Self {
        if contours.len() == 1 {
            let pts = contours.remove(0);
            let mut s = Self::default_cad_style(
                id,
                name,
                ShapeKind::Poly {
                    closed: true,
                    corner_radius: 0.0,
                },
                pts,
            );
            s.fill_enabled = true;
            s
        } else {
            let first_pts = contours.first().cloned().unwrap_or_default();
            let mut s = Self::default_cad_style(
                id,
                name,
                ShapeKind::CompoundPoly {
                    contours,
                    corner_radius: 0.0,
                },
                first_pts,
            );
            s.fill_enabled = true;
            s.fill_rule = FillRuleMode::EvenOdd;
            s
        }
    }

    pub fn sync_edge_colors_len(&mut self) {
        let needed = self.edge_segments().len().max(1);
        while self.edge_colors.len() < needed {
            self.edge_colors.push(self.stroke_color);
        }
    }

    pub fn is_closed(&self) -> bool {
        match &self.kind {
            ShapeKind::Quad { .. }
            | ShapeKind::RegularPoly { .. }
            | ShapeKind::CompoundPoly { .. } => true,
            ShapeKind::Poly { closed, .. } => *closed,
            ShapeKind::CubicBezier { closed } => *closed,
            ShapeKind::CircleArc {
                sweep_deg,
                close_pie,
                ..
            } => sweep_deg.abs() >= 359.5 || *close_pie,
            ShapeKind::ThreePointArc { close_chord } => *close_chord,
            ShapeKind::NurbsCurve { closed } => *closed,
            ShapeKind::Line { .. } => false,
        }
    }

    pub fn stroke_segments(&self) -> Vec<([f32; 2], [f32; 2])> {
        let contours = self.to_contours();
        let closed = self.is_closed();
        let mut segs = Vec::new();
        for c in &contours {
            if c.len() < 2 {
                continue;
            }
            for i in 0..(c.len() - 1) {
                segs.push((c[i], c[i + 1]));
            }
            if closed && c.len() >= 3 {
                segs.push((c[c.len() - 1], c[0]));
            }
        }
        segs
    }

    pub fn distance_to_curve(&self, pt: [f32; 2]) -> f32 {
        let segs = self.stroke_segments();
        let mut min_d = f32::MAX;
        for (a, b) in segs {
            let d = dist_to_segment(pt, a, b);
            if d < min_d {
                min_d = d;
            }
        }
        min_d
    }

    /// Reliable curve-first hit test: strictly tests proximity to the stroke/curve or filled interior.
    /// Never checks empty rectangular bounding boxes or phantom chord segments of open arcs!
    pub fn hit_test(&self, pt: [f32; 2]) -> bool {
        if !self.visible {
            return false;
        }

        // 1. Pick tolerance along the actual curve (half stroke width + 10px, min 14px)
        let stroke_tol = (self.stroke_width * 0.5 + 10.0).max(14.0);
        if self.distance_to_curve(pt) <= stroke_tol {
            return true;
        }

        // 2. Interior test ONLY if the shape is filled AND closed!
        if self.fill_enabled && self.is_closed() {
            let contours = self.to_contours();
            if point_in_contours(pt, &contours) {
                return true;
            }
        }

        false
    }

    pub fn to_contours(&self) -> Vec<Vec<[f32; 2]>> {
        match &self.kind {
            ShapeKind::CompoundPoly { contours, .. } => contours.clone(),
            ShapeKind::Quad { corner_radius, .. } | ShapeKind::Poly { corner_radius, .. } => {
                if *corner_radius > 0.5 {
                    vec![sample_filleted_poly(&self.points, true, *corner_radius)]
                } else {
                    vec![self.points.clone()]
                }
            }
            ShapeKind::RegularPoly {
                sides,
                inner_ratio,
                corner_radius,
            } => {
                if self.points.len() >= 2 {
                    let pts = generate_regular_poly_pts(
                        self.points[0],
                        self.points[1],
                        *sides,
                        *inner_ratio,
                    );
                    if *corner_radius > 0.5 {
                        vec![sample_filleted_poly(&pts, true, *corner_radius)]
                    } else {
                        vec![pts]
                    }
                } else {
                    Vec::new()
                }
            }
            ShapeKind::CircleArc {
                start_deg,
                sweep_deg,
                close_pie,
                ..
            } => {
                if self.points.len() >= 3 {
                    let c = self.points[0];
                    let rx = dist(c, self.points[1]).max(1.0);
                    let ry = dist(c, self.points[2]).max(1.0);
                    let full = sweep_deg.abs() >= 359.5;
                    let steps = 64;
                    let mut pts = Vec::with_capacity(steps + 2);
                    for i in 0..steps {
                        let t = i as f32 / (if full { steps } else { steps - 1 }) as f32;
                        let a = (start_deg + sweep_deg * t).to_radians();
                        pts.push([c[0] + rx * a.cos(), c[1] + ry * a.sin()]);
                    }
                    if !full && *close_pie {
                        pts.push(c);
                    }
                    vec![pts]
                } else {
                    Vec::new()
                }
            }
            ShapeKind::CubicBezier { .. } => {
                if self.points.len() >= 4 {
                    let (p0, c1, c2, p3) =
                        (self.points[0], self.points[1], self.points[2], self.points[3]);
                    let mut pts = Vec::with_capacity(40);
                    for i in 0..=36 {
                        let t = i as f32 / 36.0;
                        let mt = 1.0 - t;
                        pts.push([
                            mt.powi(3) * p0[0]
                                + 3.0 * mt * mt * t * c1[0]
                                + 3.0 * mt * t * t * c2[0]
                                + t.powi(3) * p3[0],
                            mt.powi(3) * p0[1]
                                + 3.0 * mt * mt * t * c1[1]
                                + 3.0 * mt * t * t * c2[1]
                                + t.powi(3) * p3[1],
                        ]);
                    }
                    vec![pts]
                } else {
                    Vec::new()
                }
            }
            ShapeKind::ThreePointArc { close_chord } => {
                if self.points.len() >= 3 {
                    let (p0, p1, p2) = (self.points[0], self.points[1], self.points[2]);
                    if let Some((c, r, start_deg, sweep_deg)) = circle_from_3_points(p0, p1, p2) {
                        let steps = 48;
                        let mut pts = Vec::with_capacity(steps + 1);
                        for i in 0..steps {
                            let t = i as f32 / (steps - 1) as f32;
                            let a = (start_deg + sweep_deg * t).to_radians();
                            pts.push([c[0] + r * a.cos(), c[1] + r * a.sin()]);
                        }
                        if *close_chord {
                            pts.push(p0);
                        }
                        vec![pts]
                    } else {
                        vec![vec![p0, p1, p2]]
                    }
                } else {
                    Vec::new()
                }
            }
            ShapeKind::NurbsCurve { closed } => {
                let cmds = catmull_rom_to_cubic_cmds(&self.points, *closed, false);
                vec![sample_path_cmds_to_points(&cmds)]
            }
            ShapeKind::Line { .. } => vec![self.points.clone()],
        }
    }

    pub fn bounding_box(&self) -> ([f32; 2], [f32; 2]) {
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        let mut any = false;
        for c in self.to_contours() {
            for p in c {
                any = true;
                min_x = min_x.min(p[0]);
                min_y = min_y.min(p[1]);
                max_x = max_x.max(p[0]);
                max_y = max_y.max(p[1]);
            }
        }
        if !any {
            return ([128.0, 128.0], [384.0, 384.0]);
        }
        ([min_x, min_y], [max_x.max(min_x + 2.0), max_y.max(min_y + 2.0)])
    }

    pub fn center(&self) -> [f32; 2] {
        let (min, max) = self.bounding_box();
        [(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5]
    }

    pub fn edge_segments(&self) -> Vec<([f32; 2], [f32; 2])> {
        let pts = match &self.kind {
            ShapeKind::Quad { .. }
            | ShapeKind::Poly { .. }
            | ShapeKind::Line { .. }
            | ShapeKind::ThreePointArc { .. }
            | ShapeKind::NurbsCurve { .. } => {
                self.points.clone()
            }
            ShapeKind::RegularPoly {
                sides, inner_ratio, ..
            } if self.points.len() >= 2 => {
                generate_regular_poly_pts(self.points[0], self.points[1], *sides, *inner_ratio)
            }
            _ => Vec::new(),
        };
        let closed = !matches!(
            self.kind,
            ShapeKind::Line { .. } | ShapeKind::Poly { closed: false, .. }
        );
        let mut segs = Vec::new();
        if pts.len() >= 2 {
            for i in 0..(pts.len() - 1) {
                segs.push((pts[i], pts[i + 1]));
            }
            if closed && pts.len() >= 3 {
                segs.push((pts[pts.len() - 1], pts[0]));
            }
        }
        segs
    }

    pub fn cad_node_positions(&self) -> Vec<[f32; 2]> {
        match &self.kind {
            ShapeKind::Quad { .. }
            | ShapeKind::Poly { .. }
            | ShapeKind::Line { .. }
            | ShapeKind::ThreePointArc { .. }
            | ShapeKind::NurbsCurve { .. } => {
                self.points.clone()
            }
            ShapeKind::CompoundPoly { contours, .. } => {
                contours.iter().flatten().copied().collect()
            }
            ShapeKind::CubicBezier { .. } => {
                if self.points.len() >= 4 {
                    vec![self.points[0], self.points[3]]
                } else {
                    self.points.clone()
                }
            }
            ShapeKind::CircleArc {
                start_deg,
                sweep_deg,
                ..
            } if self.points.len() >= 3 => {
                let c = self.points[0];
                let rx = dist(c, self.points[1]);
                let ry = dist(c, self.points[2]);
                if sweep_deg.abs() >= 359.5 {
                    vec![
                        [c[0] + rx, c[1]],
                        [c[0] - rx, c[1]],
                        [c[0], c[1] - ry],
                        [c[0], c[1] + ry],
                    ]
                } else {
                    let a0 = start_deg.to_radians();
                    let am = (start_deg + sweep_deg * 0.5).to_radians();
                    let a1 = (start_deg + sweep_deg).to_radians();
                    vec![
                        [c[0] + rx * a0.cos(), c[1] + ry * a0.sin()],
                        [c[0] + rx * am.cos(), c[1] + ry * am.sin()],
                        [c[0] + rx * a1.cos(), c[1] + ry * a1.sin()],
                    ]
                }
            }
            ShapeKind::RegularPoly {
                sides, inner_ratio, ..
            } if self.points.len() >= 2 => {
                generate_regular_poly_pts(self.points[0], self.points[1], *sides, *inner_ratio)
            }
            _ => self.points.clone(),
        }
    }

    pub fn convert_to_free_corners(&mut self) {
        let closed = self.is_closed();
        let mut contours = self.to_contours();
        if contours.len() == 1 {
            self.points = contours.remove(0);
            self.kind = ShapeKind::Poly {
                closed,
                corner_radius: 0.0,
            };
            self.sync_edge_colors_len();
        }
    }

    pub fn translate(&mut self, dx: f32, dy: f32) {
        for p in &mut self.points {
            p[0] += dx;
            p[1] += dy;
        }
        if let ShapeKind::CompoundPoly { contours, .. } = &mut self.kind {
            for c in contours {
                for p in c {
                    p[0] += dx;
                    p[1] += dy;
                }
            }
        }
    }

    pub fn rotate(&mut self, angle_deg: f32, pivot: [f32; 2]) {
        let (sin_a, cos_a) = angle_deg.to_radians().sin_cos();
        if let ShapeKind::Quad { lock_rect, .. } = &mut self.kind {
            if angle_deg.abs() % 90.0 > 0.01 {
                *lock_rect = false;
            }
        }
        if let ShapeKind::CircleArc { start_deg, .. } = &mut self.kind {
            *start_deg = (*start_deg + angle_deg).rem_euclid(360.0);
        }
        for p in &mut self.points {
            let (dx, dy) = (p[0] - pivot[0], p[1] - pivot[1]);
            *p = [
                pivot[0] + dx * cos_a - dy * sin_a,
                pivot[1] + dx * sin_a + dy * cos_a,
            ];
        }
        if let ShapeKind::CompoundPoly { contours, .. } = &mut self.kind {
            for c in contours {
                for p in c {
                    let (dx, dy) = (p[0] - pivot[0], p[1] - pivot[1]);
                    *p = [
                        pivot[0] + dx * cos_a - dy * sin_a,
                        pivot[1] + dx * sin_a + dy * cos_a,
                    ];
                }
            }
        }
    }

    pub fn mirror(&mut self, horizontal: bool, pivot: [f32; 2]) {
        if let ShapeKind::CircleArc {
            start_deg,
            sweep_deg,
            ..
        } = &mut self.kind
        {
            *start_deg = if horizontal {
                180.0 - *start_deg
            } else {
                -*start_deg
            }
            .rem_euclid(360.0);
            *sweep_deg = -*sweep_deg;
        }
        for p in &mut self.points {
            if horizontal {
                p[0] = 2.0 * pivot[0] - p[0];
            } else {
                p[1] = 2.0 * pivot[1] - p[1];
            }
        }
        if let ShapeKind::CompoundPoly { contours, .. } = &mut self.kind {
            for c in contours {
                for p in c {
                    if horizontal {
                        p[0] = 2.0 * pivot[0] - p[0];
                    } else {
                        p[1] = 2.0 * pivot[1] - p[1];
                    }
                }
            }
        }
    }

    pub fn scale(&mut self, sx: f32, sy: f32, pivot: [f32; 2], scale_stroke: bool) {
        for p in &mut self.points {
            p[0] = pivot[0] + (p[0] - pivot[0]) * sx;
            p[1] = pivot[1] + (p[1] - pivot[1]) * sy;
        }
        if let ShapeKind::CompoundPoly { contours, .. } = &mut self.kind {
            for c in contours {
                for p in c {
                    p[0] = pivot[0] + (p[0] - pivot[0]) * sx;
                    p[1] = pivot[1] + (p[1] - pivot[1]) * sy;
                }
            }
        }
        if scale_stroke {
            self.stroke_width =
                (self.stroke_width * ((sx.abs() + sy.abs()) * 0.5)).clamp(1.0, 120.0);
        }
    }

    pub fn to_path_cmds(&self, force_close: bool) -> Vec<PathCmd> {
        match &self.kind {
            ShapeKind::Quad { corner_radius, .. } => {
                build_filleted_poly_cmds(&self.points, true, *corner_radius)
            }
            ShapeKind::Poly {
                closed,
                corner_radius,
            } => build_filleted_poly_cmds(&self.points, *closed || force_close, *corner_radius),
            ShapeKind::CompoundPoly {
                contours,
                corner_radius,
            } => {
                let mut all = Vec::new();
                for c in contours {
                    all.extend(build_filleted_poly_cmds(c, true, *corner_radius));
                }
                all
            }
            ShapeKind::RegularPoly {
                sides,
                inner_ratio,
                corner_radius,
            } => {
                if self.points.len() >= 2 {
                    let pts = generate_regular_poly_pts(
                        self.points[0],
                        self.points[1],
                        *sides,
                        *inner_ratio,
                    );
                    build_filleted_poly_cmds(&pts, true, *corner_radius)
                } else {
                    Vec::new()
                }
            }
            ShapeKind::Line {
                arrow_start,
                arrow_end,
                arrow_size,
            } => {
                let mut cmds = Vec::new();
                if self.points.len() >= 2 {
                    let (p0, p1) = (self.points[0], self.points[1]);
                    cmds.push(PathCmd::MoveTo(p0[0], p0[1]));
                    cmds.push(PathCmd::LineTo(p1[0], p1[1]));
                    let (dx, dy) = (p1[0] - p0[0], p1[1] - p0[1]);
                    let len = (dx * dx + dy * dy).sqrt();
                    if len > 1e-3 {
                        let (ux, uy) = (dx / len, dy / len);
                        let (px, py) = (-uy, ux);
                        let w = arrow_size * 0.55;
                        if *arrow_end {
                            let (bx, by) = (p1[0] - ux * arrow_size, p1[1] - uy * arrow_size);
                            cmds.push(PathCmd::MoveTo(bx + px * w, by + py * w));
                            cmds.push(PathCmd::LineTo(p1[0], p1[1]));
                            cmds.push(PathCmd::LineTo(bx - px * w, by - py * w));
                        }
                        if *arrow_start {
                            let (bx, by) = (p0[0] + ux * arrow_size, p0[1] + uy * arrow_size);
                            cmds.push(PathCmd::MoveTo(bx + px * w, by + py * w));
                            cmds.push(PathCmd::LineTo(p0[0], p0[1]));
                            cmds.push(PathCmd::LineTo(bx - px * w, by - py * w));
                        }
                    }
                }
                cmds
            }
            ShapeKind::CircleArc {
                start_deg,
                sweep_deg,
                close_pie,
                ..
            } => {
                if self.points.len() >= 3 {
                    let c = self.points[0];
                    let rx = dist(c, self.points[1]).max(0.5);
                    let ry = dist(c, self.points[2]).max(0.5);
                    build_arc_cubic_cmds(
                        c,
                        rx,
                        ry,
                        *start_deg,
                        *sweep_deg,
                        *close_pie,
                        force_close,
                    )
                } else {
                    Vec::new()
                }
            }
            ShapeKind::CubicBezier { closed } => {
                let mut cmds = Vec::new();
                if self.points.len() >= 4 {
                    let (p0, c1, c2, p3) =
                        (self.points[0], self.points[1], self.points[2], self.points[3]);
                    cmds.push(PathCmd::MoveTo(p0[0], p0[1]));
                    cmds.push(PathCmd::CubicTo(c1[0], c1[1], c2[0], c2[1], p3[0], p3[1]));
                    if *closed || force_close {
                        cmds.push(PathCmd::Close);
                    }
                }
                cmds
            }
            ShapeKind::ThreePointArc { close_chord } => {
                let mut cmds = Vec::new();
                if self.points.len() >= 3 {
                    let (p0, p1, p2) = (self.points[0], self.points[1], self.points[2]);
                    if let Some((c, r, start_deg, sweep_deg)) = circle_from_3_points(p0, p1, p2) {
                        let mut arc_cmds =
                            build_arc_cubic_cmds(c, r, r, start_deg, sweep_deg, false, false);
                        cmds.append(&mut arc_cmds);
                        if *close_chord || force_close {
                            cmds.push(PathCmd::LineTo(p0[0], p0[1]));
                            cmds.push(PathCmd::Close);
                        }
                    } else {
                        cmds.push(PathCmd::MoveTo(p0[0], p0[1]));
                        cmds.push(PathCmd::LineTo(p2[0], p2[1]));
                        if *close_chord || force_close {
                            cmds.push(PathCmd::Close);
                        }
                    }
                }
                cmds
            }
            ShapeKind::NurbsCurve { closed } => {
                catmull_rom_to_cubic_cmds(&self.points, *closed, force_close)
            }
        }
    }
}

pub fn dist(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}


pub fn circle_from_3_points(
    p0: [f32; 2],
    p1: [f32; 2],
    p2: [f32; 2],
) -> Option<([f32; 2], f32, f32, f32)> {
    let (x1, y1) = (p0[0], p0[1]);
    let (x2, y2) = (p1[0], p1[1]);
    let (x3, y3) = (p2[0], p2[1]);

    let d = 2.0 * (x1 * (y2 - y3) + x2 * (y3 - y1) + x3 * (y1 - y2));
    if d.abs() < 1e-4 {
        return None;
    }

    let cx = ((x1 * x1 + y1 * y1) * (y2 - y3)
        + (x2 * x2 + y2 * y2) * (y3 - y1)
        + (x3 * x3 + y3 * y3) * (y1 - y2))
        / d;
    let cy = ((x1 * x1 + y1 * y1) * (x3 - x2)
        + (x2 * x2 + y2 * y2) * (x1 - x3)
        + (x3 * x3 + y3 * y3) * (x2 - x1))
        / d;

    let center = [cx, cy];
    let radius = dist(center, p0).max(0.5);

    let theta0 = (y1 - cy).atan2(x1 - cx);
    let theta1 = (y2 - cy).atan2(x2 - cx);
    let theta2 = (y3 - cy).atan2(x3 - cx);

    let sweep_ccw = (theta2 - theta0).rem_euclid(TAU);
    let mid_ccw = (theta1 - theta0).rem_euclid(TAU);

    let sweep = if mid_ccw < sweep_ccw {
        sweep_ccw
    } else {
        sweep_ccw - TAU
    };

    Some((center, radius, theta0.to_degrees(), sweep.to_degrees()))
}

pub fn catmull_rom_to_cubic_cmds(
    points: &[[f32; 2]],
    closed: bool,
    force_close: bool,
) -> Vec<PathCmd> {
    let n = points.len();
    if n < 2 {
        return Vec::new();
    }
    let mut cmds = Vec::new();
    cmds.push(PathCmd::MoveTo(points[0][0], points[0][1]));

    if n == 2 {
        cmds.push(PathCmd::LineTo(points[1][0], points[1][1]));
        if closed || force_close {
            cmds.push(PathCmd::Close);
        }
        return cmds;
    }

    let is_closed = closed || force_close;
    let seg_count = if is_closed { n } else { n - 1 };

    for i in 0..seg_count {
        let p0 = if is_closed {
            points[(i + n - 1) % n]
        } else if i == 0 {
            [
                points[0][0] * 2.0 - points[1][0],
                points[0][1] * 2.0 - points[1][1],
            ]
        } else {
            points[i - 1]
        };

        let p1 = points[i];
        let p2 = points[(i + 1) % n];

        let p3 = if is_closed {
            points[(i + 2) % n]
        } else if i + 2 >= n {
            [
                points[n - 1][0] * 2.0 - points[n - 2][0],
                points[n - 1][1] * 2.0 - points[n - 2][1],
            ]
        } else {
            points[i + 2]
        };

        let c1x = p1[0] + (p2[0] - p0[0]) / 6.0;
        let c1y = p1[1] + (p2[1] - p0[1]) / 6.0;
        let c2x = p2[0] - (p3[0] - p1[0]) / 6.0;
        let c2y = p2[1] - (p3[1] - p1[1]) / 6.0;

        cmds.push(PathCmd::CubicTo(c1x, c1y, c2x, c2y, p2[0], p2[1]));
    }

    if is_closed {
        cmds.push(PathCmd::Close);
    }

    cmds
}

pub fn sample_path_cmds_to_points(cmds: &[PathCmd]) -> Vec<[f32; 2]> {
    let mut pts = Vec::new();
    let mut current = [0.0f32, 0.0f32];
    for cmd in cmds {
        match *cmd {
            PathCmd::MoveTo(x, y) => {
                current = [x, y];
                pts.push(current);
            }
            PathCmd::LineTo(x, y) => {
                current = [x, y];
                pts.push(current);
            }
            PathCmd::QuadTo(cx, cy, x, y) => {
                for i in 1..=8 {
                    let t = i as f32 / 8.0;
                    let mt = 1.0 - t;
                    let px = mt * mt * current[0] + 2.0 * mt * t * cx + t * t * x;
                    let py = mt * mt * current[1] + 2.0 * mt * t * cy + t * t * y;
                    pts.push([px, py]);
                }
                current = [x, y];
            }
            PathCmd::CubicTo(c1x, c1y, c2x, c2y, x, y) => {
                for i in 1..=12 {
                    let t = i as f32 / 12.0;
                    let mt = 1.0 - t;
                    let px = mt.powi(3) * current[0]
                        + 3.0 * mt * mt * t * c1x
                        + 3.0 * mt * t * t * c2x
                        + t.powi(3) * x;
                    let py = mt.powi(3) * current[1]
                        + 3.0 * mt * mt * t * c1y
                        + 3.0 * mt * t * t * c2y
                        + t.powi(3) * y;
                    pts.push([px, py]);
                }
                current = [x, y];
            }
            PathCmd::Close => {
                if let Some(&first) = pts.first() {
                    pts.push(first);
                }
            }
        }
    }
    pts
}

pub fn dist_to_segment(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let (vx, vy) = (b[0] - a[0], b[1] - a[1]);
    let l2 = vx * vx + vy * vy;
    if l2 < 1e-6 {
        return dist(p, a);
    }
    let t = (((p[0] - a[0]) * vx + (p[1] - a[1]) * vy) / l2).clamp(0.0, 1.0);
    dist(p, [a[0] + t * vx, a[1] + t * vy])
}

pub fn point_in_polygon(pt: [f32; 2], poly: &[[f32; 2]]) -> bool {
    if poly.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (pi, pj) = (poly[i], poly[j]);
        if ((pi[1] > pt[1]) != (pj[1] > pt[1]))
            && (pt[0] < (pj[0] - pi[0]) * (pt[1] - pi[1]) / (pj[1] - pi[1] + 1e-7) + pi[0])
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

pub fn point_in_contours(pt: [f32; 2], contours: &[Vec<[f32; 2]>]) -> bool {
    let mut parity = false;
    for c in contours {
        if point_in_polygon(pt, c) {
            parity = !parity;
        }
    }
    parity
}

pub fn generate_regular_poly_pts(
    center: [f32; 2],
    tip: [f32; 2],
    sides: usize,
    inner_ratio: f32,
) -> Vec<[f32; 2]> {
    let sides = sides.clamp(3, 32);
    let r_outer = dist(center, tip).max(2.0);
    let base_angle = (tip[1] - center[1]).atan2(tip[0] - center[0]);
    let is_star = (inner_ratio - 1.0).abs() > 0.02;
    let mut pts = Vec::new();
    if is_star {
        let r_in = r_outer * inner_ratio.clamp(0.08, 1.0);
        for i in 0..(sides * 2) {
            let ang = base_angle + (i as f32) * TAU / ((sides * 2) as f32);
            let r = if i % 2 == 0 { r_outer } else { r_in };
            pts.push([center[0] + r * ang.cos(), center[1] + r * ang.sin()]);
        }
    } else {
        for i in 0..sides {
            let ang = base_angle + (i as f32) * TAU / (sides as f32);
            pts.push([center[0] + r_outer * ang.cos(), center[1] + r_outer * ang.sin()]);
        }
    }
    pts
}

fn sample_filleted_poly(pts: &[[f32; 2]], closed: bool, radius: f32) -> Vec<[f32; 2]> {
    let cmds = build_filleted_poly_cmds(pts, closed, radius);
    let mut out = Vec::new();
    let mut curr = [0.0, 0.0];
    for cmd in cmds {
        match cmd {
            PathCmd::MoveTo(x, y) | PathCmd::LineTo(x, y) => {
                curr = [x, y];
                out.push(curr);
            }
            PathCmd::QuadTo(cx, cy, x, y) => {
                for s in 1..=6 {
                    let t = s as f32 / 6.0;
                    let mt = 1.0 - t;
                    out.push([
                        mt * mt * curr[0] + 2.0 * mt * t * cx + t * t * x,
                        mt * mt * curr[1] + 2.0 * mt * t * cy + t * t * y,
                    ]);
                }
                curr = [x, y];
            }
            _ => {}
        }
    }
    out
}

pub fn build_filleted_poly_cmds(pts: &[[f32; 2]], closed: bool, radius: f32) -> Vec<PathCmd> {
    let n = pts.len();
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![PathCmd::MoveTo(pts[0][0], pts[0][1])];
    }
    if radius <= 0.25 || n < 3 {
        let mut cmds = Vec::with_capacity(n + 2);
        cmds.push(PathCmd::MoveTo(pts[0][0], pts[0][1]));
        for p in &pts[1..] {
            cmds.push(PathCmd::LineTo(p[0], p[1]));
        }
        if closed {
            cmds.push(PathCmd::Close);
        }
        return cmds;
    }

    let mut cmds = Vec::new();
    for i in 0..n {
        if !closed && (i == 0 || i == n - 1) {
            if i == 0 {
                cmds.push(PathCmd::MoveTo(pts[0][0], pts[0][1]));
            } else {
                cmds.push(PathCmd::LineTo(pts[i][0], pts[i][1]));
            }
            continue;
        }
        let p_prev = pts[(i + n - 1) % n];
        let p_curr = pts[i];
        let p_next = pts[(i + 1) % n];
        let v1 = [p_prev[0] - p_curr[0], p_prev[1] - p_curr[1]];
        let v2 = [p_next[0] - p_curr[0], p_next[1] - p_curr[1]];
        let l1 = (v1[0] * v1[0] + v1[1] * v1[1]).sqrt();
        let l2 = (v2[0] * v2[0] + v2[1] * v2[1]).sqrt();
        if l1 < 1e-3 || l2 < 1e-3 {
            continue;
        }
        let d = radius.min(l1 * 0.48).min(l2 * 0.48);
        let s_pt = [p_curr[0] + (v1[0] / l1) * d, p_curr[1] + (v1[1] / l1) * d];
        let e_pt = [p_curr[0] + (v2[0] / l2) * d, p_curr[1] + (v2[1] / l2) * d];
        if cmds.is_empty() {
            cmds.push(PathCmd::MoveTo(s_pt[0], s_pt[1]));
        } else {
            cmds.push(PathCmd::LineTo(s_pt[0], s_pt[1]));
        }
        cmds.push(PathCmd::QuadTo(p_curr[0], p_curr[1], e_pt[0], e_pt[1]));
    }
    if closed {
        cmds.push(PathCmd::Close);
    }
    cmds
}

pub fn build_arc_cubic_cmds(
    center: [f32; 2],
    rx: f32,
    ry: f32,
    start_deg: f32,
    sweep_deg: f32,
    close_pie: bool,
    force_close: bool,
) -> Vec<PathCmd> {
    let mut cmds = Vec::new();
    let full = sweep_deg.abs() >= 359.9;
    let sweep_rad = if full {
        TAU
    } else {
        sweep_deg.clamp(-360.0, 360.0).to_radians()
    };
    if sweep_rad.abs() < 1e-4 {
        return cmds;
    }
    let start_rad = start_deg.to_radians();
    let segs = ((sweep_rad.abs() / (PI * 0.25)).ceil() as usize).max(1);
    let d_theta = sweep_rad / (segs as f32);
    let k = (4.0 / 3.0) * (d_theta * 0.25).tan();
    let (x0, y0) = (center[0] + rx * start_rad.cos(), center[1] + ry * start_rad.sin());

    if !full && close_pie {
        cmds.push(PathCmd::MoveTo(center[0], center[1]));
        cmds.push(PathCmd::LineTo(x0, y0));
    } else {
        cmds.push(PathCmd::MoveTo(x0, y0));
    }
    for i in 0..segs {
        let a0 = start_rad + (i as f32) * d_theta;
        let a1 = a0 + d_theta;
        let (s0, c0) = a0.sin_cos();
        let (s1, c1) = a1.sin_cos();
        cmds.push(PathCmd::CubicTo(
            center[0] + rx * (c0 - k * s0),
            center[1] + ry * (s0 + k * c0),
            center[0] + rx * (c1 + k * s1),
            center[1] + ry * (s1 - k * c1),
            center[0] + rx * c1,
            center[1] + ry * s1,
        ));
    }
    if full || close_pie || force_close {
        cmds.push(PathCmd::Close);
    }
    cmds
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CadPreset {
    BooleanIntersectLens,
    ExtrudePushPull,
    ThreePointArc,
    IsometricCube,
    MultiColorEdges,
}

pub fn build_preset(preset: CadPreset) -> Vec<VectorShape> {
    match preset {
        CadPreset::BooleanIntersectLens => {
            let mut c1 = VectorShape::default_cad_style(
                1,
                "Circle A (Left)",
                ShapeKind::CircleArc {
                    lock_circle: true,
                    start_deg: 0.0,
                    sweep_deg: 360.0,
                    close_pie: false,
                },
                vec![[196.0, 256.0], [348.0, 256.0], [196.0, 408.0]],
            );
            c1.dashed = true;
            c1.stroke_width = 18.0;

            let mut c2 = VectorShape::default_cad_style(
                2,
                "Circle B (Right)",
                ShapeKind::CircleArc {
                    lock_circle: true,
                    start_deg: 0.0,
                    sweep_deg: 360.0,
                    close_pie: false,
                },
                vec![[316.0, 256.0], [468.0, 256.0], [316.0, 408.0]],
            );
            c2.dashed = true;
            c2.stroke_width = 18.0;

            let lens_contours =
                compute_boolean(&c1.to_contours(), &c2.to_contours(), BooleanOp::Intersect);
            let mut lens = VectorShape::from_contours(3, "True Intersection (A & B)", lens_contours);
            lens.fill_enabled = true;
            lens.fill_color = [255, 127, 42, 215];
            lens.stroke_enabled = true;
            lens.stroke_color = [255, 255, 255, 255];
            lens.stroke_width = 22.0;

            vec![c1, c2, lens]
        }
        CadPreset::ExtrudePushPull => {
            let mut top = VectorShape::default_cad_style(
                1,
                "Extruded Top",
                ShapeKind::Quad {
                    lock_rect: false,
                    corner_radius: 4.0,
                },
                vec![[256.0, 146.0], [416.0, 218.0], [256.0, 290.0], [96.0, 218.0]],
            );
            top.fill_enabled = true;
            top.fill_color = [255, 127, 42, 175];

            let mut left = VectorShape::default_cad_style(
                2,
                "Left Wall",
                ShapeKind::Quad {
                    lock_rect: false,
                    corner_radius: 0.0,
                },
                vec![[96.0, 218.0], [256.0, 290.0], [256.0, 438.0], [96.0, 366.0]],
            );
            left.fill_enabled = true;
            left.fill_color = [210, 210, 215, 50];

            let mut right = VectorShape::default_cad_style(
                3,
                "Right Wall",
                ShapeKind::Quad {
                    lock_rect: false,
                    corner_radius: 0.0,
                },
                vec![[256.0, 290.0], [416.0, 218.0], [416.0, 366.0], [256.0, 438.0]],
            );
            right.fill_enabled = true;
            right.fill_color = [140, 140, 150, 40];

            let mut arrow = VectorShape::default_cad_style(
                4,
                "Extrude Vector",
                ShapeKind::Line {
                    arrow_start: false,
                    arrow_end: true,
                    arrow_size: 68.0,
                },
                vec![[256.0, 224.0], [256.0, 56.0]],
            );
            arrow.stroke_color = [255, 136, 48, 255];
            arrow.stroke_width = 26.0;
            vec![left, right, top, arrow]
        }
        CadPreset::ThreePointArc => {
            let mut chord = VectorShape::default_cad_style(
                1,
                "Construction Chord",
                ShapeKind::Line {
                    arrow_start: false,
                    arrow_end: false,
                    arrow_size: 40.0,
                },
                vec![[100.0, 360.0], [412.0, 360.0]],
            );
            chord.dashed = true;
            chord.stroke_color = [165, 165, 175, 220];

            let mut arc = VectorShape::default_cad_style(
                2,
                "3-Point CAD Arc",
                ShapeKind::ThreePointArc {
                    close_chord: false,
                },
                vec![[100.0, 360.0], [256.0, 160.0], [412.0, 360.0]],
            );
            arc.stroke_width = 24.0;
            arc.stroke_color = [56, 182, 255, 255];
            arc.show_vertex_nodes = true;
            arc.vertex_node_radius = 28.0;
            vec![chord, arc]
        }
        CadPreset::IsometricCube => {
            let mut top = VectorShape::default_cad_style(
                1,
                "Top Iso Face",
                ShapeKind::Quad {
                    lock_rect: false,
                    corner_radius: 0.0,
                },
                vec![[256.0, 72.0], [432.0, 164.0], [256.0, 256.0], [80.0, 164.0]],
            );
            top.fill_enabled = true;
            top.fill_color = [240, 240, 245, 55];

            let mut left = VectorShape::default_cad_style(
                2,
                "Active Front Face",
                ShapeKind::Quad {
                    lock_rect: false,
                    corner_radius: 0.0,
                },
                vec![[80.0, 164.0], [256.0, 256.0], [256.0, 444.0], [80.0, 352.0]],
            );
            left.fill_enabled = true;
            left.fill_color = [255, 127, 42, 185];

            let mut right = VectorShape::default_cad_style(
                3,
                "Right Iso Face",
                ShapeKind::Quad {
                    lock_rect: false,
                    corner_radius: 0.0,
                },
                vec![[256.0, 256.0], [432.0, 164.0], [432.0, 352.0], [256.0, 444.0]],
            );
            right.fill_enabled = true;
            right.fill_color = [200, 200, 210, 35];
            vec![top, right, left]
        }
        CadPreset::MultiColorEdges => {
            let mut quad = VectorShape::default_cad_style(
                1,
                "Multi-Color CAD Quad",
                ShapeKind::Quad {
                    lock_rect: false,
                    corner_radius: 0.0,
                },
                vec![[160.0, 96.0], [432.0, 176.0], [352.0, 416.0], [80.0, 336.0]],
            );
            quad.fill_enabled = true;
            quad.fill_color = [255, 127, 42, 55];
            quad.stroke_width = 24.0;
            quad.per_edge_colors_enabled = true;
            quad.edge_colors = vec![
                [255, 127, 42, 255],
                [56, 182, 255, 255],
                [238, 238, 240, 255],
                [46, 204, 113, 255],
            ];
            quad.show_vertex_nodes = true;
            quad.vertex_node_radius = 26.0;
            vec![quad]
        }
    }
}

// ============================================================================
// CAD PROJECT SERIALIZATION (SAVE & LOAD)
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CadProject {
    #[serde(default = "default_project_name")]
    pub name: String,
    #[serde(default = "default_canvas_size")]
    pub canvas_size: f32,
    pub shapes: Vec<VectorShape>,
    #[serde(default)]
    pub primary_fill_color: Option<[u8; 4]>,
    #[serde(default)]
    pub primary_stroke_color: Option<[u8; 4]>,
    #[serde(default)]
    pub grid_step: Option<f32>,
    #[serde(default)]
    pub snap_to_grid: Option<bool>,
}

fn default_project_name() -> String {
    "cad_tool_icon".to_string()
}

fn default_canvas_size() -> f32 {
    512.0
}

impl CadProject {
    pub fn to_json_pretty(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }

    pub fn from_json_str(json: &str) -> Result<Self, String> {
        if let Ok(proj) = serde_json::from_str::<CadProject>(json) {
            return Ok(proj);
        }
        if let Ok(shapes) = serde_json::from_str::<Vec<VectorShape>>(json) {
            return Ok(CadProject {
                name: "cad_tool_icon".to_string(),
                canvas_size: 512.0,
                shapes,
                primary_fill_color: None,
                primary_stroke_color: None,
                grid_step: None,
                snap_to_grid: None,
            });
        }
        Err("Failed to parse project JSON file: invalid format.".to_string())
    }
}

