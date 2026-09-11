use std::f64::consts::PI;

const EPS: f64 = 1.0e-7;
const KEY_SCALE: f64 = 1_000_000.0;
#[derive(Clone, Copy, Debug, Default)]
pub struct P {
    pub x: f64,
    pub y: f64,
}

impl P {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }

    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }

    fn mul(self, s: f64) -> Self {
        Self::new(self.x * s, self.y * s)
    }

    fn dot(self, rhs: Self) -> f64 {
        self.x * rhs.x + self.y * rhs.y
    }

    fn cross(self, rhs: Self) -> f64 {
        self.x * rhs.y - self.y * rhs.x
    }

    fn len(self) -> f64 {
        self.x.hypot(self.y)
    }

    fn dist(self, rhs: Self) -> f64 {
        self.sub(rhs).len()
    }

    fn unit(self) -> Self {
        let l = self.len();
        if l <= EPS {
            Self::new(0.0, 0.0)
        } else {
            self.mul(1.0 / l)
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Edge {
    a: P,
    b: P,
}

#[derive(Clone, Copy, Debug)]
struct Segment {
    a: P,
    b: P,
    used: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Corner {
    pub vertex: P,
    pub start: P,
    pub end: P,
    pub turn: f64,
    pub radius: f64,
    pub requested: f64,
    pub concave: bool,
    pub limited: bool,
}

fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    v.max(lo).min(hi)
}

fn key(p: P) -> (i64, i64) {
    (
        (p.x * KEY_SCALE).round() as i64,
        (p.y * KEY_SCALE).round() as i64,
    )
}

fn same_key(a: P, b: P) -> bool {
    key(a) == key(b)
}

fn polygon_area(poly: &[P]) -> f64 {
    if poly.len() < 3 {
        return 0.0;
    }

    let mut a = 0.0;
    for i in 0..poly.len() {
        a += poly[i].cross(poly[(i + 1) % poly.len()]);
    }
    a * 0.5
}

pub fn rectangle(cx: f64, cy: f64, w: f64, h: f64, angle: f64) -> Vec<P> {
    let c = angle.cos();
    let s = angle.sin();
    let hw = w * 0.5;
    let hh = h * 0.5;

    [
        P::new(-hw, -hh),
        P::new(hw, -hh),
        P::new(hw, hh),
        P::new(-hw, hh),
    ]
    .iter()
    .map(|p| P::new(cx + p.x * c - p.y * s, cy + p.x * s + p.y * c))
    .collect()
}

fn point_inside(p: P, poly: &[P]) -> bool {
    let mut hit = false;
    let n = poly.len();
    if n < 3 {
        return false;
    }

    let mut j = n - 1;
    for i in 0..n {
        let a = poly[i];
        let b = poly[j];
        let crosses = (a.y > p.y) != (b.y > p.y);
        if crosses {
            let x = (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x;
            if p.x < x {
                hit = !hit;
            }
        }
        j = i;
    }
    hit
}

fn point_segment_distance(p: P, a: P, b: P) -> f64 {
    let ab = b.sub(a);
    let n = ab.dot(ab);
    if n <= EPS {
        return p.dist(a);
    }
    let t = clamp(p.sub(a).dot(ab) / n, 0.0, 1.0);
    p.dist(a.add(ab.mul(t)))
}

fn split_parameters(a: P, b: P, c: P, d: P) -> Vec<f64> {
    let r = b.sub(a);
    let s = d.sub(c);
    let q = c.sub(a);
    let den = r.cross(s);
    let scale = r.len() * s.len();

    if den.abs() > 1.0e-12 * scale.max(1.0) {
        let t = q.cross(s) / den;
        let u = q.cross(r) / den;
        if t >= -EPS && t <= 1.0 + EPS && u >= -EPS && u <= 1.0 + EPS {
            return vec![clamp(t, 0.0, 1.0)];
        }
        return Vec::new();
    }

    if q.cross(r).abs() > EPS * r.len().max(1.0) {
        return Vec::new();
    }

    let rr = r.dot(r);
    if rr <= EPS {
        return Vec::new();
    }

    [c, d]
        .iter()
        .filter_map(|p| {
            let t = p.sub(a).dot(r) / rr;
            if t >= -EPS && t <= 1.0 + EPS {
                Some(clamp(t, 0.0, 1.0))
            } else {
                None
            }
        })
        .collect()
}

fn clean_polygon(poly: &[P]) -> Vec<P> {
    let mut p = poly.to_vec();
    if p.len() > 1 && p[0].dist(*p.last().unwrap()) < EPS {
        p.pop();
    }

    let max_passes = p.len().max(1);
    for _ in 0..max_passes {
        if p.len() < 3 {
            break;
        }

        let mut changed = false;
        let mut q = Vec::with_capacity(p.len());

        for i in 0..p.len() {
            let prev = p[(i + p.len() - 1) % p.len()];
            let cur = p[i];
            let next = p[(i + 1) % p.len()];
            let a = cur.sub(prev);
            let b = next.sub(cur);
            let la = a.len();
            let lb = b.len();

            if la < EPS
                || lb < EPS
                || (a.cross(b).abs() < 1.0e-10 * la * lb && a.dot(b) > 0.0)
            {
                changed = true;
                continue;
            }
            q.push(cur);
        }

        p = q;
        if !changed {
            break;
        }
    }

    p
}

fn in_union(p: P, polys: &[Vec<P>]) -> bool {
    polys.iter().any(|poly| point_inside(p, poly))
}

fn segment_exists(segments: &[Segment], a: P, b: P) -> bool {
    let ka = key(a);
    let kb = key(b);
    segments
        .iter()
        .any(|s| key(s.a) == ka && key(s.b) == kb)
}

/// Boolean union for an arrangement of simple polygons.
/// Every input edge is split at intersections, exposed sub-edges survive,
/// and the surviving directed pieces are stitched into closed contours.
pub fn union(polygons: &[Vec<P>]) -> Result<Vec<Vec<P>>, u32> {
    let polys: Vec<Vec<P>> = polygons
        .iter()
        .map(|p| clean_polygon(p))
        .filter(|p| p.len() >= 3)
        .collect();

    let mut edges = Vec::new();
    for poly in &polys {
        for i in 0..poly.len() {
            edges.push(Edge {
                a: poly[i],
                b: poly[(i + 1) % poly.len()],
            });
        }
    }

    let mut segments: Vec<Segment> = Vec::new();

    for edge in &edges {
        let mut ts = vec![0.0, 1.0];
        for other in &edges {
            ts.extend(split_parameters(edge.a, edge.b, other.a, other.b));
        }
        ts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        ts.dedup_by(|a, b| (*a - *b).abs() <= 1.0e-10);

        let e = edge.b.sub(edge.a);
        let el = e.len();
        if el <= EPS {
            continue;
        }
        let n = P::new(-e.y / el, e.x / el);

        for pair in ts.windows(2) {
            let t0 = pair[0];
            let t1 = pair[1];
            let mut p = edge.a.add(e.mul(t0));
            let mut q = edge.a.add(e.mul(t1));
            let length = p.dist(q);
            if length < EPS {
                continue;
            }

            let m = p.add(q).mul(0.5);
            let probe = 1.0e-8_f64.max(1.0e-5_f64.min(length * 1.0e-5));
            let left = in_union(m.add(n.mul(probe)), &polys);
            let right = in_union(m.add(n.mul(-probe)), &polys);

            if left == right {
                continue;
            }
            if !left {
                std::mem::swap(&mut p, &mut q);
            }

            if !segment_exists(&segments, p, q) {
                segments.push(Segment {
                    a: p,
                    b: q,
                    used: false,
                });
            }
        }
    }

    let mut loops: Vec<Vec<P>> = Vec::new();

    for seed in 0..segments.len() {
        if segments[seed].used {
            continue;
        }

        segments[seed].used = true;
        let first = segments[seed].a;
        let mut last = segments[seed].b;
        let mut dir = last.sub(first);
        let mut points = vec![first];
        let mut guard = 0usize;

        while !same_key(last, first) {
            points.push(last);

            let mut best: Option<(usize, f64)> = None;
            for i in 0..segments.len() {
                if segments[i].used || !same_key(segments[i].a, last) {
                    continue;
                }
                let v = segments[i].b.sub(segments[i].a);
                let angle = dir.cross(v).atan2(dir.dot(v));
                match best {
                    Some((_, best_angle)) if angle >= best_angle => {}
                    _ => best = Some((i, angle)),
                }
            }

            let Some((next, _)) = best else {
                return Err(1);
            };

            segments[next].used = true;
            dir = segments[next].b.sub(segments[next].a);
            last = segments[next].b;
            guard += 1;
            if guard > segments.len() + 1 {
                return Err(2);
            }
        }

        let loop_points = clean_polygon(&points);
        if loop_points.len() >= 3 && polygon_area(&loop_points).abs() > EPS {
            loops.push(loop_points);
        }
    }

    Ok(loops)
}

/// Solve tangent circular fillets after Boolean topology. Radius is capped by
/// conservative local feature clearance, so tiny shoulders force tiny radii.
pub fn solve_fillets(
    loops: &[Vec<P>],
    convex_radius: f64,
    concave_radius: f64,
    safety: f64,
) -> Vec<Vec<Corner>> {
    let mut solved = Vec::with_capacity(loops.len());

    for (ri, ring) in loops.iter().enumerate() {
        let mut corners = Vec::with_capacity(ring.len());

        for i in 0..ring.len() {
            let v = ring[i];
            let prev = ring[(i + ring.len() - 1) % ring.len()];
            let next = ring[(i + 1) % ring.len()];
            let a = v.sub(prev).unit();
            let b = next.sub(v).unit();
            let turn = a.cross(b).atan2(a.dot(b));
            let concave = turn < 0.0;
            let requested = if concave {
                concave_radius.max(0.0)
            } else {
                convex_radius.max(0.0)
            };

            let mut clear = f64::INFINITY;
            for (rj, other) in loops.iter().enumerate() {
                for j in 0..other.len() {
                    let q = other[j];
                    if ri != rj || i != j {
                        clear = clear.min(v.dist(q));
                    }

                    let k = (j + 1) % other.len();
                    if ri != rj || (j != i && k != i) {
                        clear = clear.min(point_segment_distance(v, q, other[k]));
                    }
                }
            }

            let factor = (turn.abs() * 0.5).tan();
            let valid = turn.abs() > 1.0e-8
                && turn.abs() < PI - 1.0e-6
                && factor.is_finite()
                && factor > EPS
                && clear.is_finite();

            let trim = if valid {
                (requested * factor).min(clear * safety)
            } else {
                0.0
            };
            let radius = if valid { trim / factor } else { 0.0 };
            let start = v.sub(a.mul(trim));
            let end = v.add(b.mul(trim));

            corners.push(Corner {
                vertex: v,
                start,
                end,
                turn,
                radius,
                requested,
                concave,
                limited: radius + 1.0e-5 < requested,
            });
        }

        solved.push(corners);
    }

    solved
}

/// Boolean-union transformed polygons and replace each raw corner with sampled
/// tangent circular fillets. The result is renderer-independent dense loops.
pub fn merged_rounded_loops(
    polygons: &[Vec<P>],
    convex_radius: f64,
    concave_radius: f64,
    max_arc_step: f64,
) -> Result<(Vec<Vec<P>>, MergeStats), u32> {
    let raw = union(polygons)?;
    let solved = solve_fillets(&raw, convex_radius, concave_radius, 0.49);
    let mut out = Vec::with_capacity(solved.len());
    let mut stats = MergeStats::default();
    stats.contours = raw.len() as u32;
    stats.raw_vertices = raw.iter().map(|r| r.len() as u32).sum();

    for ring in &solved {
        if ring.is_empty() { continue; }
        let mut pts = Vec::new();
        for c in ring {
            if c.concave { stats.concave += 1; }
            if c.limited { stats.limited += 1; }

            if pts.last().map_or(true, |p: &P| p.dist(c.start) > 1.0e-5) {
                pts.push(c.start);
            }

            if c.radius <= EPS || c.start.dist(c.end) <= EPS {
                pts.push(c.end);
                continue;
            }

            let incoming = c.vertex.sub(c.start).unit();
            let normal_left = P::new(-incoming.y, incoming.x);
            let side = if c.turn > 0.0 { 1.0 } else { -1.0 };
            let center = c.start.add(normal_left.mul(c.radius * side));
            let a0 = (c.start.y - center.y).atan2(c.start.x - center.x);
            let a1 = (c.end.y - center.y).atan2(c.end.x - center.x);
            let mut sweep = a1 - a0;
            if c.turn > 0.0 {
                while sweep <= 0.0 { sweep += 2.0 * PI; }
            } else {
                while sweep >= 0.0 { sweep -= 2.0 * PI; }
            }
            if sweep.abs() > PI {
                sweep -= sweep.signum() * 2.0 * PI;
            }
            let step = max_arc_step.max(0.03).min(0.5);
            let n = (sweep.abs() / step).ceil().max(1.0) as usize;
            for j in 1..=n {
                let a = a0 + sweep * (j as f64 / n as f64);
                pts.push(P::new(center.x + c.radius * a.cos(), center.y + c.radius * a.sin()));
            }
        }
        let cleaned = clean_polygon(&pts);
        if cleaned.len() >= 3 { out.push(cleaned); }
    }
    Ok((out, stats))
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MergeStats {
    pub contours: u32,
    pub raw_vertices: u32,
    pub concave: u32,
    pub limited: u32,
}
