//! Native bounded-draw adaptations of SoundVisualizer 2D modes.
//! Copyright (c) 2026 Çağan Turgut (CaYatur), CaYaDev. MIT.
//! Source revision c3e7fc6bc3fa826e323486256268fa764a317963.
//! See assets/mods/LICENSE-SoundVisualizer.txt. Canvas bloom/compositing is
//! omitted; geometry and audio mappings are adapted to Vibefy's mono audio tap.
use super::{draw, end, line, n, point, repaint};
use std::f32::consts::{PI, TAU};
fn band(i: usize, count: usize) -> f32 {
    let len = (n(18) as usize).clamp(1,128);
    (n(64 + (i * len / count.max(1)).min(len-1) as i32) * n(12)).clamp(0.,1.)
}
fn wave(i: usize) -> f32 { (n(3328+(i%256) as i32)*n(12)).clamp(-1.,1.) }
fn segment(x:f32,y:f32,xx:f32,yy:f32,width:f32,alpha:f32) {
    line(width,alpha);point(x,y);point(xx,yy);end();
}
fn dot(x:f32,y:f32,r:f32,alpha:f32) {draw(4,[x,y,r,alpha,0.,0.,0.,0.]);}
fn rect(x:f32,y:f32,w:f32,h:f32,alpha:f32) {draw(3,[x,y,w,h,2.,alpha,-1.,0.]);}
pub fn render(x:f32,y:f32,w:f32,h:f32) {
    let mode=n(13) as u8;
    let t=n(4); // Playback time freezes while paused, independent of UI repaints.
    let (cx,cy)=(x+w/2.,y+h/2.);
    let dim=w.min(h);
    let bass=(0..8).map(|i|band(i,64)).sum::<f32>()/8.;
    match mode {
        0|1 => { // bars.js / centerbars.js
            let count=64; let slot=w/count as f32;
            for i in 0..count {
                let k=if mode==1 {(((i as f32-31.5).abs()/32.)*63.).round() as usize} else {i};
                let bh=(band(k,count)*h*if mode==1 {0.44} else {0.80}).max(1.);
                rect(x+i as f32*slot+slot*0.1,if mode==1 {cy-bh} else {y+h*0.9-bh},slot*0.8,if mode==1 {bh*2.} else {bh},0.9);
            }
        }
        2|7 => { // circular.js / starburst.js
            let count=64;
            let base=dim*if mode==2 {0.22*(1.+bass*0.28)} else {0.06};
            for i in 0..count {
                let a=i as f32/count as f32*TAU+t*0.1;
                let v=band(i,count);
                let len=v*if mode==2 {dim*0.22} else {(w*w+h*h).sqrt()*0.46*(1.+bass*0.35)};
                // A few tapered sections approximate the original radial mask.
                for j in 0..4 {
                    let r0=base+len*j as f32/4.;let r1=base+len*(j+1) as f32/4.;
                    segment(cx+a.cos()*r0,cy+a.sin()*r0,cx+a.cos()*r1,cy+a.sin()*r1,if mode==2 {3.} else {dim*0.006*(0.35+v)},1.-j as f32*0.2);
                }
            }
        }
        3 => { // wave.js: actual PCM, not spectrum magnitudes.
            line(2.,0.95);for i in 0..256 {point(x+w*i as f32/255.,cy+wave(i)*h*0.3528);}end();
        }
        4 => { // radialwave.js
            let radius=dim*0.22*(1.+bass*0.28);
            line(2.,0.95);for i in 0..=256 {let a=(i%256) as f32/256.*TAU+t*0.12;let r=radius+wave(i)*dim*0.1092;point(cx+a.cos()*r,cy+a.sin()*r);}end();
        }
        5|6 => { // dots.js / blocks.js: bounded 48 x 24 grid.
            let cols=48;let rows=(h/30.).round().clamp(5.,24.) as usize;
            let sx=w/cols as f32;let sy=h*0.84/rows as f32;
            for i in 0..cols {let lit=band(i,cols)*rows as f32;
                for row in 0..rows {let alpha=if (row as f32)<lit {0.9} else {0.055};let px=x+(i as f32+0.5)*sx;let py=y+h*0.92-(row as f32+0.5)*sy;
                    if mode==5 {dot(px,py,sx.min(sy)*0.34,alpha);} else {rect(px-sx*0.4,py-sy*0.38,sx*0.8,sy*0.76,alpha);}
                }
            }
        }
        8 => { // extras.js Helix
            for side in 0..2 {line(2.,0.85);for i in 0..96 {let f=i as f32/95.;let amp=h*0.276*(0.35+band(i,96)*0.9);point(x+f*w,cy+(f*TAU*2.4+t*1.1+side as f32*PI).sin()*amp);}end();}
            for i in (0..96).step_by(2) {let f=i as f32/95.;let v=band(i,96);let dy=(f*TAU*2.4+t*1.1).sin()*h*0.276*(0.35+v*0.9);segment(x+f*w,cy-dy,x+f*w,cy+dy,1.,0.25+v*0.6);}
        }
        9 => { // extras.js Strings
            let count=18;let slot=h/(count+1) as f32;
            for i in 0..count {let v=band(i,count);line(1.+v,0.35+v*0.65);for j in 0..=90 {let f=j as f32/90.;let dy=(f*PI*(1+i%3) as f32*2.+t*(7.+i as f32*1.7)).sin()*(f*PI).sin()*slot*0.4232*v;point(x+f*w,y+slot*(i+1) as f32+dy);}end();}
        }
        10 => { // extras.js Lissajous (mono phase-space, not stereo balance).
            let r=dim*0.3864;let a=t*0.15;line(1.7,0.9);
            for i in 0..256 {let u=wave(i);let v=wave(i+15);point(cx+(u*a.cos()-v*a.sin())*r,cy+(u*a.sin()+v*a.cos())*r);}end();
        }
        _ => { // extras.js Arcs
            let count=24;let r=dim*0.46;let inner=r*0.12;let step=(r-inner)/count as f32;
            for i in 0..count {let v=band(i,count);let radius=inner+step*(i as f32+0.5);let sweep=TAU*(0.06+v*0.9);let start=t*(0.3+i as f32*0.045)*if i%2==0 {1.} else {-1.}-sweep/2.;line((step*0.7).max(1.),0.28+v*0.72);for j in 0..=48 {let a=start+sweep*j as f32/48.;point(cx+a.cos()*radius,cy+a.sin()*radius);}end();}
        }
    }
    if n(6)>0. {repaint(n(26));}
}
