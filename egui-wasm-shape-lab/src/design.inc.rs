// -----------------------------------------------------------------------------
// Logical design space.
// Everything in the component is laid out here, then uniformly fitted into the
// actual egui viewport. This makes the browser, plugin window, HiDPI display,
// etc. all produce the same proportions.
// -----------------------------------------------------------------------------
const DESIGN_W: f32 = 1000.0;
const DESIGN_H: f32 = 620.0;

const PANEL_CX: f32 = 495.0;
const PANEL_CY: f32 = 440.0;
const PANEL_W: f32 = 730.0;
const PANEL_H: f32 = 260.0;
const PANEL_LEFT: f32 = PANEL_CX - PANEL_W * 0.5;   // 130
const PANEL_TOP: f32 = PANEL_CY - PANEL_H * 0.5;    // 310
const PANEL_RIGHT: f32 = PANEL_CX + PANEL_W * 0.5;  // 860

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

    fn logical_delta(self, screen_delta: Vec2) -> Vec2 {
        screen_delta / self.scale
    }
}

// -----------------------------------------------------------------------------
// Shape-language inheritance.
// Child surfaces derive curve/stroke/accent properties from their parent rather
// than each component carrying unrelated magic values.
// -----------------------------------------------------------------------------
#[derive(Clone, Copy)]
struct SurfaceStyle {
    radius: f32,
    stroke: f32,
    accent: Color32,
    fill: Color32,
}

#[derive(Clone, Copy)]
struct InheritStyle {
    radius_scale: f32,
    stroke_scale: f32,
    accent_mix_to_dark: f32,
    fill: Color32,
}

impl SurfaceStyle {
    fn child(self, rule: InheritStyle, max_radius: f32) -> Self {
        Self {
            radius: (self.radius * rule.radius_scale).clamp(0.0, max_radius),
            stroke: (self.stroke * rule.stroke_scale).max(0.5),
            accent: mix_color(self.accent, Color32::from_rgb(19, 33, 32), rule.accent_mix_to_dark),
            fill: rule.fill,
        }
    }
}

fn mix_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let [ar, ag, ab, aa] = a.to_array();
    let [br, bg, bb, ba] = b.to_array();
    let lerp = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(lerp(ar, br), lerp(ag, bg), lerp(ab, bb), lerp(aa, ba))
}

// -----------------------------------------------------------------------------
// Intrinsic component metrics and resolved layout.
// Nothing inside the tab has a hand-authored x/y. The pill derives from the tab,
// the tab derives its height from the pill, and controls are centered as a stack.
// -----------------------------------------------------------------------------
#[derive(Clone, Copy)]
struct ComponentMetrics {
    tab_width: f32,
    tab_padding_y: f32,
    pill_width: f32,
    pill_center_padding: f32,
    control_pitch: f32,
    plus_radius: f32,
    pie_radius: f32,
    control_hit_radius: f32,
    rotation_gap: f32,
    attach_overlap: f32,
    snap_distance: f32,
}

impl Default for ComponentMetrics {
    fn default() -> Self {
        // Matches the original HTML proof-of-concept's logical proportions.
        Self {
            tab_width: 96.0,
            tab_padding_y: 16.0,
            pill_width: 64.0,
            pill_center_padding: 32.0,
            control_pitch: 48.0,
            plus_radius: 18.0,
            pie_radius: 16.5,
            control_hit_radius: 24.0,
            rotation_gap: 32.0,
            attach_overlap: 20.0,
            snap_distance: 8.0,
        }
    }
}

#[derive(Clone)]
struct ComponentLayout {
    tab_size: Vec2,
    pill_size: Vec2,
    pill_radius: f32,
    control_centers: Vec<Vec2>,
    rotation_local: Vec2,
}

impl ComponentLayout {
    fn resolve(pie_count: usize, metrics: ComponentMetrics, parent: SurfaceStyle) -> Self {
        let control_count = pie_count + 1; // + button plus N pies
        let pill_h = metrics.pill_center_padding * 2.0
            + control_count.saturating_sub(1) as f32 * metrics.control_pitch;
        let tab_h = pill_h + metrics.tab_padding_y * 2.0;

        // Parent convex radius -> child pill radius. At the default 28px outer
        // curve this resolves to exactly 32px, matching the original capsule.
        let pill_style = parent.child(
            InheritStyle {
                radius_scale: 32.0 / 28.0,
                stroke_scale: 0.93,
                accent_mix_to_dark: 0.45,
                fill: Color32::from_rgb(16, 24, 23),
            },
            metrics.pill_width * 0.5,
        );

        let stack_height = control_count.saturating_sub(1) as f32 * metrics.control_pitch;
        let first_y = -stack_height * 0.5;
        let control_centers = (0..control_count)
            .map(|i| vec2(0.0, first_y + i as f32 * metrics.control_pitch))
            .collect();

        Self {
            tab_size: vec2(metrics.tab_width, tab_h),
            pill_size: vec2(metrics.pill_width, pill_h),
            pill_radius: pill_style.radius,
            control_centers,
            rotation_local: vec2(0.0, -tab_h * 0.5 - metrics.rotation_gap),
        }
    }
}
