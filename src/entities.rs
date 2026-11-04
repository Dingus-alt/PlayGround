//! Enemies, item drops, bombs, particles and floating combat text.

use crate::items::ItemId;
use crate::math::{clamp, dist, V2};
use crate::physics::{move_box, Contact};
use crate::rng::hash2;
use crate::world::World;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnemyKind {
    Slime,
    Zombie,
    Bat,
    KingSlime,
}

impl EnemyKind {
    pub fn name(self) -> &'static str {
        match self {
            EnemyKind::Slime => "Green Slime",
            EnemyKind::Zombie => "Zombie",
            EnemyKind::Bat => "Cave Bat",
            EnemyKind::KingSlime => "The Slime King",
        }
    }

    pub fn size(self) -> (f32, f32) {
        match self {
            EnemyKind::Slime => (18.0, 14.0),
            EnemyKind::Zombie => (12.0, 28.0),
            EnemyKind::Bat => (16.0, 10.0),
            EnemyKind::KingSlime => (52.0, 44.0),
        }
    }

    pub fn max_hp(self) -> f32 {
        match self {
            EnemyKind::Slime => 30.0,
            EnemyKind::Zombie => 48.0,
            EnemyKind::Bat => 20.0,
            EnemyKind::KingSlime => 420.0,
        }
    }

    pub fn contact_damage(self) -> f32 {
        match self {
            EnemyKind::Slime => 12.0,
            EnemyKind::Zombie => 18.0,
            EnemyKind::Bat => 13.0,
            EnemyKind::KingSlime => 20.0,
        }
    }

    pub fn is_boss(self) -> bool {
        self == EnemyKind::KingSlime
    }
}

#[derive(Clone, Debug)]
pub struct Enemy {
    pub kind: EnemyKind,
    pub pos: V2,
    pub vel: V2,
    pub hp: f32,
    pub max_hp: f32,
    pub w: f32,
    pub h: f32,
    pub on_ground: bool,
    pub facing: f32,
    pub timer: f32,
    pub anim: f32,
    pub hurt_flash: f32,
    pub dead: bool,
    pub death_timer: f32,
    pub spawned: f32,
    pub minion_wave: u8,
    pub contact_cd: f32,
}

impl Enemy {
    pub fn new(kind: EnemyKind, pos: V2) -> Self {
        let (w, h) = kind.size();
        Enemy {
            kind,
            pos,
            vel: V2::ZERO,
            hp: kind.max_hp(),
            max_hp: kind.max_hp(),
            w,
            h,
            on_ground: false,
            facing: 1.0,
            timer: 0.6,
            anim: 0.0,
            hurt_flash: 0.0,
            dead: false,
            death_timer: 0.0,
            spawned: 0.0,
            minion_wave: 0,
            contact_cd: 0.0,
        }
    }

    pub fn center(&self) -> V2 {
        V2::new(self.pos.x + self.w * 0.5, self.pos.y + self.h * 0.5)
    }

    pub fn aabb(&self) -> (f32, f32, f32, f32) {
        (self.pos.x, self.pos.y, self.w, self.h)
    }

    pub fn hurt(&mut self, amount: f32, knock: f32, from_x: f32) -> bool {
        if self.dead {
            return false;
        }
        self.hp -= amount;
        self.hurt_flash = 0.18;
        let dir = if self.center().x < from_x { -1.0 } else { 1.0 };
        if !self.kind.is_boss() {
            self.vel.x = dir * knock;
            self.vel.y = self.vel.y.min(-90.0);
        }
        if self.hp <= 0.0 {
            self.hp = 0.0;
            self.dead = true;
            self.death_timer = 0.0;
            return true;
        }
        false
    }

    pub fn update(&mut self, world: &World, player: V2, dt: f32, aggression: bool) {
        self.spawned = (self.spawned + dt).min(1.0);
        self.hurt_flash = (self.hurt_flash - dt).max(0.0);
        self.timer -= dt;
        self.anim += dt;
        self.contact_cd = (self.contact_cd - dt).max(0.0);
        if self.dead {
            self.death_timer += dt;
            return;
        }
        let dx = player.x - self.center().x;
        let dy = player.y - self.center().y;
        let d = dx.hypot(dy);
        if dx.abs() > 2.0 {
            self.facing = dx.signum();
        }

        match self.kind {
            EnemyKind::Slime => {
                self.vel.y += 1200.0 * dt;
                self.vel.y = self.vel.y.min(700.0);
                if self.timer <= 0.0 && self.on_ground {
                    if d < 560.0 && aggression {
                        self.vel.x = self.facing * (70.0 + (self.anim * 13.0).sin().abs() * 30.0);
                        self.vel.y = -255.0;
                    } else {
                        self.vel.x = self.facing * 35.0;
                        self.vel.y = -140.0;
                    }
                    self.timer = 1.0 + (self.anim * 7.0).sin().abs() * 0.5;
                }
                let c = move_box(world, &mut self.pos, &mut self.vel, self.w, self.h, dt, true);
                self.on_ground = c.ground;
                if c.ground {
                    self.vel.x *= 0.82;
                }
                if self.on_ground && self.vel.x.abs() > 4.0 {
                    self.vel.x *= 0.9;
                }
            }
            EnemyKind::Zombie => {
                self.vel.y += 1350.0 * dt;
                self.vel.y = self.vel.y.min(720.0);
                let speed = 58.0;
                if d < 700.0 && aggression {
                    if self.on_ground {
                        if self.vel.x.abs() < speed {
                            self.vel.x += self.facing * 520.0 * dt;
                        }
                    } else {
                        self.vel.x += self.facing * 80.0 * dt;
                    }
                } else {
                    self.vel.x *= 1.0 - 2.0 * dt;
                }
                self.vel.x = clamp(self.vel.x, -speed, speed);
                let c = move_box(world, &mut self.pos, &mut self.vel, self.w, self.h, dt, true);
                self.on_ground = c.ground;
                if c.wall && self.on_ground {
                    self.vel.y = -320.0;
                }
                if self.on_ground {
                    self.vel.x *= 0.97;
                }
            }
            EnemyKind::Bat => {
                let target = V2::new(player.x, player.y - 20.0);
                let dir = target.sub(self.center()).norm();
                let wobble = V2::new((self.anim * 2.3).sin(), (self.anim * 3.1).cos()).scale(28.0);
                let desired = dir.scale(72.0).add(wobble);
                self.vel = self.vel.lerp(desired, (dt * 2.2).min(1.0));
                let c = move_box(world, &mut self.pos, &mut self.vel, self.w, self.h, dt, false);
                if c.wall {
                    self.vel.x = -self.vel.x * 0.7;
                }
                if c.ground {
                    self.vel.y = -70.0;
                }
            }
            EnemyKind::KingSlime => {
                self.vel.y += 1150.0 * dt;
                self.vel.y = self.vel.y.min(820.0);
                if self.timer <= 0.0 && self.on_ground {
                    self.vel.x = self.facing * 115.0;
                    self.vel.y = -360.0;
                    self.timer = 1.15 + (self.anim * 3.0).sin().abs() * 0.35;
                }
                let c = move_box(world, &mut self.pos, &mut self.vel, self.w, self.h, dt, true);
                self.on_ground = c.ground;
                if self.on_ground {
                    self.vel.x *= 0.86;
                }
            }
        }
    }

    pub fn is_night_only(&self) -> bool {
        matches!(self.kind, EnemyKind::Zombie)
    }
}

#[derive(Clone, Debug)]
pub struct Drop {
    pub pos: V2,
    pub vel: V2,
    pub item: ItemId,
    pub count: u32,
    pub life: f32,
    pub bob: f32,
    pub delay: f32,
    pub on_ground: bool,
}

impl Drop {
    pub fn new(pos: V2, item: ItemId, count: u32) -> Self {
        Drop {
            pos,
            vel: V2::new(0.0, -70.0),
            item,
            count,
            life: 90.0,
            bob: 0.0,
            delay: 0.25,
            on_ground: false,
        }
    }

    pub fn update(&mut self, world: &World, player: V2, dt: f32) -> bool {
        self.life -= dt;
        self.bob += dt * 3.0;
        self.delay = (self.delay - dt).max(0.0);
        let d = dist(self.pos, player);
        if self.delay <= 0.0 && d < 110.0 {
            let dir = V2::new(player.x, player.y - 8.0).sub(self.pos).norm();
            let speed = 90.0 + (110.0 - d) * 2.0;
            self.vel = self.vel.lerp(dir.scale(speed), (dt * 8.0).min(1.0));
            self.pos = self.pos.add(self.vel.scale(dt));
            if d < 15.0 {
                return true;
            }
            return false;
        }
        self.vel.y += 900.0 * dt;
        self.vel.y = self.vel.y.min(500.0);
        self.vel.x *= 1.0 - 1.5 * dt;
        let c = move_box(world, &mut self.pos, &mut self.vel, 6.0, 6.0, dt, true);
        self.on_ground = c.ground;
        if c.ground {
            self.vel.x *= 0.8;
        }
        false
    }
}

#[derive(Clone, Debug)]
pub struct Bomb {
    pub pos: V2,
    pub vel: V2,
    pub fuse: f32,
    pub on_ground: bool,
}

impl Bomb {
    pub fn new(pos: V2, throw: V2) -> Self {
        Bomb { pos, vel: throw, fuse: 1.9, on_ground: false }
    }

    pub fn update(&mut self, world: &World, dt: f32) -> bool {
        self.fuse -= dt;
        self.vel.y += 900.0 * dt;
        self.vel.y = self.vel.y.min(600.0);
        let c = move_box(world, &mut self.pos, &mut self.vel, 8.0, 8.0, dt, true);
        self.on_ground = c.ground;
        if c.ground {
            self.vel.x *= 0.82;
        }
        self.fuse <= 0.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PKind {
    Square,
    Circle,
    Spark,
    Leaf,
    Flame,
    Smoke,
}

#[derive(Clone, Debug)]
pub struct Particle {
    pub pos: V2,
    pub vel: V2,
    pub life: f32,
    pub max_life: f32,
    pub size: f32,
    pub color: u32,
    pub gravity: f32,
    pub drag: f32,
    pub kind: PKind,
    pub spin: f32,
    pub angle: f32,
}

impl Particle {
    pub fn new(pos: V2, vel: V2, life: f32, size: f32, color: u32) -> Self {
        Particle {
            pos,
            vel,
            life,
            max_life: life,
            size,
            color,
            gravity: 420.0,
            drag: 1.0,
            kind: PKind::Square,
            spin: 0.0,
            angle: 0.0,
        }
    }

    pub fn circle(mut self) -> Self {
        self.kind = PKind::Circle;
        self
    }

    pub fn spark(mut self) -> Self {
        self.kind = PKind::Spark;
        self
    }

    pub fn flame(mut self) -> Self {
        self.kind = PKind::Flame;
        self
    }

    pub fn smoke(mut self) -> Self {
        self.kind = PKind::Smoke;
        self
    }

    pub fn leaf(mut self) -> Self {
        self.kind = PKind::Leaf;
        self.spin = 3.0;
        self
    }

    pub fn gravity(mut self, g: f32) -> Self {
        self.gravity = g;
        self
    }

    pub fn drag(mut self, d: f32) -> Self {
        self.drag = d;
        self
    }

    pub fn update(&mut self, dt: f32) {
        self.life -= dt;
        self.vel.y += self.gravity * dt;
        self.vel = self.vel.scale((1.0 - self.drag * dt).max(0.0));
        self.pos = self.pos.add(self.vel.scale(dt));
        self.angle += self.spin * dt;
    }
}

#[derive(Clone, Debug)]
pub struct FloatingText {
    pub text: String,
    pub pos: V2,
    pub vel: V2,
    pub life: f32,
    pub max_life: f32,
    pub color: u32,
    pub size: f32,
    pub outline: bool,
}

impl FloatingText {
    pub fn new(text: impl Into<String>, pos: V2, color: u32, size: f32) -> Self {
        FloatingText {
            text: text.into(),
            pos,
            vel: V2::new(0.0, -34.0),
            life: 0.9,
            max_life: 0.9,
            color,
            size,
            outline: true,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.life -= dt;
        self.pos = self.pos.add(self.vel.scale(dt));
        self.vel.y += 24.0 * dt;
    }
}

/// Dust colors for a broken tile.
pub fn tile_particle_colors(t: u8) -> [u32; 3] {
    use crate::world::*;
    match t {
        GRASS => [crate::gfx::rgb(74, 140, 58), crate::gfx::rgb(96, 168, 72), crate::gfx::rgb(92, 66, 40)],
        DIRT | CLAY => [crate::gfx::rgb(122, 84, 54), crate::gfx::rgb(94, 64, 42), crate::gfx::rgb(146, 104, 66)],
        STONE | BEDROCK => [crate::gfx::rgb(126, 130, 138), crate::gfx::rgb(96, 100, 108), crate::gfx::rgb(154, 158, 164)],
        WOOD | CACTUS => [crate::gfx::rgb(140, 94, 52), crate::gfx::rgb(104, 70, 40), crate::gfx::rgb(174, 126, 74)],
        LEAVES => [crate::gfx::rgb(66, 140, 64), crate::gfx::rgb(44, 108, 50), crate::gfx::rgb(92, 172, 82)],
        COPPER => [crate::gfx::rgb(214, 126, 70), crate::gfx::rgb(148, 92, 54), crate::gfx::rgb(126, 130, 138)],
        IRON => [crate::gfx::rgb(214, 200, 180), crate::gfx::rgb(150, 142, 130), crate::gfx::rgb(126, 130, 138)],
        GOLD => [crate::gfx::rgb(250, 208, 80), crate::gfx::rgb(196, 154, 54), crate::gfx::rgb(126, 130, 138)],
        GEM => [crate::gfx::rgb(176, 104, 255), crate::gfx::rgb(120, 70, 190), crate::gfx::rgb(126, 130, 138)],
        SAND => [crate::gfx::rgb(226, 202, 146), crate::gfx::rgb(196, 170, 118), crate::gfx::rgb(164, 138, 92)],
        SNOW => [crate::gfx::rgb(236, 246, 255), crate::gfx::rgb(198, 218, 238), crate::gfx::rgb(170, 190, 214)],
        TORCH => [crate::gfx::rgb(255, 176, 70), crate::gfx::rgb(255, 224, 140), crate::gfx::rgb(120, 82, 48)],
        _ => [crate::gfx::rgb(150, 150, 150), crate::gfx::rgb(110, 110, 110), crate::gfx::rgb(190, 190, 190)],
    }
}

pub fn spawn_burst(
    particles: &mut Vec<Particle>,
    pos: V2,
    colors: [u32; 3],
    count: i32,
    speed: f32,
    life: f32,
    size: f32,
    seed: u32,
) {
    for i in 0..count {
        let h = hash2(i, (seed & 0xffff) as i32, seed);
        let a = h * std::f32::consts::TAU;
        let s = speed * (0.35 + hash2(i + 17, 3, seed) * 0.8);
        let c = colors[(i as usize) % 3];
        let mut p = Particle::new(pos, V2::new(a.cos() * s, a.sin() * s - speed * 0.3), life * (0.6 + h * 0.6), size * (0.6 + h * 0.7), c);
        p.drag = 2.2;
        if i % 3 == 0 {
            p = p.spark();
        }
        particles.push(p);
    }
}
