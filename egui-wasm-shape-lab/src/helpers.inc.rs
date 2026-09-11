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

fn rounded_rect_points(w:f32,h:f32,radius:f32,steps:usize)->Vec<Vec2>{
    let r=radius.clamp(0.0,(w*0.5).min(h*0.5));
    if r <= 0.001 {
        return vec![
            vec2(-w*0.5,-h*0.5), vec2(w*0.5,-h*0.5),
            vec2(w*0.5,h*0.5), vec2(-w*0.5,h*0.5),
        ];
    }
    let centers=[
        vec2(-w*0.5+r,-h*0.5+r),
        vec2( w*0.5-r,-h*0.5+r),
        vec2( w*0.5-r, h*0.5-r),
        vec2(-w*0.5+r, h*0.5-r),
    ];
    let starts=[core::f32::consts::PI, -core::f32::consts::FRAC_PI_2, 0.0, core::f32::consts::FRAC_PI_2];
    let mut out=Vec::with_capacity((steps+1)*4);
    for corner in 0..4 {
        let start=starts[corner];
        for i in 0..=steps {
            let a=start+core::f32::consts::FRAC_PI_2*(i as f32/steps.max(1) as f32);
            out.push(centers[corner]+vec2(a.cos()*r,a.sin()*r));
        }
    }
    out
}

fn draw_plus(p:&egui::Painter,c:Pos2,r:f32,hover:bool,scale:f32,accent:Color32){
    let col=if hover { Color32::from_rgb(75,219,249) } else { accent };
    p.circle_filled(c,r,Color32::from_rgb(25,37,34));
    p.circle_stroke(c,r,Stroke::new(1.8*scale,col));
    let arm=r*0.39;
    p.line_segment([c-vec2(arm,0.0),c+vec2(arm,0.0)],Stroke::new(1.8*scale,col));
    p.line_segment([c-vec2(0.0,arm),c+vec2(0.0,arm)],Stroke::new(1.8*scale,col));
}

fn draw_pie(p:&egui::Painter,c:Pos2,r:f32,value:f32,hover:bool,scale:f32,accent:Color32){
    let rim=if hover { Color32::from_rgb(82,214,241) } else { accent };
    p.circle_filled(c,r,Color32::from_rgb(40,57,55));
    let n=((value*32.0).ceil() as usize).max(1);
    let mut pts=Vec::with_capacity(n+2); pts.push(c);
    let start=-core::f32::consts::FRAC_PI_2;
    let sweep=value*core::f32::consts::TAU;
    for i in 0..=n { let a=start+sweep*(i as f32/n as f32); pts.push(c+vec2(a.cos()*r,a.sin()*r)); }
    if let Some(m)=polygon_mesh(&pts,accent){ p.add(Shape::mesh(m)); }
    p.circle_stroke(c,r,Stroke::new(1.7*scale,rim));
}

fn custom_slider(ui:&mut Ui,p:&egui::Painter,id:Id,rect:Rect,value:&mut f32,min:f32,max:f32,scale:f32){
    let response=ui.interact(rect,id,Sense::click_and_drag());
    if response.dragged() || response.clicked() {
        if let Some(pos)=ui.input(|i|i.pointer.interact_pos()) {
            *value = egui::remap_clamp(pos.x, rect.left()..=rect.right(), min..=max);
        }
    }
    let track=Rect::from_center_size(rect.center(),vec2(rect.width(),3.0*scale));
    p.rect_filled(track,1.5*scale,Color32::from_rgb(67,77,77));
    let t=egui::remap_clamp(*value,min..=max,0.0..=1.0);
    let x=egui::lerp(rect.x_range(),t);
    p.rect_filled(Rect::from_min_max(track.min,pos2(x,track.max.y)),1.5*scale,Color32::from_rgb(47,189,226));
    p.circle_filled(pos2(x,rect.center().y), if response.hovered(){7.0*scale}else{6.0*scale}, Color32::from_rgb(53,198,235));
}

fn draw_preset_icon(p:&egui::Painter,r:Rect,i:usize,scale:f32){
    let c=r.center(); let col=Color32::from_rgb(132,154,154);
    let v=|x:f32,y:f32|vec2(x*scale,y*scale);
    match i {
        0=>{p.line_segment([c-v(9.0,7.0),c-v(9.0,-7.0)],Stroke::new(1.5*scale,col));p.line_segment([c-v(5.0,0.0),c+v(9.0,0.0)],Stroke::new(1.5*scale,col));},
        1=>{p.line_segment([c-v(9.0,0.0),c+v(9.0,0.0)],Stroke::new(1.5*scale,col));p.line_segment([c-v(0.0,7.0),c+v(0.0,7.0)],Stroke::new(1.5*scale,col));},
        2=>{p.circle_stroke(c-v(6.0,0.0),4.0*scale,Stroke::new(1.2*scale,col));p.circle_stroke(c+v(6.0,0.0),4.0*scale,Stroke::new(1.2*scale,col));},
        _=>{p.circle_stroke(c,7.0*scale,Stroke::new(1.2*scale,col));p.line_segment([c+v(4.0,-5.0),c+v(8.0,-5.0)],Stroke::new(1.2*scale,col));}
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
