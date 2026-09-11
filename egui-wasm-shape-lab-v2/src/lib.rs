#![allow(clippy::too_many_arguments)]
#![allow(clippy::missing_safety_doc)]

#[path = "../../egui-wasm-shape-lab/src/geometry.rs"]
mod geometry;
mod layout;
mod style;

use core::ptr;
use egui::{
    Color32, Context, Event, Id, Mesh, Modifiers, PointerButton, Pos2, RawInput, Rect,
    Sense, Shape, Stroke, Ui, Vec2, pos2, vec2,
};
use geometry::{MergeStats, merged_rounded_loops, rectangle};
use layout::{LayoutParams, ResolvedLayout};
use style::{RadiusRule, ResolvedStyle, SurfaceStyle};

const MAGIC: u32 = 0x4547_5532; // "EGU2"
const VERTEX_STRIDE: usize = 6;

const DESIGN_W: f32 = 1000.0;
const DESIGN_H: f32 = 620.0;

const PANEL_LEFT: f32 = 90.0;
const PANEL_TOP: f32 = 285.0;
const PANEL_W: f32 = 570.0;
const PANEL_H: f32 = 230.0;
const PANEL_CX: f32 = PANEL_LEFT + PANEL_W * 0.5;
const PANEL_CY: f32 = PANEL_TOP + PANEL_H * 0.5;
const PANEL_RIGHT: f32 = PANEL_LEFT + PANEL_W;

#[derive(Clone, Copy)]
struct DesignSpace {
    scale: f32,
    origin: Pos2,
}

impl DesignSpace {
    fn fit(screen: Rect) -> Self {
        let scale = (screen.width() / DESIGN_W)
            .min(screen.height() / DESIGN_H)
            .max(0.001);
        let size = vec2(DESIGN_W, DESIGN_H) * scale;
        Self {
            scale,
            origin: screen.center() - size * 0.5,
        }
    }

    fn point(self, p: Pos2) -> Pos2 {
        self.origin + p.to_vec2() * self.scale
    }

    fn vec(self, v: Vec2) -> Vec2 {
        v * self.scale
    }

    fn scalar(self, x: f32) -> f32 {
        x * self.scale
    }

    fn rect(self, r: Rect) -> Rect {
        Rect::from_min_max(self.point(r.min), self.point(r.max))
    }

    fn logical_delta(self, delta: Vec2) -> Vec2 {
        delta / self.scale
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Anchor {
    Left,
    Center,
    Right,
    Free,
}

struct App {
    ctx: Context,
    events: Vec<Event>,
    vertices: Vec<f32>,
    indices: Vec<u32>,

    initialized: bool,
    anchor: Anchor,
    tab_center: Pos2,

    parent_radius: f32,
    concave_radius: f32,
    child_roundness_scale: f32,
    requested_tab_width: f32,
    attach_overlap: f32,
    control_diameter: f32,
    gap: f32,
    pill_padding: f32,
    tab_padding: f32,
    border_width: f32,
    angle: f32,

    pies: Vec<f32>,
    stats: MergeStats,
}

impl Default for App {
    fn default() -> Self {
        Self {
            ctx: Context::default(),
            events: Vec::new(),
            vertices: Vec::new(),
            indices: Vec::new(),
            initialized: false,
            anchor: Anchor::Left,
            tab_center: Pos2::ZERO,
            parent_radius: 28.0,
            concave_radius: 30.0,
            child_roundness_scale: 1.0,
            requested_tab_width: 92.0,
            attach_overlap: 18.0,
            control_diameter: 34.0,
            gap: 10.0,
            pill_padding: 10.0,
            tab_padding: 14.0,
            border_width: 1.5,
            angle: 0.0,
            pies: vec![0.62, 0.27],
            stats: MergeStats::default(),
        }
    }
}

impl App {
    fn root_style(&self) -> SurfaceStyle {
        SurfaceStyle {
            radius: self.parent_radius,
            stroke: self.border_width,
            accent: Color32::from_rgb(58, 195, 229),
            fill: Color32::from_rgb(35, 40, 40),
        }
    }

    fn root_resolved(&self) -> ResolvedStyle {
        self.root_style().resolve_root(vec2(PANEL_W, PANEL_H))
    }

    fn layout_params(&self) -> LayoutParams {
        LayoutParams {
            requested_tab_width: self.requested_tab_width,
            tab_padding: self.tab_padding,
            pill_padding: self.pill_padding,
            control_diameter: self.control_diameter,
            gap: self.gap,
            rotation_gap: 28.0,
        }
    }

    fn layout(&self) -> ResolvedLayout {
        ResolvedLayout::resolve(
            self.pies.len(),
            self.layout_params(),
            self.root_resolved(),
            vec2(PANEL_W, PANEL_H),
            self.child_roundness_scale,
        )
    }

    fn pill_style(&self, layout: &ResolvedLayout) -> ResolvedStyle {
        self.root_resolved().child(
            RadiusRule::ParentNormalized {
                scale: self.child_roundness_scale,
            },
            vec2(PANEL_W, PANEL_H),
            layout.pill_size,
            0.82,
            Color32::from_rgb(14, 21, 21),
            0.44,
        )
    }

    fn anchored_center(&self, layout: &ResolvedLayout, anchor: Anchor) -> Pos2 {
        let x = match anchor {
            Anchor::Left => PANEL_LEFT + layout.tab_size.x * 0.5,
            Anchor::Center => PANEL_CX,
            Anchor::Right => PANEL_RIGHT - layout.tab_size.x * 0.5,
            Anchor::Free => self.tab_center.x,
        };
        let y = if anchor == Anchor::Free {
            self.tab_center.y
        } else {
            PANEL_TOP - layout.tab_size.y * 0.5 + self.attach_overlap
        };
        pos2(x, y)
    }

    fn ensure_initial(&mut self) {
        if self.initialized {
            return;
        }
        let layout = self.layout();
        self.tab_center = self.anchored_center(&layout, Anchor::Left);
        self.initialized = true;
    }

    fn apply_anchor(&mut self, layout: &ResolvedLayout) {
        if self.anchor != Anchor::Free {
            self.tab_center = self.anchored_center(layout, self.anchor);
        }
    }

    fn snap_tab(&mut self, layout: &ResolvedLayout) {
        if self.angle.abs() > 0.025 {
            return;
        }
        let target_y = PANEL_TOP - layout.tab_size.y * 0.5 + self.attach_overlap;
        let snap = 10.0;
        if (self.tab_center.y - target_y).abs() > snap {
            return;
        }
        let targets = [
            (Anchor::Left, PANEL_LEFT + layout.tab_size.x * 0.5),
            (Anchor::Center, PANEL_CX),
            (Anchor::Right, PANEL_RIGHT - layout.tab_size.x * 0.5),
        ];
        if let Some((anchor, x)) = targets
            .into_iter()
            .min_by(|a, b| {
                (self.tab_center.x - a.1)
                    .abs()
                    .partial_cmp(&(self.tab_center.x - b.1).abs())
                    .unwrap_or(core::cmp::Ordering::Equal)
            })
        {
            if (self.tab_center.x - x).abs() <= snap {
                self.anchor = anchor;
                self.tab_center = pos2(x, target_y);
            }
        }
    }

    fn local_to_logical(&self, local: Vec2) -> Pos2 {
        let c = self.angle.cos();
        let s = self.angle.sin();
        self.tab_center + vec2(local.x * c - local.y * s, local.x * s + local.y * c)
    }

    fn local_to_screen(&self, space: DesignSpace, local: Vec2) -> Pos2 {
        space.point(self.local_to_logical(local))
    }

    fn draw(&mut self, ui: &mut Ui) {
        self.ensure_initial();
        let screen = ui.max_rect();
        let space = DesignSpace::fit(screen);
        let painter = ui.painter().clone();
        painter.rect_filled(screen, 0.0, Color32::from_rgb(12, 15, 15));

        let layout = self.layout();
        self.apply_anchor(&layout);
        let root = self.root_resolved();
        let pill_style = self.pill_style(&layout);

        let base = rectangle(
            PANEL_CX as f64,
            PANEL_CY as f64,
            PANEL_W as f64,
            PANEL_H as f64,
            0.0,
        );
        let tab = rectangle(
            self.tab_center.x as f64,
            self.tab_center.y as f64,
            layout.tab_size.x as f64,
            layout.tab_size.y as f64,
            self.angle as f64,
        );

        if let Ok((loops, stats)) = merged_rounded_loops(
            &[base, tab],
            root.radius as f64,
            self.concave_radius as f64,
            0.10,
        ) {
            self.stats = stats;
            for loop_points in loops {
                let pts: Vec<Pos2> = loop_points
                    .iter()
                    .map(|p| space.point(pos2(p.x as f32, p.y as f32)))
                    .collect();
                if let Some(mesh) = polygon_mesh(&pts, root.fill) {
                    painter.add(Shape::mesh(mesh));
                }
                painter.add(Shape::closed_line(
                    pts,
                    Stroke::new(space.scalar(root.stroke), root.accent),
                ));
            }
        }

        // Panel waveform.
        let graph = space.rect(Rect::from_min_max(
            pos2(PANEL_LEFT + 44.0, PANEL_TOP + 118.0),
            pos2(PANEL_RIGHT - 44.0, PANEL_TOP + 188.0),
        ));
        let mut wave = Vec::with_capacity(110);
        for i in 0..110 {
            let t = i as f32 / 109.0;
            let x = egui::lerp(graph.x_range(), t);
            let y = graph.center().y
                + (t * 7.0 * core::f32::consts::TAU).sin() * graph.height() * 0.28;
            wave.push(pos2(x, y));
        }
        painter.add(Shape::line(
            wave,
            Stroke::new(
                space.scalar(1.25),
                Color32::from_rgba_premultiplied(108, 141, 137, 115),
            ),
        ));

        // Outer tab drag.
        let tab_corners: Vec<Pos2> = [
            vec2(-layout.tab_size.x * 0.5, -layout.tab_size.y * 0.5),
            vec2(layout.tab_size.x * 0.5, -layout.tab_size.y * 0.5),
            vec2(layout.tab_size.x * 0.5, layout.tab_size.y * 0.5),
            vec2(-layout.tab_size.x * 0.5, layout.tab_size.y * 0.5),
        ]
        .into_iter()
        .map(|p| self.local_to_screen(space, p))
        .collect();
        let tab_bounds = bounds_of(&tab_corners).expand(space.scalar(2.0));
        let tab_response = ui.interact(tab_bounds, Id::new("outer-tab"), Sense::drag());
        if tab_response.dragged() {
            self.anchor = Anchor::Free;
            self.tab_center += space.logical_delta(ui.input(|i| i.pointer.delta()));
            self.snap_tab(&layout);
        }

        // Rotation handle derives from tab height.
        let top = self.local_to_screen(space, vec2(0.0, -layout.tab_size.y * 0.5));
        let rot = self.local_to_screen(space, layout.rotation_local);
        let rot_resp = ui.interact(
            Rect::from_center_size(rot, space.vec(vec2(28.0, 28.0))),
            Id::new("rotation-handle"),
            Sense::drag(),
        );
        painter.line_segment(
            [top, rot],
            Stroke::new(space.scalar(1.0), Color32::from_rgb(53, 172, 203)),
        );
        painter.circle_filled(rot, space.scalar(10.0), Color32::from_rgb(30, 40, 40));
        painter.circle_stroke(
            rot,
            space.scalar(10.0),
            Stroke::new(space.scalar(1.4), root.accent),
        );
        painter.circle_stroke(
            rot,
            space.scalar(4.5),
            Stroke::new(space.scalar(1.0), Color32::from_rgb(118, 160, 164)),
        );
        if rot_resp.dragged() {
            if let Some(p) = ui.input(|i| i.pointer.interact_pos()) {
                let c = space.point(self.tab_center);
                self.angle = (p.y - c.y).atan2(p.x - c.x)
                    + core::f32::consts::FRAC_PI_2;
            }
        }

        // The child radius is not an absolute copy of the parent radius.
        // It inherits the parent's dimensionless roundness:
        // child_r / child_short == parent_r / parent_short (at scale = 1).
        let pill_points = rounded_rect_points(
            layout.pill_size.x,
            layout.pill_size.y,
            layout.pill_radius,
            10,
        );
        let pill_world: Vec<Pos2> = pill_points
            .into_iter()
            .map(|p| self.local_to_screen(space, p))
            .collect();
        if let Some(mesh) = polygon_mesh(&pill_world, pill_style.fill) {
            painter.add(Shape::mesh(mesh));
        }
        painter.add(Shape::closed_line(
            pill_world,
            Stroke::new(space.scalar(pill_style.stroke), pill_style.accent),
        ));

        let hit = space.scalar((self.control_diameter + 10.0).max(28.0));
        let plus_center = self.local_to_screen(space, layout.control_centers[0]);
        let plus_resp = ui.interact(
            Rect::from_center_size(plus_center, vec2(hit, hit)),
            Id::new("plus"),
            Sense::click(),
        );
        draw_plus(
            &painter,
            plus_center,
            space.scalar(self.control_diameter * 0.43),
            plus_resp.hovered(),
            space.scale,
            root.accent,
        );
        if plus_resp.clicked() && self.pies.len() < 5 {
            self.pies.push(0.5);
        }

        for i in 0..self.pies.len() {
            let center = self.local_to_screen(space, layout.control_centers[i + 1]);
            let resp = ui.interact(
                Rect::from_center_size(center, vec2(hit, hit)),
                Id::new(("pie", i)),
                Sense::drag(),
            );
            if resp.dragged() {
                let dy = ui.input(|inp| inp.pointer.delta().y) / space.scale;
                self.pies[i] = (self.pies[i] - dy / 145.0).clamp(0.0, 1.0);
            }
            draw_pie(
                &painter,
                center,
                space.scalar(self.control_diameter * 0.39),
                self.pies[i],
                resp.hovered(),
                space.scale,
                root.accent,
            );
        }

        // 11 live properties, all real egui hit targets.
        let x = 755.0;
        let w = 190.0;
        let y0 = 44.0;
        let step = 46.0;
        slider(ui, &painter, space, 1, x, y0 + step * 0.0, w, &mut self.parent_radius, 0.0, 80.0);
        slider(ui, &painter, space, 2, x, y0 + step * 1.0, w, &mut self.concave_radius, 0.0, 80.0);
        slider(ui, &painter, space, 3, x, y0 + step * 2.0, w, &mut self.child_roundness_scale, 0.0, 2.5);
        slider(ui, &painter, space, 4, x, y0 + step * 3.0, w, &mut self.requested_tab_width, 60.0, 170.0);
        slider(ui, &painter, space, 5, x, y0 + step * 4.0, w, &mut self.attach_overlap, 0.0, 50.0);
        slider(ui, &painter, space, 6, x, y0 + step * 5.0, w, &mut self.control_diameter, 20.0, 54.0);
        slider(ui, &painter, space, 7, x, y0 + step * 6.0, w, &mut self.gap, 0.0, 28.0);
        slider(ui, &painter, space, 8, x, y0 + step * 7.0, w, &mut self.pill_padding, 2.0, 28.0);
        slider(ui, &painter, space, 9, x, y0 + step * 8.0, w, &mut self.tab_padding, 2.0, 32.0);
        slider(ui, &painter, space, 10, x, y0 + step * 9.0, w, &mut self.border_width, 0.5, 4.0);
        let mut degrees = self.angle.to_degrees();
        slider(ui, &painter, space, 11, x, y0 + step * 10.0, w, &mut degrees, -90.0, 90.0);
        self.angle = degrees.to_radians();

        // Presets / remove / reset.
        let by = 572.0;
        let bx = 34.0;
        let bw = 44.0;
        for i in 0..5 {
            let r = space.rect(Rect::from_min_size(
                pos2(bx + i as f32 * (bw + 10.0), by),
                vec2(bw, 30.0),
            ));
            let resp = ui.interact(r, Id::new(("preset", i)), Sense::click());
            painter.rect_filled(
                r,
                space.scalar(7.0),
                if resp.hovered() {
                    Color32::from_rgb(44, 50, 50)
                } else {
                    Color32::from_rgb(29, 34, 34)
                },
            );
            painter.rect_stroke(
                r,
                space.scalar(7.0),
                Stroke::new(space.scalar(1.0), Color32::from_rgb(61, 71, 71)),
                egui::StrokeKind::Inside,
            );
            draw_preset_icon(&painter, r, i, space.scale);
            if resp.clicked() {
                match i {
                    0 => self.anchor = Anchor::Left,
                    1 => self.anchor = Anchor::Center,
                    2 => self.anchor = Anchor::Right,
                    3 => {
                        if !self.pies.is_empty() {
                            self.pies.pop();
                        }
                    }
                    _ => {
                        *self = App {
                            ctx: self.ctx.clone(),
                            events: core::mem::take(&mut self.events),
                            vertices: core::mem::take(&mut self.vertices),
                            indices: core::mem::take(&mut self.indices),
                            ..App::default()
                        };
                    }
                }
            }
        }

        // EGUI proof badge; labels are HTML debugger overlay only.
        let proof = space.rect(Rect::from_min_size(pos2(24.0, 24.0), vec2(92.0, 24.0)));
        painter.rect_filled(proof, space.scalar(12.0), Color32::from_rgb(21, 29, 29));
        painter.rect_stroke(
            proof,
            space.scalar(12.0),
            Stroke::new(space.scalar(1.0), Color32::from_rgb(45, 108, 115)),
            egui::StrokeKind::Inside,
        );
        for i in 0..4 {
            let px = proof.left() + space.scalar(16.0 + i as f32 * 18.0);
            let h = space.scalar(5.0 + i as f32 * 2.8);
            painter.rect_filled(
                Rect::from_center_size(
                    pos2(px, proof.center().y),
                    vec2(space.scalar(7.0), h),
                ),
                space.scalar(3.5),
                root.accent,
            );
        }
    }

    fn frame(&mut self, width: f32, height: f32, time: f64) {
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(width.max(1.0), height.max(1.0)));
        let raw = RawInput {
            screen_rect: Some(screen),
            time: Some(time),
            events: core::mem::take(&mut self.events),
            focused: true,
            ..Default::default()
        };
        let ctx = self.ctx.clone();
        let full = ctx.run_ui(raw, |ui| self.draw(ui));
        let primitives = ctx.tessellate(full.shapes, full.pixels_per_point);

        self.vertices.clear();
        self.indices.clear();
        for clipped in primitives {
            if let egui::epaint::Primitive::Mesh(mesh) = clipped.primitive {
                let base = (self.vertices.len() / VERTEX_STRIDE) as u32;
                for v in mesh.vertices {
                    let [r, g, b, a] = v.color.to_array();
                    self.vertices.extend_from_slice(&[
                        v.pos.x,
                        v.pos.y,
                        r as f32 / 255.0,
                        g as f32 / 255.0,
                        b as f32 / 255.0,
                        a as f32 / 255.0,
                    ]);
                }
                self.indices.extend(mesh.indices.into_iter().map(|i| base + i));
            }
        }
    }
}

fn slider(
    ui: &mut Ui,
    painter: &egui::Painter,
    space: DesignSpace,
    number: usize,
    x: f32,
    y: f32,
    w: f32,
    value: &mut f32,
    min: f32,
    max: f32,
) {
    let rect = space.rect(Rect::from_min_size(pos2(x, y), vec2(w, 28.0)));
    let response = ui.interact(rect, Id::new(("slider", number)), Sense::click_and_drag());
    if response.dragged() || response.clicked() {
        if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
            *value = egui::remap_clamp(pos.x, rect.left()..=rect.right(), min..=max);
        }
    }
    let track = Rect::from_center_size(rect.center(), vec2(rect.width(), space.scalar(3.0)));
    painter.rect_filled(track, space.scalar(1.5), Color32::from_rgb(61, 72, 72));
    let t = egui::remap_clamp(*value, min..=max, 0.0..=1.0);
    let knob_x = egui::lerp(rect.x_range(), t);
    painter.rect_filled(
        Rect::from_min_max(track.min, pos2(knob_x, track.max.y)),
        space.scalar(1.5),
        Color32::from_rgb(46, 187, 225),
    );
    painter.circle_filled(
        pos2(knob_x, rect.center().y),
        space.scalar(if response.hovered() { 7.0 } else { 6.0 }),
        Color32::from_rgb(52, 198, 235),
    );
    // Tiny visual index ticks: N short marks at the left. HTML debugger gives labels.
    let mark_x = rect.left() - space.scalar(12.0);
    for k in 0..number.min(3) {
        painter.circle_filled(
            pos2(mark_x - space.scalar(k as f32 * 4.0), rect.center().y),
            space.scalar(1.3),
            Color32::from_rgb(79, 119, 121),
        );
    }
}

fn bounds_of(points: &[Pos2]) -> Rect {
    let mut r = Rect::NOTHING;
    for &p in points {
        r.extend_with(p);
    }
    r
}

fn polygon_area(points: &[Pos2]) -> f32 {
    let mut a = 0.0;
    for i in 0..points.len() {
        let p = points[i];
        let q = points[(i + 1) % points.len()];
        a += p.x * q.y - q.x * p.y;
    }
    a * 0.5
}

fn cross(a: Pos2, b: Pos2, c: Pos2) -> f32 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

fn point_in_tri(p: Pos2, a: Pos2, b: Pos2, c: Pos2) -> bool {
    let c1 = cross(a, b, p);
    let c2 = cross(b, c, p);
    let c3 = cross(c, a, p);
    (c1 >= -1e-4 && c2 >= -1e-4 && c3 >= -1e-4)
        || (c1 <= 1e-4 && c2 <= 1e-4 && c3 <= 1e-4)
}

fn polygon_mesh(points: &[Pos2], color: Color32) -> Option<Mesh> {
    if points.len() < 3 {
        return None;
    }
    let mut ids: Vec<usize> = (0..points.len()).collect();
    if polygon_area(points) < 0.0 {
        ids.reverse();
    }
    let mut tris = Vec::<u32>::new();
    let mut guard = 0usize;
    while ids.len() > 3 && guard < points.len() * points.len() {
        guard += 1;
        let n = ids.len();
        let mut found = false;
        for k in 0..n {
            let ia = ids[(k + n - 1) % n];
            let ib = ids[k];
            let ic = ids[(k + 1) % n];
            let a = points[ia];
            let b = points[ib];
            let c = points[ic];
            if cross(a, b, c) <= 1e-5 {
                continue;
            }
            let mut contains = false;
            for &j in &ids {
                if j == ia || j == ib || j == ic {
                    continue;
                }
                if point_in_tri(points[j], a, b, c) {
                    contains = true;
                    break;
                }
            }
            if !contains {
                tris.extend_from_slice(&[ia as u32, ib as u32, ic as u32]);
                ids.remove(k);
                found = true;
                break;
            }
        }
        if !found {
            break;
        }
    }
    if ids.len() == 3 {
        tris.extend_from_slice(&[ids[0] as u32, ids[1] as u32, ids[2] as u32]);
    }
    if tris.is_empty() {
        return None;
    }
    let mut mesh = Mesh::default();
    mesh.vertices = points
        .iter()
        .map(|&p| egui::epaint::Vertex::untextured(p, color))
        .collect();
    mesh.indices = tris;
    Some(mesh)
}

fn rounded_rect_points(w: f32, h: f32, radius: f32, steps: usize) -> Vec<Vec2> {
    let r = radius.clamp(0.0, (w * 0.5).min(h * 0.5));
    if r <= 0.001 {
        return vec![
            vec2(-w * 0.5, -h * 0.5),
            vec2(w * 0.5, -h * 0.5),
            vec2(w * 0.5, h * 0.5),
            vec2(-w * 0.5, h * 0.5),
        ];
    }
    let centers = [
        vec2(-w * 0.5 + r, -h * 0.5 + r),
        vec2(w * 0.5 - r, -h * 0.5 + r),
        vec2(w * 0.5 - r, h * 0.5 - r),
        vec2(-w * 0.5 + r, h * 0.5 - r),
    ];
    let starts = [
        core::f32::consts::PI,
        -core::f32::consts::FRAC_PI_2,
        0.0,
        core::f32::consts::FRAC_PI_2,
    ];
    let mut out = Vec::with_capacity((steps + 1) * 4);
    for corner in 0..4 {
        for i in 0..=steps {
            let a = starts[corner]
                + core::f32::consts::FRAC_PI_2 * (i as f32 / steps.max(1) as f32);
            out.push(centers[corner] + vec2(a.cos() * r, a.sin() * r));
        }
    }
    out
}

fn draw_plus(
    p: &egui::Painter,
    c: Pos2,
    r: f32,
    hover: bool,
    scale: f32,
    accent: Color32,
) {
    let col = if hover {
        Color32::from_rgb(76, 220, 249)
    } else {
        accent
    };
    p.circle_filled(c, r, Color32::from_rgb(24, 35, 34));
    p.circle_stroke(c, r, Stroke::new(1.7 * scale, col));
    let arm = r * 0.39;
    p.line_segment([c - vec2(arm, 0.0), c + vec2(arm, 0.0)], Stroke::new(1.7 * scale, col));
    p.line_segment([c - vec2(0.0, arm), c + vec2(0.0, arm)], Stroke::new(1.7 * scale, col));
}

fn draw_pie(
    p: &egui::Painter,
    c: Pos2,
    r: f32,
    value: f32,
    hover: bool,
    scale: f32,
    accent: Color32,
) {
    let rim = if hover {
        Color32::from_rgb(83, 215, 241)
    } else {
        accent
    };
    p.circle_filled(c, r, Color32::from_rgb(39, 55, 54));
    let n = ((value * 32.0).ceil() as usize).max(1);
    let mut pts = Vec::with_capacity(n + 2);
    pts.push(c);
    let start = -core::f32::consts::FRAC_PI_2;
    let sweep = value * core::f32::consts::TAU;
    for i in 0..=n {
        let a = start + sweep * (i as f32 / n as f32);
        pts.push(c + vec2(a.cos() * r, a.sin() * r));
    }
    if let Some(m) = polygon_mesh(&pts, accent) {
        p.add(Shape::mesh(m));
    }
    p.circle_stroke(c, r, Stroke::new(1.6 * scale, rim));
}

fn draw_preset_icon(p: &egui::Painter, r: Rect, i: usize, scale: f32) {
    let c = r.center();
    let col = Color32::from_rgb(130, 152, 152);
    let v = |x: f32, y: f32| vec2(x * scale, y * scale);
    match i {
        0 => {
            p.line_segment([c - v(9.0, 7.0), c - v(9.0, -7.0)], Stroke::new(1.5 * scale, col));
            p.line_segment([c - v(5.0, 0.0), c + v(9.0, 0.0)], Stroke::new(1.5 * scale, col));
        }
        1 => {
            p.line_segment([c - v(9.0, 0.0), c + v(9.0, 0.0)], Stroke::new(1.5 * scale, col));
            p.line_segment([c - v(0.0, 7.0), c + v(0.0, 7.0)], Stroke::new(1.5 * scale, col));
        }
        2 => {
            p.line_segment([c + v(9.0, 7.0), c + v(9.0, -7.0)], Stroke::new(1.5 * scale, col));
            p.line_segment([c - v(9.0, 0.0), c + v(5.0, 0.0)], Stroke::new(1.5 * scale, col));
        }
        3 => {
            p.line_segment([c - v(7.0, 0.0), c + v(7.0, 0.0)], Stroke::new(1.5 * scale, col));
        }
        _ => {
            p.circle_stroke(c, 7.0 * scale, Stroke::new(1.2 * scale, col));
            p.line_segment([c + v(4.0, -5.0), c + v(8.0, -5.0)], Stroke::new(1.2 * scale, col));
        }
    }
}

static mut APP_PTR: *mut App = ptr::null_mut();

unsafe fn app() -> &'static mut App {
    if APP_PTR.is_null() {
        APP_PTR = Box::into_raw(Box::new(App::default()));
    }
    &mut *APP_PTR
}

#[no_mangle]
pub extern "C" fn magic() -> u32 { MAGIC }
#[no_mangle]
pub extern "C" fn egui_version_code() -> u32 { 0x0036_0002 }
#[no_mangle]
pub extern "C" fn vertex_stride() -> u32 { VERTEX_STRIDE as u32 }

#[no_mangle]
pub extern "C" fn frame(width: f32, height: f32, time: f64) -> u32 {
    unsafe {
        app().frame(width, height, time);
        app().indices.len() as u32
    }
}

#[no_mangle]
pub extern "C" fn pointer_move(x: f32, y: f32) {
    unsafe { app().events.push(Event::PointerMoved(pos2(x, y))); }
}

#[no_mangle]
pub extern "C" fn pointer_button(x: f32, y: f32, button: u32, pressed: u32) {
    let button = match button {
        1 => PointerButton::Secondary,
        2 => PointerButton::Middle,
        _ => PointerButton::Primary,
    };
    unsafe {
        app().events.push(Event::PointerButton {
            pos: pos2(x, y),
            button,
            pressed: pressed != 0,
            modifiers: Modifiers::NONE,
        });
    }
}

#[no_mangle]
pub extern "C" fn pointer_gone() {
    unsafe { app().events.push(Event::PointerGone); }
}

#[no_mangle]
pub extern "C" fn vertices_ptr() -> *const f32 { unsafe { app().vertices.as_ptr() } }
#[no_mangle]
pub extern "C" fn vertices_len() -> u32 { unsafe { app().vertices.len() as u32 } }
#[no_mangle]
pub extern "C" fn indices_ptr() -> *const u32 { unsafe { app().indices.as_ptr() } }
#[no_mangle]
pub extern "C" fn indices_len() -> u32 { unsafe { app().indices.len() as u32 } }

// Live inspector exports. The HTML sidebar reads these only for explanation;
// all actual controls/layout/geometry remain Rust + egui.
#[no_mangle] pub extern "C" fn get_parent_radius() -> f32 { unsafe { app().parent_radius } }
#[no_mangle] pub extern "C" fn get_concave_radius() -> f32 { unsafe { app().concave_radius } }
#[no_mangle] pub extern "C" fn get_child_scale() -> f32 { unsafe { app().child_roundness_scale } }
#[no_mangle] pub extern "C" fn get_requested_tab_width() -> f32 { unsafe { app().requested_tab_width } }
#[no_mangle] pub extern "C" fn get_attach_overlap() -> f32 { unsafe { app().attach_overlap } }
#[no_mangle] pub extern "C" fn get_control_diameter() -> f32 { unsafe { app().control_diameter } }
#[no_mangle] pub extern "C" fn get_gap() -> f32 { unsafe { app().gap } }
#[no_mangle] pub extern "C" fn get_pill_padding() -> f32 { unsafe { app().pill_padding } }
#[no_mangle] pub extern "C" fn get_tab_padding() -> f32 { unsafe { app().tab_padding } }
#[no_mangle] pub extern "C" fn get_border_width() -> f32 { unsafe { app().border_width } }
#[no_mangle] pub extern "C" fn get_angle_degrees() -> f32 { unsafe { app().angle.to_degrees() } }
#[no_mangle] pub extern "C" fn get_parent_short() -> f32 { PANEL_H.min(PANEL_W) }
#[no_mangle] pub extern "C" fn get_parent_roundness() -> f32 { unsafe { app().root_resolved().radius / PANEL_H.min(PANEL_W) } }
#[no_mangle] pub extern "C" fn get_pill_width() -> f32 { unsafe { app().layout().pill_size.x } }
#[no_mangle] pub extern "C" fn get_pill_height() -> f32 { unsafe { app().layout().pill_size.y } }
#[no_mangle] pub extern "C" fn get_pill_radius() -> f32 { unsafe { app().layout().pill_radius } }
#[no_mangle] pub extern "C" fn get_tab_width() -> f32 { unsafe { app().layout().tab_size.x } }
#[no_mangle] pub extern "C" fn get_tab_height() -> f32 { unsafe { app().layout().tab_size.y } }
#[no_mangle] pub extern "C" fn get_pie_count() -> u32 { unsafe { app().pies.len() as u32 } }
#[no_mangle] pub extern "C" fn get_concave_count() -> u32 { unsafe { app().stats.concave } }
#[no_mangle] pub extern "C" fn get_limited_count() -> u32 { unsafe { app().stats.limited } }
