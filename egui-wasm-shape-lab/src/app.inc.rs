struct App {
    ctx: Context,
    events: Vec<Event>,
    vertices: Vec<f32>,
    indices: Vec<u32>,
    initialized: bool,

    // All state lives in logical design coordinates.
    tab_center: Pos2,
    angle: f32,
    convex: f32,
    concave: f32,
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
            tab_center: Pos2::ZERO,
            angle: 0.0,
            convex: 28.0,
            concave: 32.0,
            pies: vec![0.62, 0.27],
            stats: MergeStats::default(),
        }
    }
}

impl App {
    fn outer_style(&self) -> SurfaceStyle {
        SurfaceStyle {
            radius: self.convex,
            stroke: 1.5,
            accent: Color32::from_rgb(63, 196, 222),
            fill: Color32::from_rgb(37, 41, 41),
        }
    }

    fn layout(&self) -> ComponentLayout {
        ComponentLayout::resolve(self.pies.len(), ComponentMetrics::default(), self.outer_style())
    }

    fn attached_left_center(layout: &ComponentLayout) -> Pos2 {
        let m = ComponentMetrics::default();
        pos2(
            PANEL_LEFT + layout.tab_size.x * 0.5,
            PANEL_TOP - layout.tab_size.y * 0.5 + m.attach_overlap,
        )
    }

    fn attached_center(layout: &ComponentLayout) -> Pos2 {
        let m = ComponentMetrics::default();
        pos2(
            PANEL_CX,
            PANEL_TOP - layout.tab_size.y * 0.5 + m.attach_overlap,
        )
    }

    fn separated_center(layout: &ComponentLayout) -> Pos2 {
        pos2(
            PANEL_LEFT + layout.tab_size.x * 0.5,
            PANEL_TOP - layout.tab_size.y * 0.5 - 32.0,
        )
    }

    fn ensure_initial(&mut self) {
        if self.initialized {
            return;
        }
        let layout = self.layout();
        self.tab_center = Self::attached_left_center(&layout);
        self.initialized = true;
    }

    fn preserve_bottom_while<F: FnOnce(&mut Vec<f32>)>(&mut self, edit: F) {
        let old_layout = self.layout();
        let bottom = self.tab_center.y + old_layout.tab_size.y * 0.5;
        edit(&mut self.pies);
        let new_layout = self.layout();
        self.tab_center.y = bottom - new_layout.tab_size.y * 0.5;
    }

    fn local_to_logical(&self, local: Vec2) -> Pos2 {
        let c = self.angle.cos();
        let s = self.angle.sin();
        self.tab_center + vec2(local.x * c - local.y * s, local.x * s + local.y * c)
    }

    fn local_to_screen(&self, space: DesignSpace, local: Vec2) -> Pos2 {
        space.point(self.local_to_logical(local))
    }

    fn snap_tab(&mut self, layout: &ComponentLayout) {
        if self.angle.abs() > 0.02 {
            return;
        }
        let m = ComponentMetrics::default();
        let target_y = PANEL_TOP - layout.tab_size.y * 0.5 + m.attach_overlap;
        if (self.tab_center.y - target_y).abs() <= m.snap_distance {
            self.tab_center.y = target_y;
        }

        let candidates = [
            PANEL_LEFT + layout.tab_size.x * 0.5,
            PANEL_CX,
            PANEL_RIGHT - layout.tab_size.x * 0.5,
        ];
        if let Some(best) = candidates
            .into_iter()
            .min_by(|a, b| {
                (self.tab_center.x - *a)
                    .abs()
                    .partial_cmp(&(self.tab_center.x - *b).abs())
                    .unwrap_or(core::cmp::Ordering::Equal)
            })
        {
            if (self.tab_center.x - best).abs() <= m.snap_distance {
                self.tab_center.x = best;
            }
        }
    }

    fn draw(&mut self, ui: &mut Ui) {
        self.ensure_initial();
        let screen = ui.max_rect();
        let space = DesignSpace::fit(screen);
        let painter = ui.painter().clone();
        painter.rect_filled(screen, 0.0, Color32::from_rgb(13, 16, 16));

        let layout = self.layout();
        let outer = self.outer_style();
        let m = ComponentMetrics::default();

        // Boolean and fillet operations happen entirely in logical coordinates.
        let base = rectangle(PANEL_CX as f64, PANEL_CY as f64, PANEL_W as f64, PANEL_H as f64, 0.0);
        let tab = rectangle(
            self.tab_center.x as f64,
            self.tab_center.y as f64,
            layout.tab_size.x as f64,
            layout.tab_size.y as f64,
            self.angle as f64,
        );

        if let Ok((loops, stats)) = merged_rounded_loops(
            &[base, tab],
            self.convex as f64,
            self.concave as f64,
            0.10,
        ) {
            self.stats = stats;
            for loop_points in loops {
                let pts: Vec<Pos2> = loop_points
                    .iter()
                    .map(|p| space.point(pos2(p.x as f32, p.y as f32)))
                    .collect();
                if let Some(mesh) = polygon_mesh(&pts, outer.fill) {
                    painter.add(Shape::mesh(mesh));
                }
                painter.add(Shape::closed_line(
                    pts,
                    Stroke::new(space.scalar(outer.stroke), outer.accent),
                ));
            }
        }

        let graph_rect = space.rect(Rect::from_min_max(pos2(170.0, 430.0), pos2(820.0, 525.0)));
        let mut wave = Vec::with_capacity(100);
        for i in 0..100 {
            let t = i as f32 / 99.0;
            let x = egui::lerp(graph_rect.x_range(), t);
            let y = graph_rect.center().y
                + (t * 8.0 * core::f32::consts::TAU).sin() * graph_rect.height() * 0.27;
            wave.push(pos2(x, y));
        }
        painter.add(Shape::line(
            wave,
            Stroke::new(space.scalar(1.35), Color32::from_rgba_premultiplied(113, 141, 131, 115)),
        ));

        let tab_corners: Vec<Pos2> = [
            vec2(-layout.tab_size.x * 0.5, -layout.tab_size.y * 0.5),
            vec2( layout.tab_size.x * 0.5, -layout.tab_size.y * 0.5),
            vec2( layout.tab_size.x * 0.5,  layout.tab_size.y * 0.5),
            vec2(-layout.tab_size.x * 0.5,  layout.tab_size.y * 0.5),
        ]
        .into_iter()
        .map(|p| self.local_to_screen(space, p))
        .collect();
        let tab_bounds = bounds_of(&tab_corners).expand(space.scalar(2.0));
        let tab_response = ui.interact(tab_bounds, Id::new("outer-tab-drag"), Sense::drag());
        if tab_response.dragged() {
            let delta = ui.input(|i| i.pointer.delta());
            self.tab_center += space.logical_delta(delta);
            self.snap_tab(&layout);
        }

        let top = self.local_to_screen(space, vec2(0.0, -layout.tab_size.y * 0.5));
        let rot_handle = self.local_to_screen(space, layout.rotation_local);
        let rot_rect = Rect::from_center_size(rot_handle, space.vec(vec2(28.0, 28.0)));
        let rot_response = ui.interact(rot_rect, Id::new("rotate-handle"), Sense::drag());
        painter.line_segment([top, rot_handle], Stroke::new(space.scalar(1.0), Color32::from_rgb(53, 174, 204)));
        painter.circle_filled(rot_handle, space.scalar(10.0), Color32::from_rgb(31, 42, 42));
        painter.circle_stroke(rot_handle, space.scalar(10.0), Stroke::new(space.scalar(1.4), outer.accent));
        painter.circle_stroke(rot_handle, space.scalar(4.5), Stroke::new(space.scalar(1.0), Color32::from_rgb(121, 165, 170)));
        if rot_response.dragged() {
            if let Some(p) = ui.input(|i| i.pointer.interact_pos()) {
                let c = space.point(self.tab_center);
                self.angle = (p.y - c.y).atan2(p.x - c.x) + core::f32::consts::FRAC_PI_2;
            }
        }

        // Child surface style derives from the parent surface style.
        let pill_style = outer.child(
            InheritStyle {
                radius_scale: 32.0 / 28.0,
                stroke_scale: 0.93,
                accent_mix_to_dark: 0.45,
                fill: Color32::from_rgb(16, 24, 23),
            },
            layout.pill_size.x * 0.5,
        );
        let pill_local = rounded_rect_points(
            layout.pill_size.x,
            layout.pill_size.y,
            layout.pill_radius,
            9,
        );
        let pill_world: Vec<Pos2> = pill_local
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

        // All child centers come from intrinsic layout; no per-item coordinates.
        let plus_center = self.local_to_screen(space, layout.control_centers[0]);
        let plus_hit = space.scalar(m.control_hit_radius * 2.0);
        let plus_resp = ui.interact(
            Rect::from_center_size(plus_center, vec2(plus_hit, plus_hit)),
            Id::new("add-pie"),
            Sense::click(),
        );
        draw_plus(&painter, plus_center, space.scalar(m.plus_radius), plus_resp.hovered(), space.scale, outer.accent);
        if plus_resp.clicked() && self.pies.len() < 5 {
            self.preserve_bottom_while(|pies| pies.push(0.5));
        }

        for i in 0..self.pies.len() {
            let center = self.local_to_screen(space, layout.control_centers[i + 1]);
            let hit = space.scalar(m.control_hit_radius * 2.0);
            let response = ui.interact(
                Rect::from_center_size(center, vec2(hit, hit)),
                Id::new(("pie", i)),
                Sense::drag(),
            );
            if response.dragged() {
                let dy = ui.input(|input| input.pointer.delta().y) / space.scale;
                self.pies[i] = (self.pies[i] - dy / 150.0).clamp(0.0, 1.0);
            }
            draw_pie(
                &painter,
                center,
                space.scalar(m.pie_radius),
                self.pies[i],
                response.hovered(),
                space.scale,
                outer.accent,
            );
        }

        // Auto-aligned controls anchored in the logical design space.
        let controls_x = 770.0;
        let controls_w = 178.0;
        let slider_y = 78.0;
        custom_slider(
            ui,
            &painter,
            Id::new("convex-slider"),
            space.rect(Rect::from_min_size(pos2(controls_x, slider_y), vec2(controls_w, 28.0))),
            &mut self.convex,
            0.0,
            64.0,
            space.scale,
        );
        custom_slider(
            ui,
            &painter,
            Id::new("concave-slider"),
            space.rect(Rect::from_min_size(pos2(controls_x, slider_y + 52.0), vec2(controls_w, 28.0))),
            &mut self.concave,
            0.0,
            80.0,
            space.scale,
        );
        let mut deg = self.angle.to_degrees();
        custom_slider(
            ui,
            &painter,
            Id::new("rotation-slider"),
            space.rect(Rect::from_min_size(pos2(controls_x, slider_y + 104.0), vec2(controls_w, 28.0))),
            &mut deg,
            -180.0,
            180.0,
            space.scale,
        );
        self.angle = deg.to_radians();

        let by = 574.0;
        let bx = 40.0;
        let bw = 44.0;
        for i in 0..4 {
            let r = space.rect(Rect::from_min_size(pos2(bx + i as f32 * (bw + 10.0), by), vec2(bw, 30.0)));
            let resp = ui.interact(r, Id::new(("preset", i)), Sense::click());
            painter.rect_filled(r, space.scalar(7.0), if resp.hovered() { Color32::from_rgb(43,49,49) } else { Color32::from_rgb(30,35,35) });
            painter.rect_stroke(r, space.scalar(7.0), Stroke::new(space.scalar(1.0), Color32::from_rgb(62,71,71)), egui::StrokeKind::Inside);
            draw_preset_icon(&painter, r, i, space.scale);
            if resp.clicked() {
                match i {
                    0 => { self.tab_center = Self::attached_left_center(&layout); self.angle = 0.0; }
                    1 => { self.tab_center = Self::attached_center(&layout); self.angle = 0.0; }
                    2 => { self.tab_center = Self::separated_center(&layout); self.angle = 0.0; }
                    _ => {
                        self.convex = 28.0;
                        self.concave = 32.0;
                        self.pies = vec![0.62, 0.27];
                        let reset_layout = self.layout();
                        self.tab_center = Self::attached_left_center(&reset_layout);
                        self.angle = 0.0;
                    }
                }
            }
        }

        // EGUI proof badge; still rendered by egui, not HTML.
        let proof = space.rect(Rect::from_min_size(pos2(24.0, 24.0), vec2(92.0, 24.0)));
        painter.rect_filled(proof, space.scalar(12.0), Color32::from_rgb(21,29,29));
        painter.rect_stroke(proof, space.scalar(12.0), Stroke::new(space.scalar(1.0), Color32::from_rgb(45,108,115)), egui::StrokeKind::Inside);
        for i in 0..4 {
            let x = proof.left() + space.scalar(16.0 + i as f32 * 18.0);
            let h = space.scalar(5.0 + i as f32 * 2.8);
            painter.rect_filled(Rect::from_center_size(pos2(x,proof.center().y), vec2(space.scalar(7.0),h)), space.scalar(3.5), outer.accent);
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
                    let [r,g,b,a] = v.color.to_array();
                    self.vertices.extend_from_slice(&[
                        v.pos.x, v.pos.y,
                        r as f32 / 255.0,
                        g as f32 / 255.0,
                        b as f32 / 255.0,
                        a as f32 / 255.0,
                    ]);
                }
                self.indices.extend(mesh.indices.into_iter().map(|i| base+i));
            }
        }
    }
}
