//! Vibefy mod ABI 1. Compiled separately for each catalogue entry; never linked
//! into the desktop executable. The host exposes numbers and bounded draw commands.
#[link(wasm_import_module = "vibefy")]
unsafe extern "C" {
    fn number(index: i32) -> f32;
    fn emit(kind: i32, a: f32, b: f32, c: f32, d: f32, e: f32, f: f32, g: f32, h: f32);
}
fn n(i: i32) -> f32 { unsafe { number(i) } }
fn draw(k:i32, v:[f32;8]) { unsafe { emit(k,v[0],v[1],v[2],v[3],v[4],v[5],v[6],v[7]) } }
fn sprite(asset:f32, frame:f32, x:f32,y:f32,w:f32,h:f32) { draw(5,[x,y,w,h,frame,asset,0.,0.]); }
fn repaint(fps:f32) { draw(8,[fps,0.,0.,0.,0.,0.,0.,0.]); }
#[unsafe(no_mangle)] pub extern "C" fn abi() -> i32 { 1 }
#[unsafe(no_mangle)] pub extern "C" fn identity() -> i32 {
    if cfg!(nyan) {0} else if cfg!(wavelink) {1} else if cfg!(juicevault) {2}
    else if cfg!(oneko) {3} else if cfg!(catjam) {4}
    else {8}
}
/// Native service adapters remain in the host so credentials are never shared
/// with mods. Provider modules register the corresponding optional entry points.
#[unsafe(no_mangle)] pub extern "C" fn capabilities() -> i32 {
    if cfg!(wavelink) {1} else if cfg!(juicevault) {2} else {0}
}
#[unsafe(no_mangle)] pub extern "C" fn render() {
    let (x,y,w,h)=(n(0),n(1),n(2),n(3));
    if w < 1. || h < 1. { return; }
    if cfg!(nyan) {
        let tip=x+w*n(11).clamp(0.,1.);
        for i in 0..6 {draw(3,[x,y+h/2.-3.+i as f32,(tip-x).max(0.),1.,0.,1.,i as f32,0.]);}
        let frame=(n(4)*n(7)/60.*6.).floor().rem_euclid(6.);
        let center=if w>=34. {tip.clamp(x+17.,x+w-17.)} else {x+w/2.};
        sprite(0.,frame,center-17.,y+h/2.-10.5,34.,21.);
        if n(6)>0. {repaint(30.);}
    } else if cfg!(catjam) {
        let scale=n(10).clamp(0.5,2.);
        let frame=(n(4)*n(7)/135.48/9.75*325.).floor().rem_euclid(325.);
        sprite(1.,frame,x,y+h-n(25)-86.*scale,112.*scale,86.*scale);
        if n(6)>0. {repaint(30.);}
    } else if cfg!(oneko) { neko(x,y,w,h); }
    else if cfg!(trackpeek) {
        let width=((w-n(23))/2.-10.).clamp(0.,180.);
        if width<28. {return;}
        for next in 0..2 {let left=if next==1 {x+w/2.+n(23)/2.+8.} else {x+w/2.-n(23)/2.-8.-width};draw(6,[left,n(24)-21.,width,42.,next as f32,0.,0.,0.]);}
    }
}
static mut CAT: [f32;4]=[48.,64.,0.,0.];
fn neko(x:f32,y:f32,w:f32,h:f32) {
    let [mut px,mut py,last,mut idle]=unsafe {CAT};let time=n(5);let dx=n(8)-px;let dy=n(9)-py;let distance=(dx*dx+dy*dy).sqrt();let moving=distance>24.;let frame=(time*8.) as i32%2;
    let (col,row)=if moving {let step=((time-last).clamp(0.,0.1)*130.).min(distance-20.);px+=dx/distance*step;py+=dy/distance*step;idle=time;
        if dx.abs()>dy.abs()*2. {if dx>0. {(3,frame)} else {(4,2+frame)}} else if dy.abs()>dx.abs()*2. {if dy>0. {if frame==0 {(6,3)} else {(7,2)}} else {(1,2+frame)}} else if dx>0. {if dy>0. {if frame==0 {(5,1)} else {(5,2)}} else {(0,2+frame)}} else if dy>0. {if frame==0 {(5,3)} else {(6,1)}} else {(1,frame)}
    } else if time-idle>4. {(2,0)} else {(3,3)};
    px=px.clamp(x+16.,(x+w-16.).max(x+16.));py=py.clamp(y+16.,(y+h-16.).max(y+16.));unsafe {CAT=[px,py,time,idle]};
    sprite(2.,(row*8+col) as f32,px-16.,py-16.,32.,32.);
    if moving {repaint(20.);} else if time-idle<4.1 {repaint(1.);}
}
