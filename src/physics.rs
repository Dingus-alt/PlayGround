//! Simple AABB movement shared by enemies, drops and bombs.

use crate::math::V2;
use crate::world::{is_solid, World, TILE, PLATFORM};

#[derive(Clone, Copy, Debug, Default)]
pub struct Contact {
    pub ground: bool,
    pub ceiling: bool,
    pub wall: bool,
}

pub fn move_box(world: &World, pos: &mut V2, vel: &mut V2, w: f32, h: f32, dt: f32, use_platforms: bool) -> Contact {
    let mut contact = Contact::default();
    let dx = vel.x * dt;
    let dy = vel.y * dt;
    let steps = ((dx.abs().max(dy.abs()) / 6.0).ceil() as i32).clamp(1, 12);

    for _ in 0..steps {
        // X
        let sx = dx / steps as f32;
        if sx.abs() > 0.0001 {
            pos.x += sx;
            let x0 = (pos.x / TILE as f32).floor() as i32;
            let x1 = ((pos.x + w) / TILE as f32).floor() as i32;
            let y0 = (pos.y / TILE as f32).floor() as i32;
            let y1 = ((pos.y + h - 0.01) / TILE as f32).floor() as i32;
            'x: for y in y0..=y1 {
                for x in x0..=x1 {
                    if is_solid(world.get(x, y)) {
                        if sx > 0.0 {
                            pos.x = x as f32 * TILE as f32 - w - 0.01;
                        } else {
                            pos.x = (x + 1) as f32 * TILE as f32 + 0.01;
                        }
                        vel.x = 0.0;
                        contact.wall = true;
                        break 'x;
                    }
                }
            }
        }

        // Y
        let sy = dy / steps as f32;
        if sy.abs() > 0.0001 {
            let prev_bottom = pos.y + h;
            pos.y += sy;
            let x0 = (pos.x / TILE as f32).floor() as i32;
            let x1 = ((pos.x + w) / TILE as f32).floor() as i32;
            let y0 = (pos.y / TILE as f32).floor() as i32;
            let y1 = ((pos.y + h) / TILE as f32).floor() as i32;
            'y: for y in y0..=y1 {
                for x in x0..=x1 {
                    let t = world.get(x, y);
                    let solid = is_solid(t);
                    let plat = use_platforms && t == PLATFORM;
                    if solid || plat {
                        if sy > 0.0 {
                            if plat {
                                let top = y as f32 * TILE as f32;
                                if prev_bottom > top + 1.0 {
                                    continue;
                                }
                            }
                            pos.y = y as f32 * TILE as f32 - h - 0.01;
                            vel.y = 0.0;
                            contact.ground = true;
                            break 'y;
                        } else if sy < 0.0 && solid {
                            pos.y = (y + 1) as f32 * TILE as f32 + 0.01;
                            vel.y = 0.0;
                            contact.ceiling = true;
                            break 'y;
                        }
                    }
                }
            }
        }
    }
    contact
}

pub fn overlaps(world: &World, pos: V2, w: f32, h: f32) -> bool {
    let x0 = (pos.x / TILE as f32).floor() as i32;
    let x1 = ((pos.x + w) / TILE as f32).floor() as i32;
    let y0 = (pos.y / TILE as f32).floor() as i32;
    let y1 = ((pos.y + h) / TILE as f32).floor() as i32;
    for y in y0..=y1 {
        for x in x0..=x1 {
            if is_solid(world.get(x, y)) {
                return true;
            }
        }
    }
    false
}
