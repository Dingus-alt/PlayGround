/// Small vector math helpers used across the game.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V2 {
    pub x: f32,
    pub y: f32,
}

impl V2 {
    pub const ZERO: V2 = V2 { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        V2 { x, y }
    }

    pub fn add(self, other: V2) -> V2 {
        V2::new(self.x + other.x, self.y + other.y)
    }

    pub fn sub(self, other: V2) -> V2 {
        V2::new(self.x - other.x, self.y - other.y)
    }

    pub fn scale(self, s: f32) -> V2 {
        V2::new(self.x * s, self.y * s)
    }

    pub fn len(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn norm(self) -> V2 {
        let l = self.len();
        if l > 0.0001 {
            self.scale(1.0 / l)
        } else {
            V2::ZERO
        }
    }

    pub fn lerp(self, other: V2, t: f32) -> V2 {
        V2::new(self.x + (other.x - self.x) * t, self.y + (other.y - self.y) * t)
    }
}

pub fn clamp(v: f32, lo: f32, hi: f32) -> f32 {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}

pub fn clamp_i(v: i32, lo: i32, hi: i32) -> i32 {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn smoothstep(t: f32) -> f32 {
    let t = clamp(t, 0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn approach(current: f32, target: f32, delta: f32) -> f32 {
    if current < target {
        (current + delta).min(target)
    } else {
        (current - delta).max(target)
    }
}

pub fn dist(a: V2, b: V2) -> f32 {
    (a.x - b.x).hypot(a.y - b.y)
}

pub fn aabb_overlap(ax: f32, ay: f32, aw: f32, ah: f32, bx: f32, by: f32, bw: f32, bh: f32) -> bool {
    ax < bx + bw && ax + aw > bx && ay < by + bh && ay + ah > by
}

pub fn ease_out(t: f32) -> f32 {
    let t = clamp(t, 0.0, 1.0);
    1.0 - (1.0 - t) * (1.0 - t)
}
