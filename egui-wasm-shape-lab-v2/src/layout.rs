use egui::{Vec2, vec2};

use crate::style::{RadiusRule, ResolvedStyle};

#[derive(Clone, Copy, Debug)]
pub struct LayoutParams {
    pub requested_tab_width: f32,
    pub tab_padding: f32,
    pub pill_padding: f32,
    pub control_diameter: f32,
    pub gap: f32,
    pub rotation_gap: f32,
}

impl Default for LayoutParams {
    fn default() -> Self {
        Self {
            requested_tab_width: 96.0,
            tab_padding: 14.0,
            pill_padding: 10.0,
            control_diameter: 34.0,
            gap: 10.0,
            rotation_gap: 28.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ResolvedLayout {
    pub tab_size: Vec2,
    pub pill_size: Vec2,
    pub pill_radius: f32,
    pub control_centers: Vec<Vec2>,
    pub rotation_local: Vec2,
    pub parent_roundness: f32,
}

impl ResolvedLayout {
    pub fn resolve(
        pie_count: usize,
        p: LayoutParams,
        parent_style: ResolvedStyle,
        parent_reference_size: Vec2,
        child_roundness_scale: f32,
    ) -> Self {
        let count = pie_count + 1; // plus + pies
        let controls_h = count as f32 * p.control_diameter
            + count.saturating_sub(1) as f32 * p.gap;

        let pill_w = p.control_diameter + p.pill_padding * 2.0;
        let pill_h = controls_h + p.pill_padding * 2.0;
        let pill_size = vec2(pill_w, pill_h);

        let min_tab_w = pill_w + p.tab_padding * 2.0;
        let tab_w = p.requested_tab_width.max(min_tab_w);
        let tab_h = pill_h + p.tab_padding * 2.0;
        let tab_size = vec2(tab_w, tab_h);

        let pill_style = parent_style.child(
            RadiusRule::ParentNormalized {
                scale: child_roundness_scale,
            },
            parent_reference_size,
            pill_size,
            0.82,
            egui::Color32::from_rgb(15, 22, 22),
            0.44,
        );

        let first_y = -controls_h * 0.5 + p.control_diameter * 0.5;
        let pitch = p.control_diameter + p.gap;
        let control_centers = (0..count)
            .map(|i| vec2(0.0, first_y + i as f32 * pitch))
            .collect();

        Self {
            tab_size,
            pill_size,
            pill_radius: pill_style.radius,
            control_centers,
            rotation_local: vec2(0.0, -tab_h * 0.5 - p.rotation_gap),
            parent_roundness: pill_style.parent_roundness,
        }
    }
}
