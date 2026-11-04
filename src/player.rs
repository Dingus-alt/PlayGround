//! Player state, movement and tile collision.

use crate::input::Input;
use crate::math::{approach, clamp, V2};
use crate::world::{World, TILE};

pub const PLAYER_W: f32 = 11.0;
pub const PLAYER_H: f32 = 27.0;

#[derive(Clone, Debug)]
pub struct Player {
    pub pos: V2,
    pub vel: V2,
    pub width: f32,
    pub height: f32,
    pub on_ground: bool,
    pub facing: f32,
    pub hp: f32,
    pub max_hp: f32,
    pub invuln: f32,
    pub hurt_flash: f32,
    pub walk_phase: f32,
    pub coyote: f32,
    pub jump_buffer: f32,
    pub in_water: f32,
    pub swim_phase: f32,
    pub use_anim: f32,
    pub use_item: u8,
    pub attack_cooldown: f32,
    pub potion_cooldown: f32,
    pub respawn: V2,
    pub dead: bool,
    pub death_timer: f32,
    pub moving: f32,
    pub fall_speed: f32,
}

impl Player {
    pub fn new(spawn: V2) -> Self {
        Player {
            pos: spawn,
            vel: V2::ZERO,
            width: PLAYER_W,
            height: PLAYER_H,
            on_ground: false,
            facing: 1.0,
            hp: 100.0,
            max_hp: 100.0,
            invuln: 0.0,
            hurt_flash: 0.0,
            walk_phase: 0.0,
            coyote: 0.0,
            jump_buffer: 0.0,
            in_water: 0.0,
            swim_phase: 0.0,
            use_anim: 0.0,
            use_item: 0,
            attack_cooldown: 0.0,
            potion_cooldown: 0.0,
            respawn: spawn,
            dead: false,
            death_timer: 0.0,
            moving: 0.0,
            fall_speed: 0.0,
        }
    }

    pub fn center(&self) -> V2 {
        V2::new(self.pos.x + self.width * 0.5, self.pos.y + self.height * 0.5)
    }

    pub fn feet(&self) -> V2 {
        V2::new(self.pos.x + self.width * 0.5, self.pos.y + self.height)
    }

    pub fn aabb(&self) -> (f32, f32, f32, f32) {
        (self.pos.x, self.pos.y, self.width, self.height)
    }

    pub fn respawn(&mut self) {
        self.pos = self.respawn;
        self.vel = V2::ZERO;
        self.hp = self.max_hp;
        self.dead = false;
        self.death_timer = 0.0;
        self.invuln = 2.0;
        self.in_water = 0.0;
    }

    pub fn hurt(&mut self, amount: f32, from_x: f32) -> bool {
        if self.invuln > 0.0 || self.dead {
            return false;
        }
        self.hp -= amount;
        self.invuln = 0.65;
        self.hurt_flash = 0.4;
        let dir = if self.center().x < from_x { -1.0 } else { 1.0 };
        self.vel.x = dir * 165.0;
        self.vel.y = -175.0;
        if self.hp <= 0.0 {
            self.hp = 0.0;
            self.dead = true;
            self.death_timer = 0.0;
        }
        true
    }

    pub fn heal(&mut self, amount: f32) -> bool {
        if self.hp >= self.max_hp {
            return false;
        }
        self.hp = (self.hp + amount).min(self.max_hp);
        true
    }

    pub fn update(&mut self, world: &World, input: &Input, dt: f32) -> bool {
        let mut jumped = false;
        if self.dead {
            self.death_timer += dt;
            self.vel.y = (self.vel.y + 1100.0 * dt).min(700.0);
            self.pos = self.pos.add(self.vel.scale(dt));
            return false;
        }

        self.invuln = (self.invuln - dt).max(0.0);
        self.hurt_flash = (self.hurt_flash - dt).max(0.0);
        self.attack_cooldown = (self.attack_cooldown - dt).max(0.0);
        self.potion_cooldown = (self.potion_cooldown - dt).max(0.0);
        self.use_anim = (self.use_anim - dt * 5.5).max(0.0);
        self.coyote = (self.coyote - dt).max(0.0);
        self.jump_buffer = (self.jump_buffer - dt).max(0.0);

        let feet = self.feet();
        let water = world.water_px(feet.x, feet.y - 2.0) || world.water_px(self.center().x, self.center().y);
        if water {
            self.in_water = (self.in_water + dt).min(4.0);
        } else {
            self.in_water = (self.in_water - dt * 3.0).max(0.0);
        }
        let swimming = self.in_water > 0.05;

        if input.jump_pressed() {
            self.jump_buffer = 0.14;
        }

        let mut dir = 0.0;
        if input.left() {
            dir -= 1.0;
        }
        if input.right() {
            dir += 1.0;
        }
        if dir != 0.0 {
            self.facing = dir;
        }

        let max_speed = if swimming { 92.0 } else { 154.0 };
        let accel = if self.on_ground {
            1250.0
        } else if swimming {
            500.0
        } else {
            760.0
        };
        if dir != 0.0 {
            self.vel.x = approach(self.vel.x, dir * max_speed, accel * dt);
            self.walk_phase += dt * (if self.on_ground { 9.5 } else { 3.5 });
            self.moving = 1.0;
        } else {
            let friction = if self.on_ground { 1500.0 } else if swimming { 420.0 } else { 260.0 };
            self.vel.x = approach(self.vel.x, 0.0, friction * dt);
            self.moving = 0.0;
        }

        if swimming {
            self.vel.y += 220.0 * dt;
            self.vel.y = self.vel.y.min(105.0);
            if input.jump_held() {
                self.vel.y -= 520.0 * dt;
                self.vel.y = self.vel.y.max(-125.0);
                self.swim_phase += dt * 9.0;
                self.walk_phase += dt * 5.0;
            }
            if input.down() {
                self.vel.y += 220.0 * dt;
            }
            self.vel.y *= 1.0 - 1.6 * dt;
        } else {
            self.vel.y += 1420.0 * dt;
            self.vel.y = self.vel.y.min(780.0);
            if self.jump_buffer > 0.0 && (self.on_ground || self.coyote > 0.0) {
                self.vel.y = -392.0;
                self.on_ground = false;
                self.coyote = 0.0;
                self.jump_buffer = 0.0;
                jumped = true;
            }
            // Variable jump height.
            if !input.jump_held() && self.vel.y < -140.0 {
                self.vel.y += 1500.0 * dt;
            }
        }

        if self.on_ground {
            self.coyote = 0.1;
        }
        self.fall_speed = self.vel.y;

        let was_ground = self.on_ground;
        self.pos.x += self.vel.x * dt;
        self.resolve_x(world);
        let prev_bottom = self.pos.y + self.height;
        self.pos.y += self.vel.y * dt;
        self.resolve_y(world, prev_bottom);
        if self.on_ground && !was_ground && self.fall_speed > 420.0 {
            // landing handled by game for particles
        }
        if self.pos.y > (crate::world::H as f32 * TILE as f32) + 200.0 {
            self.hp = 0.0;
            self.dead = true;
        }
        self.pos.x = clamp(self.pos.x, 1.0, (crate::world::W as f32 * TILE as f32) - self.width - 1.0);
        jumped
    }

    fn resolve_x(&mut self, world: &World) {
        let x0 = (self.pos.x / TILE as f32).floor() as i32;
        let x1 = ((self.pos.x + self.width) / TILE as f32).floor() as i32;
        let y0 = (self.pos.y / TILE as f32).floor() as i32;
        let y1 = ((self.pos.y + self.height - 0.01) / TILE as f32).floor() as i32;
        for y in y0..=y1 {
            for x in x0..=x1 {
                if world.solid(x, y) {
                    if self.vel.x > 0.0 {
                        self.pos.x = x as f32 * TILE as f32 - self.width - 0.01;
                    } else if self.vel.x < 0.0 {
                        self.pos.x = (x + 1) as f32 * TILE as f32 + 0.01;
                    }
                    self.vel.x = 0.0;
                    return;
                }
            }
        }
    }

    fn resolve_y(&mut self, world: &World, prev_bottom: f32) {
        let x0 = (self.pos.x / TILE as f32).floor() as i32;
        let x1 = ((self.pos.x + self.width) / TILE as f32).floor() as i32;
        let y0 = (self.pos.y / TILE as f32).floor() as i32;
        let y1 = ((self.pos.y + self.height) / TILE as f32).floor() as i32;
        self.on_ground = false;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let t = world.get(x, y);
                let solid = crate::world::is_solid(t);
                let plat = t == crate::world::PLATFORM;
                if solid || (plat && self.vel.y >= 0.0) {
                    if self.vel.y > 0.0 {
                        if plat {
                            let top = y as f32 * TILE as f32;
                            if prev_bottom > top + 1.0 {
                                continue;
                            }
                        }
                        self.pos.y = y as f32 * TILE as f32 - self.height - 0.01;
                        self.vel.y = 0.0;
                        self.on_ground = true;
                        return;
                    } else if self.vel.y < 0.0 && solid {
                        self.pos.y = (y + 1) as f32 * TILE as f32 + 0.01;
                        self.vel.y = 0.0;
                        return;
                    }
                }
            }
        }
        // Platform check: support only from above.
        if self.vel.y >= 0.0 {
            let feet_y = self.pos.y + self.height;
            let ty = (feet_y / TILE as f32).floor() as i32;
            for x in x0..=x1 {
                if world.get(x, ty) == crate::world::PLATFORM {
                    let top = ty as f32 * TILE as f32;
                    if feet_y >= top - 0.5 && feet_y <= top + 2.5 {
                        self.pos.y = top - self.height - 0.01;
                        self.vel.y = 0.0;
                        self.on_ground = true;
                        return;
                    }
                }
            }
        }
    }

    pub fn intersects_tile(&self, tx: i32, ty: i32) -> bool {
        let x = tx as f32 * TILE as f32;
        let y = ty as f32 * TILE as f32;
        self.pos.x < x + TILE as f32
            && self.pos.x + self.width > x
            && self.pos.y < y + TILE as f32
            && self.pos.y + self.height > y
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::world;

    #[test]
    fn player_falls_and_lands() {
        let mut w = world::generate(42);
        // A flat test platform.
        for x in 100..130 {
            for y in 100..104 {
                w.set_tile(x, y, world::STONE);
            }
            for y in 90..100 {
                w.set_tile(x, y, world::AIR);
            }
        }
        let mut p = Player::new(V2::new(108.0 * 16.0, 92.0 * 16.0));
        let input = Input::new();
        for _ in 0..240 {
            p.update(&w, &input, 1.0 / 60.0);
        }
        assert!(p.on_ground, "player should land");
        assert!(p.pos.y + p.height <= 100.0 * 16.0 + 1.0);
    }

    #[test]
    fn player_jumps() {
        let mut w = world::generate(42);
        for x in 100..130 {
            for y in 100..104 {
                w.set_tile(x, y, world::STONE);
            }
            for y in 88..100 {
                w.set_tile(x, y, world::AIR);
            }
        }
        let mut p = Player::new(V2::new(108.0 * 16.0, 98.0 * 16.0));
        let mut input = Input::new();
        for _ in 0..30 {
            p.update(&w, &input, 1.0 / 60.0);
        }
        input.keys[crate::input::KEY_SPACE] = true;
        input.pressed[crate::input::KEY_SPACE] = true;
        p.update(&w, &input, 1.0 / 60.0);
        assert!(p.vel.y < 0.0);
    }
}
