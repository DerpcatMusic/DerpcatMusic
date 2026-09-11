use egui::{Color32, Vec2};

#[derive(Clone, Copy, Debug)]
pub enum RadiusRule {
    Px(f32),
    ParentNormalized { scale: f32 },
}

#[derive(Clone, Copy, Debug)]
pub struct SurfaceStyle {
    pub radius: f32,
    pub stroke: f32,
    pub accent: Color32,
    pub fill: Color32,
}

#[derive(Clone, Copy, Debug)]
pub struct ResolvedStyle {
    pub radius: f32,
    pub stroke: f32,
    pub accent: Color32,
    pub fill: Color32,
    pub parent_roundness: f32,
}

impl SurfaceStyle {
    pub fn resolve_root(self, bounds: Vec2) -> ResolvedStyle {
        let short = bounds.x.min(bounds.y).max(1.0);
        ResolvedStyle {
            radius: self.radius.clamp(0.0, short * 0.5),
            stroke: self.stroke.max(0.0),
            accent: self.accent,
            fill: self.fill,
            parent_roundness: self.radius.max(0.0) / short,
        }
    }
}

impl ResolvedStyle {
    pub fn child(
        self,
        rule: RadiusRule,
        parent_bounds: Vec2,
        own_bounds: Vec2,
        stroke_scale: f32,
        fill: Color32,
        accent_mix_to_dark: f32,
    ) -> ResolvedStyle {
        let parent_short = parent_bounds.x.min(parent_bounds.y).max(1.0);
        let own_short = own_bounds.x.min(own_bounds.y).max(1.0);
        let parent_roundness = self.radius / parent_short;

        let raw_radius = match rule {
            RadiusRule::Px(px) => px,
            RadiusRule::ParentNormalized { scale } => {
                own_short * parent_roundness * scale.max(0.0)
            }
        };

        ResolvedStyle {
            radius: raw_radius.clamp(0.0, own_short * 0.5),
            stroke: (self.stroke * stroke_scale).max(0.0),
            accent: mix_color(
                self.accent,
                Color32::from_rgb(18, 31, 31),
                accent_mix_to_dark,
            ),
            fill,
            parent_roundness,
        }
    }
}

pub fn mix_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let [ar, ag, ab, aa] = a.to_array();
    let [br, bg, bb, ba] = b.to_array();
    let lerp = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(
        lerp(ar, br),
        lerp(ag, bg),
        lerp(ab, bb),
        lerp(aa, ba),
    )
}
