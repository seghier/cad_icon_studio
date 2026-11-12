use crate::model::{dist, point_in_contours};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BooleanOp {
    Union,       // A ∪ B (Add)
    Subtract,    // A - B (Minus Front)
    Intersect,   // A ∩ B (Intersection)
    Xor,         // A ⊕ B (Exclude Overlap)
}

#[derive(Clone, Debug)]
struct DirectedSeg {
    a: [f32; 2],
    b: [f32; 2],
}

/// Ensures contour has positive signed area (clockwise in screen coordinates where +Y is down)
pub fn normalize_cw(mut poly: Vec<[f32; 2]>) -> Vec<[f32; 2]> {
    poly.dedup_by(|p1, p2| dist(*p1, *p2) < 0.05);
    if poly.len() > 2 && dist(poly[0], *poly.last().unwrap()) < 0.05 {
        poly.pop();
    }
    if signed_area(&poly) < 0.0 {
        poly.reverse();
    }
    poly
}

pub fn signed_area(poly: &[[f32; 2]]) -> f32 {
    if poly.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for i in 0..poly.len() {
        let p1 = poly[i];
        let p2 = poly[(i + 1) % poly.len()];
        area += p1[0] * p2[1] - p2[0] * p1[1];
    }
    area * 0.5
}

/// Computes exact 2D Boolean Pathfinder result between two contour sets A and B
pub fn compute_boolean(
    contours_a: &[Vec<[f32; 2]>],
    contours_b: &[Vec<[f32; 2]>],
    op: BooleanOp,
) -> Vec<Vec<[f32; 2]>> {
    if op == BooleanOp::Xor {
        let mut out = compute_boolean(contours_a, contours_b, BooleanOp::Subtract);
        let mut b_minus_a = compute_boolean(contours_b, contours_a, BooleanOp::Subtract);
        out.append(&mut b_minus_a);
        return out;
    }

    let norm_a: Vec<Vec<[f32; 2]>> = contours_a
        .iter()
        .filter(|c| c.len() >= 3)
        .map(|c| normalize_cw(c.clone()))
        .collect();

    // Perturb B by an imperceptible 0.004px to prevent degenerate collinear/vertex-on-edge ties
    let norm_b: Vec<Vec<[f32; 2]>> = contours_b
        .iter()
        .filter(|c| c.len() >= 3)
        .map(|c| {
            let shifted: Vec<[f32; 2]> = c
                .iter()
                .map(|p| [p[0] + 0.0037, p[1] + 0.0051])
                .collect();
            normalize_cw(shifted)
        })
        .collect();

    if norm_a.is_empty() || norm_b.is_empty() {
        return match op {
            BooleanOp::Union | BooleanOp::Subtract => norm_a,
            BooleanOp::Intersect | BooleanOp::Xor => Vec::new(),
        };
    }

    let segs_a = contours_to_segments(&norm_a);
    let segs_b = contours_to_segments(&norm_b);

    let split_a = split_segments_against(&segs_a, &segs_b);
    let split_b = split_segments_against(&segs_b, &segs_a);

    let mut kept_segments = Vec::new();

    // Filter segments of A based on whether their midpoint is inside B
    for seg in &split_a {
        let mid = [(seg.a[0] + seg.b[0]) * 0.5, (seg.a[1] + seg.b[1]) * 0.5];
        let inside_b = point_in_contours(mid, &norm_b);
        let keep = match op {
            BooleanOp::Union | BooleanOp::Subtract => !inside_b,
            BooleanOp::Intersect => inside_b,
            BooleanOp::Xor => false,
        };
        if keep {
            kept_segments.push(seg.clone());
        }
    }

    // Filter segments of B based on whether their midpoint is inside A
    for seg in &split_b {
        let mid = [(seg.a[0] + seg.b[0]) * 0.5, (seg.a[1] + seg.b[1]) * 0.5];
        let inside_a = point_in_contours(mid, &norm_a);
        match op {
            BooleanOp::Union => {
                if !inside_a {
                    kept_segments.push(seg.clone());
                }
            }
            BooleanOp::Intersect => {
                if inside_a {
                    kept_segments.push(seg.clone());
                }
            }
            BooleanOp::Subtract => {
                if inside_a {
                    // Reverse segment direction so hole/cutout boundary winds properly
                    kept_segments.push(DirectedSeg { a: seg.b, b: seg.a });
                }
            }
            BooleanOp::Xor => {}
        }
    }

    stitch_segments_into_contours(kept_segments)
}

/// Pathfinder "Divide": splits two overlapping shapes into (A ∩ B), (A - B), and (B - A)
pub fn divide_two_shapes(
    contours_a: &[Vec<[f32; 2]>],
    contours_b: &[Vec<[f32; 2]>],
) -> Vec<(&'static str, Vec<Vec<[f32; 2]>>)> {
    let mut parts = Vec::new();
    let inter = compute_boolean(contours_a, contours_b, BooleanOp::Intersect);
    if !inter.is_empty() {
        parts.push(("Intersect (A ∩ B)", inter));
    }
    let a_sub_b = compute_boolean(contours_a, contours_b, BooleanOp::Subtract);
    if !a_sub_b.is_empty() {
        parts.push(("Part (A - B)", a_sub_b));
    }
    let b_sub_a = compute_boolean(contours_b, contours_a, BooleanOp::Subtract);
    if !b_sub_a.is_empty() {
        parts.push(("Part (B - A)", b_sub_a));
    }
    parts
}

fn contours_to_segments(contours: &[Vec<[f32; 2]>]) -> Vec<DirectedSeg> {
    let mut out = Vec::new();
    for c in contours {
        for i in 0..c.len() {
            let a = c[i];
            let b = c[(i + 1) % c.len()];
            if dist(a, b) > 0.02 {
                out.push(DirectedSeg { a, b });
            }
        }
    }
    out
}

fn split_segments_against(subject: &[DirectedSeg], clip: &[DirectedSeg]) -> Vec<DirectedSeg> {
    let mut result = Vec::new();
    for s in subject {
        let mut ts = vec![0.0f32, 1.0f32];
        for c in clip {
            if let Some((t_s, _t_c)) = segment_intersection(s.a, s.b, c.a, c.b) {
                ts.push(t_s);
            }
        }
        ts.sort_by(|a, b| a.partial_cmp(b).unwrap());
        ts.dedup_by(|a, b| (*a - *b).abs() < 1e-4);

        for w in ts.windows(2) {
            let t0 = w[0];
            let t1 = w[1];
            let p0 = [
                s.a[0] + (s.b[0] - s.a[0]) * t0,
                s.a[1] + (s.b[1] - s.a[1]) * t0,
            ];
            let p1 = [
                s.a[0] + (s.b[0] - s.a[0]) * t1,
                s.a[1] + (s.b[1] - s.a[1]) * t1,
            ];
            if dist(p0, p1) > 0.02 {
                result.push(DirectedSeg { a: p0, b: p1 });
            }
        }
    }
    result
}

fn segment_intersection(
    p1: [f32; 2],
    p2: [f32; 2],
    p3: [f32; 2],
    p4: [f32; 2],
) -> Option<(f32, f32)> {
    let r = [p2[0] - p1[0], p2[1] - p1[1]];
    let s = [p4[0] - p3[0], p4[1] - p3[1]];
    let denom = r[0] * s[1] - r[1] * s[0];
    if denom.abs() < 1e-6 {
        return None;
    }
    let qp = [p3[0] - p1[0], p3[1] - p1[1]];
    let t = (qp[0] * s[1] - qp[1] * s[0]) / denom;
    let u = (qp[0] * r[1] - qp[1] * r[0]) / denom;
    if (-1e-4..=1.0001).contains(&t) && (-1e-4..=1.0001).contains(&u) {
        Some((t.clamp(0.0, 1.0), u.clamp(0.0, 1.0)))
    } else {
        None
    }
}

fn stitch_segments_into_contours(segs: Vec<DirectedSeg>) -> Vec<Vec<[f32; 2]>> {
    let mut contours = Vec::new();
    let mut used = vec![false; segs.len()];

    for start_idx in 0..segs.len() {
        if used[start_idx] {
            continue;
        }
        used[start_idx] = true;
        let mut chain = vec![segs[start_idx].a, segs[start_idx].b];

        loop {
            let curr_end = *chain.last().unwrap();
            if chain.len() >= 3 && dist(curr_end, chain[0]) < 0.6 {
                chain.pop();
                break;
            }
            let mut best_next = None;
            let mut best_d = 1.5f32; // Snap tolerance in 512x512 canvas units
            for (i, s) in segs.iter().enumerate() {
                if !used[i] {
                    let d = dist(curr_end, s.a);
                    if d < best_d {
                        best_d = d;
                        best_next = Some(i);
                    }
                }
            }
            if let Some(next_i) = best_next {
                used[next_i] = true;
                chain.push(segs[next_i].b);
            } else {
                break;
            }
        }

        let simplified = simplify_contour(&chain, 0.35);
        if simplified.len() >= 3 && signed_area(&simplified).abs() > 8.0 {
            contours.push(simplified);
        }
    }
    contours
}

fn simplify_contour(pts: &[[f32; 2]], collinear_tol: f32) -> Vec<[f32; 2]> {
    if pts.len() <= 4 {
        return pts.to_vec();
    }
    let mut out = Vec::with_capacity(pts.len());
    let n = pts.len();
    for i in 0..n {
        let prev = *out.last().unwrap_or(&pts[(i + n - 1) % n]);
        let curr = pts[i];
        let next = pts[(i + 1) % n];
        if dist(prev, curr) < 0.25 {
            continue;
        }
        // Check perpendicular distance of `curr` from line `prev -> next`
        let vx = next[0] - prev[0];
        let vy = next[1] - prev[1];
        let len = (vx * vx + vy * vy).sqrt();
        if len > 0.5 {
            let cross = ((curr[0] - prev[0]) * vy - (curr[1] - prev[1]) * vx).abs() / len;
            if cross < collinear_tol {
                continue;
            }
        }
        out.push(curr);
    }
    out
}
