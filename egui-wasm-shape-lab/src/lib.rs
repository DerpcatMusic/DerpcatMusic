#![allow(clippy::too_many_arguments)]
#![allow(clippy::missing_safety_doc)]

mod geometry;

use core::ptr;
use egui::{
    Color32, Context, Event, Id, Mesh, Modifiers, PointerButton, Pos2, RawInput, Rect,
    Sense, Shape, Stroke, Ui, Vec2, pos2, vec2,
};
use geometry::{MergeStats, merged_rounded_loops, rectangle};

const MAGIC: u32 = 0x4547_5549; // "EGUI"
const VERTEX_STRIDE: usize = 6; // x, y, premultiplied RGBA as floats

struct App {
    ctx: Context,
    events: Vec<Event>,
    vertices: Vec<f32>,
    indices: Vec<u32>,
    initialized: bool,
    tab_center: Pos2,
    tab_size: Vec2,
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
            tab_size: vec2(112.0, 216.0),
            angle: 0.0,
            convex: 28.0,
            concave: 32.0,
            pies: vec![0.62, 0.27],
            stats: MergeStats::default(),
        }
    }
}

impl App {
    fn ensure_initial(&mut self, screen: Rect) {
        if self.initialized { return; }
        let panel_left = screen.left() + screen.width() * 0.21;
        let panel_top = screen.top() + screen.height() * 0.43;
        self.tab_center = pos2(
            panel_left + self.tab_size.x * 0.5,
            panel_top - self.tab_size.y * 0.5 + 22.0,
        );
        self.initialized = true;
    }

    fn reset_left(&mut self, screen: Rect) {
        let panel_left = screen.left() + screen.width() * 0.21;
        let panel_top = screen.top() + screen.height() * 0.43;
        self.tab_center = pos2(panel_left + self.tab_size.x * 0.5, panel_top - self.tab_size.y * 0.5 + 22.0);
        self.angle = 0.0;
    }

    fn local_to_world(&self, local: Vec2) -> Pos2 {
        let c = self.angle.cos();
        let s = self.angle.sin();
        self.tab_center + vec2(local.x * c - local.y * s, local.x * s + local.y * c)
    }

    fn draw(&mut self, ui: &mut Ui) {
        let screen = ui.max_rect();
        self.ensure_initial(screen);
        let painter = ui.painter().clone();
        painter.rect_filled(screen, 0.0, Color32::from_rgb(13, 16, 16));

        let panel_left = screen.left() + screen.width() * 0.21;
        let panel_top = screen.top() + screen.height() * 0.43;
        let panel_right = screen.right() - screen.width() * 0.07;
        let panel_bottom = screen.bottom() - screen.height() * 0.10;
        let panel_cx = (panel_left + panel_right) * 0.5;
        let panel_cy = (panel_top + panel_bottom) * 0.5;
        let panel_w = (panel_right - panel_left).max(80.0);
        let panel_h = (panel_bottom - panel_top).max(80.0);

        // Actual Rust geometry merger.
        let base = rectangle(panel_cx as f64, panel_cy as f64, panel_w as f64, panel_h as f64, 0.0);
        let tab = rectangle(
            self.tab_center.x as f64,
            self.tab_center.y as f64,
            self.tab_size.x as f64,
            self.tab_size.y as f64,
            self.angle as f64,
        );
        if let Ok((loops, stats)) = merged_rounded_loops(
            &[base, tab],
            self.convex as f64,
            self.concave as f64,
            0.12,
        ) {
            self.stats = stats;
            for loop_points in loops {
                let pts: Vec<Pos2> = loop_points.iter().map(|p| pos2(p.x as f32, p.y as f32)).collect();
                if let Some(mesh) = polygon_mesh(&pts, Color32::from_rgb(33, 39, 39)) {
                    painter.add(Shape::mesh(mesh));
                }
                painter.add(Shape::closed_line(pts, Stroke::new(1.6, Color32::from_rgb(50, 196, 232))));
            }
        }

        let graph_rect = Rect::from_min_max(
            pos2(panel_left + 44.0, panel_top + panel_h * 0.46),
            pos2(panel_right - 44.0, panel_top + panel_h * 0.69),
        );
        let mut wave = Vec::with_capacity(90);
        for i in 0..90 {
            let t = i as f32 / 89.0;
            let x = egui::lerp(graph_rect.x_range(), t);
            let y = graph_rect.center().y + (t * 10.0 * core::f32::consts::TAU).sin() * graph_rect.height() * 0.28;
            wave.push(pos2(x, y));
        }
        painter.add(Shape::line(wave, Stroke::new(1.0, Color32::from_rgba_premultiplied(112, 143, 142, 105))));

        // Egui interaction: drag the outer tab.
        let tab_corners: Vec<Pos2> = [
            vec2(-self.tab_size.x*0.5, -self.tab_size.y*0.5),
            vec2( self.tab_size.x*0.5, -self.tab_size.y*0.5),
            vec2( self.tab_size.x*0.5,  self.tab_size.y*0.5),
            vec2(-self.tab_size.x*0.5,  self.tab_size.y*0.5),
        ].into_iter().map(|p| self.local_to_world(p)).collect();
        let tab_bounds = bounds_of(&tab_corners).expand(2.0);
        let tab_response = ui.interact(tab_bounds, Id::new("outer-tab-drag"), Sense::drag());
        if tab_response.dragged() {
            let delta = ui.input(|i| i.pointer.delta());
            self.tab_center += delta;
        }

        // Rotation handle.
        let rot_handle = self.local_to_world(vec2(0.0, -self.tab_size.y * 0.5 - 34.0));
        let rot_rect = Rect::from_center_size(rot_handle, vec2(28.0, 28.0));
        let rot_response = ui.interact(rot_rect, Id::new("rotate-handle"), Sense::drag());
        painter.line_segment(
            [self.local_to_world(vec2(0.0, -self.tab_size.y*0.5)), rot_handle],
            Stroke::new(1.0, Color32::from_rgb(53, 174, 204)),
        );
        painter.circle_filled(rot_handle, 10.0, Color32::from_rgb(31, 42, 42));
        painter.circle_stroke(rot_handle, 10.0, Stroke::new(1.4, Color32::from_rgb(62, 198, 231)));
        painter.circle_stroke(rot_handle, 4.5, Stroke::new(1.0, Color32::from_rgb(121, 165, 170)));
        if rot_response.dragged() {
            if let Some(p) = ui.input(|i| i.pointer.interact_pos()) {
                self.angle = (p.y - self.tab_center.y).atan2(p.x - self.tab_center.x) + core::f32::consts::FRAC_PI_2;
            }
        }

        // Independent dark capsule inside the tab.
        let pill_w = self.tab_size.x - 26.0;
        let item_d = 32.0;
        let gap = 11.0;
        let n = self.pies.len() + 1;
        let desired_h = 22.0 + n as f32 * item_d + (n.saturating_sub(1)) as f32 * gap;
        let pill_h = desired_h.min(self.tab_size.y - 24.0).max(70.0);
        let pill = capsule_points(pill_w, pill_h, 18);
        let pill_world: Vec<Pos2> = pill.into_iter().map(|p| self.local_to_world(p)).collect();
        if let Some(mesh) = polygon_mesh(&pill_world, Color32::from_rgb(12, 18, 18)) {
            painter.add(Shape::mesh(mesh));
        }
        painter.add(Shape::closed_line(pill_world, Stroke::new(1.1, Color32::from_rgb(50, 123, 132))));

        let content_top = -pill_h * 0.5 + 21.0 + item_d * 0.5;
        let plus_center = self.local_to_world(vec2(0.0, content_top));
        let plus_resp = ui.interact(Rect::from_center_size(plus_center, vec2(34.0,34.0)), Id::new("add-pie"), Sense::click());
        draw_plus(&painter, plus_center, plus_resp.hovered());
        if plus_resp.clicked() && self.pies.len() < 4 { self.pies.push(0.5); }

        for i in 0..self.pies.len() {
            let y = content_top + (i as f32 + 1.0) * (item_d + gap);
            if y > pill_h * 0.5 - item_d * 0.4 { break; }
            let center = self.local_to_world(vec2(0.0, y));
            let response = ui.interact(Rect::from_center_size(center, vec2(38.0,38.0)), Id::new(("pie", i)), Sense::drag());
            if response.dragged() {
                let dy = ui.input(|input| input.pointer.delta().y);
                self.pies[i] = (self.pies[i] - dy * 0.007).clamp(0.0, 1.0);
            }
            draw_pie(&painter, center, 13.0, self.pies[i], response.hovered());
        }

        // Custom sliders, but real egui layout/hit-testing/painting.
        let controls_x = screen.right() - screen.width() * 0.25;
        let controls_w = screen.width() * 0.18;
        let slider_y = screen.top() + 58.0;
        custom_slider(ui, &painter, Id::new("convex-slider"), Rect::from_min_size(pos2(controls_x, slider_y), vec2(controls_w, 28.0)), &mut self.convex, 0.0, 64.0);
        custom_slider(ui, &painter, Id::new("concave-slider"), Rect::from_min_size(pos2(controls_x, slider_y + 52.0), vec2(controls_w, 28.0)), &mut self.concave, 0.0, 64.0);

        let mut deg = self.angle.to_degrees();
        custom_slider(ui, &painter, Id::new("rotation-slider"), Rect::from_min_size(pos2(controls_x, slider_y + 104.0), vec2(controls_w, 28.0)), &mut deg, -180.0, 180.0);
        self.angle = deg.to_radians();

        // Four icon-only egui buttons: left, center, separate, reset.
        let by = screen.bottom() - 44.0;
        let bx = screen.left() + 32.0;
        let bw = 44.0;
        for i in 0..4 {
            let r = Rect::from_min_size(pos2(bx + i as f32 * (bw + 10.0), by), vec2(bw, 30.0));
            let resp = ui.interact(r, Id::new(("preset", i)), Sense::click());
            painter.rect_filled(r, 7.0, if resp.hovered() { Color32::from_rgb(43,49,49) } else { Color32::from_rgb(30,35,35) });
            painter.rect_stroke(r, 7.0, Stroke::new(1.0, Color32::from_rgb(62,71,71)), egui::StrokeKind::Inside);
            draw_preset_icon(&painter, r, i);
            if resp.clicked() {
                match i {
                    0 => self.reset_left(screen),
                    1 => { self.tab_center.x = (panel_left + panel_right) * 0.5; self.tab_center.y = panel_top - self.tab_size.y*0.5 + 22.0; self.angle = 0.0; },
                    2 => { self.tab_center = pos2(panel_left - 105.0, panel_top - 110.0); self.angle = 0.0; },
                    _ => { self.reset_left(screen); self.convex=28.0; self.concave=32.0; self.pies=vec![0.62,0.27]; },
                }
            }
        }

        // Graphical proof badge: four cyan bars encode E-G-U-I.
        let proof = Rect::from_min_size(pos2(screen.left()+24.0, screen.top()+24.0), vec2(92.0, 24.0));
        painter.rect_filled(proof, 12.0, Color32::from_rgb(21,29,29));
        painter.rect_stroke(proof, 12.0, Stroke::new(1.0, Color32::from_rgb(45,108,115)), egui::StrokeKind::Inside);
        for i in 0..4 {
            let x = proof.left()+16.0+i as f32*18.0;
            let h = 5.0+i as f32*2.8;
            painter.rect_filled(Rect::from_center_size(pos2(x,proof.center().y),vec2(7.0,h)), 3.5, Color32::from_rgb(53,198,235));
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
                // This demo deliberately draws only untextured primitives.
                // The WHITE_UV vertices emitted by egui are safe to present as solid colors.
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

fn bounds_of(points: &[Pos2]) -> Rect {
    let mut r = Rect::NOTHING;
    for &p in points { r.extend_with(p); }
    r
}

fn polygon_area(points: &[Pos2]) -> f32 {
    let mut a=0.0;
    for i in 0..points.len() {
        let p=points[i]; let q=points[(i+1)%points.len()];
        a += p.x*q.y-q.x*p.y;
    }
    a*0.5
}

fn cross(a: Pos2,b: Pos2,c: Pos2)->f32 { (b.x-a.x)*(c.y-a.y)-(b.y-a.y)*(c.x-a.x) }
fn point_in_tri(p:Pos2,a:Pos2,b:Pos2,c:Pos2)->bool {
    let c1=cross(a,b,p); let c2=cross(b,c,p); let c3=cross(c,a,p);
    (c1>=-1e-4 && c2>=-1e-4 && c3>=-1e-4) || (c1<=1e-4 && c2<=1e-4 && c3<=1e-4)
}

fn polygon_mesh(points:&[Pos2], color:Color32)->Option<Mesh> {
    if points.len()<3 { return None; }
    let mut ids: Vec<usize> = (0..points.len()).collect();
    if polygon_area(points)<0.0 { ids.reverse(); }
    let mut tris=Vec::<u32>::new();
    let mut guard=0usize;
    while ids.len()>3 && guard<points.len()*points.len() {
        guard+=1;
        let n=ids.len();
        let mut found=false;
        for k in 0..n {
            let ia=ids[(k+n-1)%n]; let ib=ids[k]; let ic=ids[(k+1)%n];
            let a=points[ia]; let b=points[ib]; let c=points[ic];
            if cross(a,b,c)<=1e-5 { continue; }
            let mut contains=false;
            for &j in &ids {
                if j==ia || j==ib || j==ic { continue; }
                if point_in_tri(points[j],a,b,c) { contains=true; break; }
            }
            if !contains {
                tris.extend_from_slice(&[ia as u32,ib as u32,ic as u32]);
                ids.remove(k); found=true; break;
            }
        }
        if !found { break; }
    }
    if ids.len()==3 { tris.extend_from_slice(&[ids[0] as u32,ids[1] as u32,ids[2] as u32]); }
    if tris.is_empty() { return None; }
    let mut mesh=Mesh::default();
    mesh.vertices=points.iter().map(|&p| egui::epaint::Vertex::untextured(p,color)).collect();
    mesh.indices=tris;
    Some(mesh)
}

fn capsule_points(w:f32,h:f32,steps:usize)->Vec<Vec2>{
    let r=(w*0.5).min(h*0.5);
    let straight=(h*0.5-r).max(0.0);
    let mut out=Vec::with_capacity(steps*2+2);
    for i in 0..=steps {
        let a=core::f32::consts::PI + core::f32::consts::PI*(i as f32/steps as f32);
        out.push(vec2(a.cos()*r, -straight+a.sin()*r));
    }
    for i in 0..=steps {
        let a=core::f32::consts::PI*(i as f32/steps as f32);
        out.push(vec2(a.cos()*r, straight+a.sin()*r));
    }
    out
}

fn draw_plus(p:&egui::Painter,c:Pos2,hover:bool){
    let col=if hover { Color32::from_rgb(75,219,249) } else { Color32::from_rgb(48,188,226) };
    p.circle_filled(c,14.0,Color32::from_rgb(15,29,31));
    p.circle_stroke(c,14.0,Stroke::new(1.4,col));
    p.line_segment([c-vec2(6.0,0.0),c+vec2(6.0,0.0)],Stroke::new(1.5,col));
    p.line_segment([c-vec2(0.0,6.0),c+vec2(0.0,6.0)],Stroke::new(1.5,col));
}

fn draw_pie(p:&egui::Painter,c:Pos2,r:f32,value:f32,hover:bool){
    let rim=if hover { Color32::from_rgb(82,214,241) } else { Color32::from_rgb(47,173,208) };
    p.circle_filled(c,r,Color32::from_rgb(12,22,23));
    let n=((value*30.0).ceil() as usize).max(1);
    let mut pts=Vec::with_capacity(n+2); pts.push(c);
    let start=-core::f32::consts::FRAC_PI_2;
    let sweep=value*core::f32::consts::TAU;
    for i in 0..=n { let a=start+sweep*(i as f32/n as f32); pts.push(c+vec2(a.cos()*r,a.sin()*r)); }
    if let Some(m)=polygon_mesh(&pts,Color32::from_rgb(47,191,229)){ p.add(Shape::mesh(m)); }
    p.circle_stroke(c,r,Stroke::new(1.1,rim));
}

fn custom_slider(ui:&mut Ui,p:&egui::Painter,id:Id,rect:Rect,value:&mut f32,min:f32,max:f32){
    let response=ui.interact(rect,id,Sense::click_and_drag());
    if response.dragged() || response.clicked() {
        if let Some(pos)=ui.input(|i|i.pointer.interact_pos()) {
            *value = egui::remap_clamp(pos.x, rect.left()..=rect.right(), min..=max);
        }
    }
    let track=Rect::from_center_size(rect.center(),vec2(rect.width(),3.0));
    p.rect_filled(track,1.5,Color32::from_rgb(67,77,77));
    let t=egui::remap_clamp(*value,min..=max,0.0..=1.0);
    let x=egui::lerp(rect.x_range(),t);
    p.rect_filled(Rect::from_min_max(track.min,pos2(x,track.max.y)),1.5,Color32::from_rgb(47,189,226));
    p.circle_filled(pos2(x,rect.center().y), if response.hovered(){7.0}else{6.0}, Color32::from_rgb(53,198,235));
}

fn draw_preset_icon(p:&egui::Painter,r:Rect,i:usize){
    let c=r.center(); let col=Color32::from_rgb(132,154,154);
    match i {
        0=>{p.line_segment([c-vec2(9.0,7.0),c-vec2(9.0,-7.0)],Stroke::new(1.5,col));p.line_segment([c-vec2(5.0,0.0),c+vec2(9.0,0.0)],Stroke::new(1.5,col));},
        1=>{p.line_segment([c-vec2(9.0,0.0),c+vec2(9.0,0.0)],Stroke::new(1.5,col));p.line_segment([c-vec2(0.0,7.0),c+vec2(0.0,7.0)],Stroke::new(1.5,col));},
        2=>{p.circle_stroke(c-vec2(6.0,0.0),4.0,Stroke::new(1.2,col));p.circle_stroke(c+vec2(6.0,0.0),4.0,Stroke::new(1.2,col));},
        _=>{p.circle_stroke(c,7.0,Stroke::new(1.2,col));p.line_segment([c+vec2(4.0,-5.0),c+vec2(8.0,-5.0)],Stroke::new(1.2,col));}
    }
}

static mut APP_PTR: *mut App = ptr::null_mut();
unsafe fn app() -> &'static mut App {
    if APP_PTR.is_null() { APP_PTR=Box::into_raw(Box::new(App::default())); }
    &mut *APP_PTR
}

#[no_mangle] pub extern "C" fn magic()->u32 { MAGIC }
#[no_mangle] pub extern "C" fn egui_version_code()->u32 { 0x0036_0002 }
#[no_mangle] pub extern "C" fn vertex_stride()->u32 { VERTEX_STRIDE as u32 }

#[no_mangle]
pub extern "C" fn frame(width:f32,height:f32,time:f64)->u32 {
    unsafe { app().frame(width,height,time); app().indices.len() as u32 }
}

#[no_mangle]
pub extern "C" fn pointer_move(x:f32,y:f32){ unsafe { app().events.push(Event::PointerMoved(pos2(x,y))); } }

#[no_mangle]
pub extern "C" fn pointer_button(x:f32,y:f32,button:u32,pressed:u32){
    let button=match button {1=>PointerButton::Secondary,2=>PointerButton::Middle,_=>PointerButton::Primary};
    unsafe { app().events.push(Event::PointerButton{pos:pos2(x,y),button,pressed:pressed!=0,modifiers:Modifiers::NONE}); }
}

#[no_mangle]
pub extern "C" fn pointer_gone(){ unsafe { app().events.push(Event::PointerGone); } }

#[no_mangle] pub extern "C" fn vertices_ptr()->*const f32 { unsafe { app().vertices.as_ptr() } }
#[no_mangle] pub extern "C" fn vertices_len()->u32 { unsafe { app().vertices.len() as u32 } }
#[no_mangle] pub extern "C" fn indices_ptr()->*const u32 { unsafe { app().indices.as_ptr() } }
#[no_mangle] pub extern "C" fn indices_len()->u32 { unsafe { app().indices.len() as u32 } }
#[no_mangle] pub extern "C" fn concave_count()->u32 { unsafe { app().stats.concave } }
#[no_mangle] pub extern "C" fn limited_count()->u32 { unsafe { app().stats.limited } }
