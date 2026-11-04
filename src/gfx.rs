//! Thin immediate-mode drawing bridge to the browser's Canvas2D context.
//!
//! All rendering calls are batched by the browser. Keeping the bridge tiny
//! makes it easy to run the same simulation natively in unit tests.

use crate::math::V2;

#[inline]
pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
    ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (a as u32)
}

#[inline]
pub const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    rgba(r, g, b, 255)
}

#[inline]
pub fn alpha(color: u32, a: u8) -> u32 {
    (color & 0xFFFF_FF00) | (a as u32)
}

#[inline]
pub fn mix(a: u32, b: u32, t: f32) -> u32 {
    let t = crate::math::clamp(t, 0.0, 1.0);
    let ar = (a >> 24) as f32;
    let ag = ((a >> 16) & 255) as f32;
    let ab = ((a >> 8) & 255) as f32;
    let aa = (a & 255) as f32;
    let br = (b >> 24) as f32;
    let bg = ((b >> 16) & 255) as f32;
    let bb = ((b >> 8) & 255) as f32;
    let ba = (b & 255) as f32;
    rgba(
        (ar + (br - ar) * t) as u8,
        (ag + (bg - ag) * t) as u8,
        (ab + (bb - ab) * t) as u8,
        (aa + (ba - aa) * t) as u8,
    )
}

#[inline]
pub fn with_light(color: u32, light: f32) -> u32 {
    let l = 0.35 + 0.65 * light;
    let r = (((color >> 24) & 255) as f32 * l) as u8;
    let g = (((color >> 16) & 255) as f32 * l) as u8;
    let b = (((color >> 8) & 255) as f32 * l) as u8;
    let a = (color & 255) as u8;
    rgba(r, g, b, a)
}

#[cfg(target_arch = "wasm32")]
mod sys {
    #[link(wasm_import_module = "env")]
    extern "C" {
        pub fn js_rect(x: f32, y: f32, w: f32, h: f32, color: u32);
        pub fn js_rect_rot(cx: f32, cy: f32, w: f32, h: f32, angle: f32, color: u32);
        pub fn js_circle(x: f32, y: f32, r: f32, color: u32);
        pub fn js_ellipse(x: f32, y: f32, rx: f32, ry: f32, color: u32);
        pub fn js_ring(x: f32, y: f32, r: f32, width: f32, color: u32);
        pub fn js_line(x1: f32, y1: f32, x2: f32, y2: f32, width: f32, color: u32);
        pub fn js_tri(x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, color: u32);
        pub fn js_poly(ptr: *const f32, count: u32, color: u32);
        pub fn js_text(
            x: f32,
            y: f32,
            size: f32,
            color: u32,
            align: u32,
            weight: u32,
            ptr: *const u8,
            len: u32,
        );
        pub fn js_text_width(ptr: *const u8, len: u32, size: f32, weight: u32) -> f32;
        pub fn js_gradient(x: f32, y: f32, w: f32, h: f32, c1: u32, c2: u32, vertical: u32);
        pub fn js_glow(x: f32, y: f32, r: f32, color: u32);
        pub fn js_minimap(ptr: *const u8, w: u32, h: u32);
        pub fn js_minimap_draw(x: f32, y: f32, w: f32, h: f32, alpha: f32);
        pub fn js_vignette(strength: f32);
        pub fn js_sfx(id: u32, volume: f32);
        pub fn js_music(mode: u32);
        pub fn js_save(ptr: *const u8, len: u32);
        pub fn js_load_len() -> u32;
        pub fn js_load_read(ptr: *mut u8, len: u32) -> u32;
        pub fn js_show_title();
        pub fn js_mute(muted: u32);
        pub fn js_log(ptr: *const u8, len: u32);
    }
}

#[cfg(target_arch = "wasm32")]
#[inline]
pub fn rect(x: f32, y: f32, w: f32, h: f32, color: u32) {
    unsafe { sys::js_rect(x, y, w, h, color) }
}

#[cfg(target_arch = "wasm32")]
#[inline]
pub fn rect_rot(cx: f32, cy: f32, w: f32, h: f32, angle: f32, color: u32) {
    unsafe { sys::js_rect_rot(cx, cy, w, h, angle, color) }
}

#[cfg(target_arch = "wasm32")]
#[inline]
pub fn circle(x: f32, y: f32, r: f32, color: u32) {
    unsafe { sys::js_circle(x, y, r, color) }
}

#[cfg(target_arch = "wasm32")]
#[inline]
pub fn ellipse(x: f32, y: f32, rx: f32, ry: f32, color: u32) {
    unsafe { sys::js_ellipse(x, y, rx, ry, color) }
}

#[cfg(target_arch = "wasm32")]
#[inline]
pub fn ring(x: f32, y: f32, r: f32, width: f32, color: u32) {
    unsafe { sys::js_ring(x, y, r, width, color) }
}

#[cfg(target_arch = "wasm32")]
#[inline]
pub fn line(x1: f32, y1: f32, x2: f32, y2: f32, width: f32, color: u32) {
    unsafe { sys::js_line(x1, y1, x2, y2, width, color) }
}

#[cfg(target_arch = "wasm32")]
#[inline]
pub fn tri(x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, color: u32) {
    unsafe { sys::js_tri(x1, y1, x2, y2, x3, y3, color) }
}

#[cfg(target_arch = "wasm32")]
pub fn poly(points: &[V2], color: u32) {
    if points.is_empty() {
        return;
    }
    let flat: Vec<f32> = points.iter().flat_map(|p| [p.x, p.y]).collect();
    unsafe {
        sys::js_poly(flat.as_ptr(), points.len() as u32, color);
    }
}

#[cfg(target_arch = "wasm32")]
pub fn text(s: &str, x: f32, y: f32, size: f32, color: u32, align: u32, weight: u32) {
    unsafe {
        sys::js_text(
            x,
            y,
            size,
            color,
            align,
            weight,
            s.as_ptr(),
            s.len() as u32,
        );
    }
}

#[cfg(target_arch = "wasm32")]
pub fn text_width(s: &str, size: f32, weight: u32) -> f32 {
    unsafe { sys::js_text_width(s.as_ptr(), s.len() as u32, size, weight) }
}

#[cfg(target_arch = "wasm32")]
#[inline]
pub fn gradient(x: f32, y: f32, w: f32, h: f32, c1: u32, c2: u32, vertical: bool) {
    unsafe { sys::js_gradient(x, y, w, h, c1, c2, vertical as u32) }
}

#[cfg(target_arch = "wasm32")]
#[inline]
pub fn glow(x: f32, y: f32, r: f32, color: u32) {
    unsafe { sys::js_glow(x, y, r, color) }
}

#[cfg(target_arch = "wasm32")]
#[inline]
pub fn minimap(ptr: *const u8, w: u32, h: u32) {
    unsafe { sys::js_minimap(ptr, w, h) }
}

#[cfg(target_arch = "wasm32")]
#[inline]
pub fn minimap_draw(x: f32, y: f32, w: f32, h: f32, alpha: f32) {
    unsafe { sys::js_minimap_draw(x, y, w, h, alpha) }
}

#[cfg(target_arch = "wasm32")]
#[inline]
pub fn vignette(strength: f32) {
    unsafe { sys::js_vignette(strength) }
}

#[cfg(target_arch = "wasm32")]
#[inline]
pub fn sfx(id: u32, volume: f32) {
    unsafe { sys::js_sfx(id, volume) }
}

#[cfg(target_arch = "wasm32")]
#[inline]
pub fn music(mode: u32) {
    unsafe { sys::js_music(mode) }
}

#[cfg(target_arch = "wasm32")]
#[inline]
pub fn save(data: &[u8]) {
    unsafe { sys::js_save(data.as_ptr(), data.len() as u32) }
}

#[cfg(target_arch = "wasm32")]
#[inline]
pub fn load_len() -> u32 {
    unsafe { sys::js_load_len() }
}

#[cfg(target_arch = "wasm32")]
#[inline]
pub fn load_read(buf: &mut [u8]) -> u32 {
    unsafe { sys::js_load_read(buf.as_mut_ptr(), buf.len() as u32) }
}

#[cfg(target_arch = "wasm32")]
#[inline]
pub fn show_title() {
    unsafe { sys::js_show_title() }
}

#[cfg(target_arch = "wasm32")]
#[inline]
pub fn set_mute(muted: bool) {
    unsafe { sys::js_mute(muted as u32) }
}

#[cfg(target_arch = "wasm32")]
#[inline]
pub fn log(s: &str) {
    unsafe { sys::js_log(s.as_ptr(), s.len() as u32) }
}

// ---------------------------------------------------------------------------
// Native fallbacks: the simulation still runs headless for tests.
// ---------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::*;

    pub fn rect(_: f32, _: f32, _: f32, _: f32, _: u32) {}
    pub fn rect_rot(_: f32, _: f32, _: f32, _: f32, _: f32, _: u32) {}
    pub fn circle(_: f32, _: f32, _: f32, _: u32) {}
    pub fn ellipse(_: f32, _: f32, _: f32, _: f32, _: u32) {}
    pub fn ring(_: f32, _: f32, _: f32, _: f32, _: u32) {}
    pub fn line(_: f32, _: f32, _: f32, _: f32, _: f32, _: u32) {}
    pub fn tri(_: f32, _: f32, _: f32, _: f32, _: f32, _: f32, _: u32) {}
    pub fn poly(_: &[V2], _: u32) {}
    pub fn text(_: &str, _: f32, _: f32, _: f32, _: u32, _: u32, _: u32) {}
    pub fn text_width(s: &str, size: f32, _: u32) -> f32 {
        s.len() as f32 * size * 0.56
    }
    pub fn gradient(_: f32, _: f32, _: f32, _: f32, _: u32, _: u32, _: bool) {}
    pub fn glow(_: f32, _: f32, _: f32, _: u32) {}
    pub fn minimap(_: *const u8, _: u32, _: u32) {}
    pub fn minimap_draw(_: f32, _: f32, _: f32, _: f32, _: f32) {}
    pub fn vignette(_: f32) {}
    pub fn sfx(_: u32, _: f32) {}
    pub fn music(_: u32) {}
    pub fn save(_: &[u8]) {}
    pub fn load_len() -> u32 {
        0
    }
    pub fn load_read(_: &mut [u8]) -> u32 {
        0
    }
    pub fn show_title() {}
    pub fn set_mute(_: bool) {}
    pub fn log(_: &str) {}
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::*;
