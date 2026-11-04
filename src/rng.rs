/// Deterministic RNG and hashing so worlds can be reproduced from a seed.
#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u32) -> Self {
        let mut s = 0x9E37_79B9_7F4A_7C15u64 ^ (seed as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        if s == 0 {
            s = 0xDEAD_BEEF_CAFE_F00D;
        }
        Rng { state: s }
    }

    pub fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    pub fn f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / 16_777_216.0
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f32()
    }

    pub fn range_i(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u32() % ((hi - lo + 1) as u32)) as i32
    }

    pub fn chance(&mut self, p: f32) -> bool {
        self.f32() < p
    }

    pub fn sign(&mut self) -> f32 {
        if self.next_u32() & 1 == 0 {
            -1.0
        } else {
            1.0
        }
    }
}

/// Stable 2D hash used for per-tile decoration. Returns a value in [0, 1).
pub fn hash2(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x27D4_EB2D)
        ^ (y as u32).wrapping_mul(0x1656_67B1)
        ^ seed.wrapping_mul(0x9E37_79B9);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    h = h.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 16;
    (h >> 8) as f32 / 16_777_216.0
}

pub fn hash_i(x: i32, y: i32, seed: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x27D4_EB2D)
        ^ (y as u32).wrapping_mul(0x1656_67B1)
        ^ seed.wrapping_mul(0x9E37_79B9);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    h = h.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 16;
    h
}

/// Cheap 1D value noise with smooth interpolation, used for terrain.
pub fn value_noise(x: f32, seed: u32) -> f32 {
    let i = x.floor() as i32;
    let f = x - x.floor();
    let a = hash2(i, 0, seed);
    let b = hash2(i + 1, 0, seed);
    let t = f * f * (3.0 - 2.0 * f);
    a + (b - a) * t
}

pub fn fbm(x: f32, seed: u32, octaves: u32) -> f32 {
    let mut amp = 0.5;
    let mut freq = 1.0;
    let mut sum = 0.0;
    let mut norm = 0.0;
    for i in 0..octaves {
        sum += value_noise(x * freq, seed.wrapping_add(i * 7919)) * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.03;
    }
    sum / norm
}
