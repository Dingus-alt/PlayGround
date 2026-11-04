//! World and entity rendering. Everything is drawn with the small Canvas2D
//! bridge in `gfx`, so the same code can be inspected natively.

use crate::entities::{Enemy, EnemyKind, PKind, Particle};
use crate::game::{Game, Mode};
use crate::gfx::{self, alpha, mix, rgb, rgba};
use crate::items::{ItemId, UseKind};
use crate::math::{clamp, clamp_i, lerp, V2};
use crate::rng::{fbm, hash2};
use crate::world::{self, World, TILE};

struct View {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    t: f32,
    seed: u32,
}

impl View {
    fn sx(&self, wx: f32) -> f32 {
        wx - self.x
    }
    fn sy(&self, wy: f32) -> f32 {
        wy - self.y
    }
    fn tile_x(&self, tx: i32) -> f32 {
        tx as f32 * TILE as f32 - self.x
    }
    fn tile_y(&self, ty: i32) -> f32 {
        ty as f32 * TILE as f32 - self.y
    }
}

pub fn draw(game: &Game) {
    let t = game.title_phase;
    let shake = game.shake;
    let shx = (hash2((t * 60.0) as i32, 11, game.seed) - 0.5) * shake * 1.5;
    let shy = (hash2((t * 60.0) as i32, 12, game.seed) - 0.5) * shake * 1.5;
    let view = View {
        x: game.camera.x + shx,
        y: game.camera.y + shy,
        w: game.view.x,
        h: game.view.y,
        t,
        seed: game.seed,
    };

    draw_sky(game, &view);
    draw_parallax(game, &view);
    draw_world(game, &view);
    draw_entities(game, &view);
    draw_particles(game, &view);
    draw_lighting(game, &view);
    draw_glows(game, &view);
    draw_cursor(game, &view);
    draw_floating_text(game, &view);

    // Screen effects.
    if game.flash > 0.01 {
        let a = (game.flash * 150.0).min(255.0) as u8;
        gfx::rect(0.0, 0.0, view.w, view.h, alpha(game.flash_color, a));
    }
    gfx::vignette(0.42 + game.hurt_vignette * 0.35);
}

// ---------------------------------------------------------------------------
// Sky, sun, clouds and parallax mountains
// ---------------------------------------------------------------------------

fn sky_palette(time: f32) -> (u32, u32, u32) {
    let keys: [(f32, u32, u32, u32); 7] = [
        (0.00, 0x22305C, 0xD88068, 0x8FA0C8),
        (0.10, 0x3C86C8, 0xB5E2F8, 0x9CC8E8),
        (0.28, 0x4B9CE0, 0xCDEEFF, 0xAFD8F0),
        (0.52, 0x4E92D8, 0xD6EEFF, 0xAED2EA),
        (0.60, 0x3A2A68, 0xF08A5A, 0xC87870),
        (0.68, 0x080D24, 0x1B2A52, 0x26365E),
        (0.95, 0x080D24, 0x1B2A52, 0x26365E),
    ];
    let mut top = keys[0].1;
    let mut hor = keys[0].2;
    let mut haze = keys[0].3;
    for i in 0..keys.len() - 1 {
        let a = keys[i];
        let b = keys[i + 1];
        if time >= a.0 && time <= b.0 {
            let k = ((time - a.0) / (b.0 - a.0)).max(0.0);
            let k = k * k * (3.0 - 2.0 * k);
            top = mix(a.1, b.1, k);
            hor = mix(a.2, b.2, k);
            haze = mix(a.3, b.3, k);
            break;
        }
    }
    (top, hor, haze)
}

fn draw_sky(game: &Game, v: &View) {
    let (top, horizon, haze) = sky_palette(game.time);
    gfx::gradient(0.0, 0.0, v.w, v.h, top, horizon, true);
    let light = game.sky_light();

    // Stars.
    let star_alpha = ((1.0 - light) * 255.0).clamp(0.0, 255.0) as u8;
    if star_alpha > 8 {
        for i in 0..90 {
            let sx = (hash2(i, 4, v.seed) * v.w + v.t * (0.4 + (i % 5) as f32 * 0.2)) % v.w;
            let sy = hash2(i, 7, v.seed) * v.h * 0.6;
            let tw = 0.55 + 0.45 * (v.t * 1.7 + i as f32 * 1.13).sin();
            let a = (star_alpha as f32 * tw) as u8;
            let size = if i % 9 == 0 { 1.6 } else { 1.0 };
            gfx::rect(sx, sy, size, size, rgba(220, 230, 255, a));
            if i % 17 == 0 {
                gfx::glow(sx + 0.5, sy + 0.5, 5.0, rgba(160, 190, 255, (a as f32 * 0.25) as u8));
            }
        }
    }

    // Sun.
    if game.time > 0.01 && game.time < 0.61 {
        let t = ((game.time - 0.01) / 0.60).clamp(0.0, 1.0);
        let x = v.w * (0.06 + t * 0.88);
        let y = v.h * 0.62 - (t * std::f32::consts::PI).sin() * v.h * 0.54;
        gfx::glow(x, y, 70.0, rgba(255, 228, 150, 70));
        gfx::circle(x, y, 13.0, rgb(255, 238, 176));
        gfx::circle(x, y, 9.0, rgb(255, 250, 220));
    }
    // Moon.
    if game.time > 0.61 || game.time < 0.02 {
        let t = if game.time > 0.61 {
            ((game.time - 0.61) / 0.39).clamp(0.0, 1.0)
        } else {
            ((game.time + 0.39) / 0.41).clamp(0.0, 1.0)
        };
        let x = v.w * (0.06 + t * 0.88);
        let y = v.h * 0.62 - (t * std::f32::consts::PI).sin() * v.h * 0.54;
        gfx::glow(x, y, 55.0, rgba(150, 180, 255, 45));
        gfx::circle(x, y, 11.0, rgb(224, 232, 250));
        gfx::circle(x - 3.0, y - 2.0, 2.0, rgba(172, 186, 220, 180));
        gfx::circle(x + 4.0, y + 3.0, 1.4, rgba(180, 194, 228, 160));
        gfx::circle(x + 2.0, y - 5.0, 1.2, rgba(180, 194, 228, 140));
    }

    // Stars/haze glow at horizon.
    gfx::rect(0.0, v.h * 0.58, v.w, v.h * 0.42, alpha(haze, 22));

    // Clouds.
    let cloud_tint = mix(rgb(255, 255, 255), rgb(90, 110, 160), clamp((0.55 - game.sky_light()).max(0.0) * 1.8, 0.0, 0.85));
    for i in 0..8 {
        let speed = 2.5 + (i % 3) as f32 * 1.8;
        let base = hash2(i, 21, v.seed) * (v.w + 400.0) - 200.0;
        let x = ((base + v.t * speed) % (v.w + 400.0) + (v.w + 400.0)) % (v.w + 400.0) - 200.0;
        let y = v.h * (0.06 + hash2(i, 22, v.seed) * 0.24);
        let s = 0.75 + hash2(i, 23, v.seed) * 0.9;
        let a = (120.0 * (0.45 + 0.55 * game.sky_light())) as u8;
        cloud(x, y, s, alpha(cloud_tint, a));
    }
}

fn cloud(x: f32, y: f32, s: f32, color: u32) {
    gfx::ellipse(x, y, 34.0 * s, 12.0 * s, color);
    gfx::ellipse(x - 22.0 * s, y + 3.0 * s, 20.0 * s, 9.0 * s, color);
    gfx::ellipse(x + 24.0 * s, y + 4.0 * s, 22.0 * s, 8.0 * s, color);
    gfx::ellipse(x + 4.0 * s, y - 8.0 * s, 18.0 * s, 10.0 * s, color);
    gfx::ellipse(x + 18.0 * s, y - 3.0 * s, 15.0 * s, 7.0 * s, color);
}

fn draw_parallax(game: &Game, v: &View) {
    let center_col = clamp_i(((v.x + v.w * 0.5) / TILE as f32) as i32, 0, world::W - 1);
    let horizon_world = game.world.surface_px(center_col);
    let horizon = horizon_world - v.y;
    if horizon < -v.h || horizon > v.h * 1.6 {
        return;
    }
    let (_, _, haze) = sky_palette(game.time);
    let layer_colors = [
        mix(haze, rgb(44, 62, 92), 0.45),
        mix(haze, rgb(38, 78, 66), 0.62),
        mix(haze, rgb(26, 58, 48), 0.78),
    ];
    let specs = [(0.06, 150.0, 0.55), (0.12, 115.0, 0.72), (0.20, 82.0, 0.9)];
    for (li, (par, amp, darkness)) in specs.iter().enumerate() {
        let base = horizon + 42.0 - li as f32 * 4.0;
        let off = v.x * par;
        let mut pts: Vec<V2> = Vec::new();
        pts.push(V2::new(-40.0, v.h + 20.0));
        let mut sx = -40.0;
        while sx <= v.w + 40.0 {
            let wx = sx + off;
            let n = fbm(wx * 0.0042 + li as f32 * 11.0, v.seed ^ (li as u32 * 0x1234), 3);
            let hill = (n * 0.72 + 0.28) * amp;
            let y = base - hill;
            pts.push(V2::new(sx, y));
            sx += 38.0;
        }
        pts.push(V2::new(v.w + 40.0, v.h + 20.0));
        let c = mix(layer_colors[li], rgb(8, 12, 22), 1.0 - darkness);
        gfx::poly(&pts, c);
    }
}

// ---------------------------------------------------------------------------
// Terrain
// ---------------------------------------------------------------------------

fn draw_world(game: &Game, v: &View) {
    let w = &game.world;
    let x0 = ((v.x / TILE as f32).floor() as i32 - 1).max(0);
    let y0 = ((v.y / TILE as f32).floor() as i32 - 1).max(0);
    let x1 = (((v.x + v.w) / TILE as f32).ceil() as i32 + 1).min(world::W - 1);
    let y1 = (((v.y + v.h) / TILE as f32).ceil() as i32 + 1).min(world::H - 1);

    // Solid underground backdrop to cover the sky when deep.
    let surf_y = w.surface_px(clamp_i(((v.x + v.w * 0.5) / TILE as f32) as i32, 0, world::W - 1));
    if v.y + v.h > surf_y + 64.0 {
        let dark_top = (surf_y + 64.0 - v.y).max(0.0);
        gfx::rect(0.0, dark_top, v.w, v.h - dark_top, rgb(10, 12, 20));
    }

    // Background walls.
    for ty in y0..=y1 {
        for tx in x0..=x1 {
            let wall = w.get_wall(tx, ty);
            if wall == world::WALL_NONE {
                continue;
            }
            let t = w.get(tx, ty);
            if world::is_solid(t) {
                continue;
            }
            draw_wall(v, tx, ty, wall);
        }
    }

    // Tiles.
    for ty in y0..=y1 {
        for tx in x0..=x1 {
            let t = w.get(tx, ty);
            if t == world::AIR {
                continue;
            }
            let sx = v.tile_x(tx).floor();
            let sy = v.tile_y(ty).floor();
            match t {
                world::WATER => draw_water(v, tx, ty, sx, sy, w),
                world::TORCH => draw_torch(v, tx, ty, sx, sy),
                world::PLATFORM => draw_platform(sx, sy),
                world::BENCH => draw_bench(sx, sy),
                world::MUSHROOM => draw_mushroom(sx, sy),
                _ => draw_block(v, tx, ty, t, sx, sy, w),
            }
        }
    }

    // Surface decorations (grass blades, flowers, snow sparkle).
    for ty in y0..=y1 {
        for tx in x0..=x1 {
            let t = w.get(tx, ty);
            if t != world::GRASS && t != world::SNOW && t != world::SAND {
                continue;
            }
            if w.get(tx, ty - 1) != world::AIR {
                continue;
            }
            let sx = v.tile_x(tx);
            let sy = v.tile_y(ty);
            let h = hash2(tx, ty, v.seed);
            if t == world::GRASS {
                let n = if h > 0.6 { 3 } else if h > 0.3 { 2 } else { 1 };
                for k in 0..n {
                    let ox = 3.0 + hash2(tx + k * 7, ty, v.seed) * 10.0;
                    let hh = 3.0 + hash2(tx, ty + k * 3, v.seed) * 5.0;
                    let sway = (v.t * 1.4 + tx as f32 + k as f32).sin() * 1.2;
                    gfx::line(sx + ox, sy + 1.0, sx + ox + sway, sy - hh, 1.1, rgb(66, 132, 54));
                }
                if h > 0.87 {
                    let col = if h > 0.94 { rgb(250, 214, 96) } else { rgb(228, 120, 146) };
                    gfx::rect(sx + 5.0, sy - 5.0, 2.0, 3.0, col);
                    gfx::rect(sx + 4.0, sy - 3.0, 1.5, 3.0, rgb(70, 140, 60));
                }
            } else if t == world::SNOW && h > 0.72 {
                let a = (160.0 + (v.t * 3.0 + tx as f32).sin() * 50.0) as u8;
                gfx::rect(sx + 3.0 + h * 8.0, sy - 2.0, 1.2, 1.2, rgba(255, 255, 255, a));
            }
        }
    }
}

fn draw_wall(v: &View, tx: i32, ty: i32, wall: u8) {
    let x = v.tile_x(tx).floor();
    let y = v.tile_y(ty).floor();
    let h = hash2(tx, ty, v.seed);
    let (base, dark, light) = match wall {
        world::WALL_DIRT => (rgb(58, 42, 30), rgb(40, 29, 21), rgb(78, 58, 42)),
        world::WALL_STONE => (rgb(38, 40, 46), rgb(27, 28, 34), rgb(52, 54, 62)),
        world::WALL_WOOD => (rgb(60, 43, 28), rgb(43, 31, 20), rgb(82, 60, 40)),
        _ => (rgb(30, 32, 38), rgb(22, 24, 30), rgb(44, 46, 52)),
    };
    gfx::rect(x, y, 16.5, 16.5, base);
    if wall == world::WALL_WOOD {
        gfx::rect(x + 5.0, y, 1.0, 16.0, dark);
        gfx::rect(x + 11.0, y, 1.0, 16.0, dark);
        if (ty % 3) == 0 {
            gfx::rect(x, y + 4.0, 16.0, 1.0, dark);
        }
    } else if wall == world::WALL_STONE {
        if h > 0.45 {
            gfx::rect(x + h * 9.0, y + h * 7.0, 2.0 + h * 2.0, 1.6, dark);
            gfx::rect(x + 3.0 + h * 6.0, y + 9.0 + h * 4.0, 3.0, 1.0, light);
        }
        if (tx + ty) % 4 == 0 {
            gfx::rect(x, y + 13.0, 16.0, 1.0, dark);
        }
    } else {
        if h > 0.3 {
            gfx::rect(x + h * 12.0, y + h * 12.0, 2.0, 2.0, dark);
            gfx::rect(x + (1.0 - h) * 12.0, y + (1.0 - h) * 12.0, 1.6, 1.6, light);
        }
    }
}

fn draw_block(v: &View, tx: i32, ty: i32, t: u8, x: f32, y: f32, w: &World) {
    let h = hash2(tx, ty, v.seed);
    let h2 = hash2(tx + 17, ty - 9, v.seed);
    match t {
        world::DIRT | world::GRASS | world::CLAY => {
            let (base, dark, light) = if t == world::CLAY {
                (rgb(160, 100, 78), rgb(120, 72, 56), rgb(184, 124, 96))
            } else {
                (rgb(122, 84, 52), rgb(88, 60, 38), rgb(148, 106, 66))
            };
            gfx::rect(x, y, 16.6, 16.6, base);
            gfx::rect(x, y + 13.0, 16.6, 3.6, dark);
            for i in 0..3 {
                let hh = hash2(tx * 3 + i, ty * 5 + i, v.seed);
                let xx = x + 1.5 + hh * 12.0;
                let yy = y + 1.5 + hh * 11.0;
                gfx::rect(xx, yy, 2.0, 1.6, if i % 2 == 0 { dark } else { light });
            }
            if t == world::GRASS {
                gfx::rect(x, y, 16.6, 4.5, rgb(72, 136, 56));
                gfx::rect(x, y, 16.6, 2.0, rgb(96, 164, 70));
                if h > 0.5 {
                    gfx::rect(x + h * 12.0, y + 4.0, 1.5, 2.0, rgb(58, 116, 48));
                }
            }
            if world::is_solid(w.get(tx, ty - 1)) {
                gfx::rect(x, y, 16.6, 1.2, light);
            }
        }
        world::STONE => {
            gfx::rect(x, y, 16.6, 16.6, rgb(108, 112, 120));
            gfx::rect(x, y + 13.5, 16.6, 3.1, rgb(82, 86, 94));
            if h > 0.42 {
                gfx::rect(x + h * 10.0, y + h * 10.0, 3.0, 2.0, rgb(88, 92, 100));
                gfx::rect(x + h2 * 11.0, y + h2 * 11.0, 2.0, 1.5, rgb(130, 134, 142));
            }
            if h2 > 0.66 {
                gfx::line(x + 2.0, y + 3.0, x + 7.0, y + 9.0, 1.0, rgb(84, 88, 96));
            }
            if world::is_solid(w.get(tx, ty - 1)) {
                gfx::rect(x, y, 16.6, 1.0, rgb(136, 140, 148));
            }
        }
        world::COPPER | world::IRON | world::GOLD | world::GEM => {
            let (ore, ore_dark, spark) = match t {
                world::COPPER => (rgb(218, 126, 70), rgb(160, 88, 48), rgb(255, 174, 106)),
                world::IRON => (rgb(214, 202, 184), rgb(156, 146, 132), rgb(244, 236, 220)),
                world::GOLD => (rgb(250, 208, 80), rgb(190, 150, 40), rgb(255, 240, 150)),
                _ => (rgb(178, 104, 255), rgb(120, 66, 196), rgb(226, 178, 255)),
            };
            gfx::rect(x, y, 16.6, 16.6, rgb(100, 104, 112));
            gfx::rect(x, y + 13.5, 16.6, 3.1, rgb(78, 82, 90));
            for i in 0..4 {
                let hh = hash2(tx + i * 3, ty + i * 5, v.seed);
                let ox = 2.0 + hh * 10.0;
                let oy = 2.0 + hash2(tx, ty + i, v.seed) * 10.0;
                gfx::rect(x + ox, y + oy, 3.0 + (i % 2) as f32, 2.4, ore);
                gfx::rect(x + ox, y + oy + 2.0, 2.4, 1.0, ore_dark);
                if i == 0 {
                    gfx::rect(x + ox + 0.6, y + oy + 0.5, 1.0, 0.8, spark);
                }
            }
        }
        world::WOOD => {
            gfx::rect(x, y, 16.6, 16.6, rgb(118, 82, 46));
            gfx::rect(x + 2.0, y, 2.0, 16.6, rgb(94, 64, 36));
            gfx::rect(x + 11.0, y, 1.5, 16.6, rgb(94, 64, 36));
            gfx::rect(x, y, 16.6, 1.6, rgb(142, 102, 60));
            if h > 0.5 {
                gfx::rect(x + 6.0, y + 4.0 + h * 6.0, 3.0, 1.2, rgb(96, 66, 38));
            }
        }
        world::LEAVES => {
            let base = if (tx + ty) % 3 == 0 { rgb(62, 132, 62) } else { rgb(48, 112, 54) };
            gfx::rect(x, y, 16.6, 16.6, base);
            gfx::ellipse(x + 5.0, y + 5.0, 4.0, 3.5, rgb(86, 160, 76));
            gfx::ellipse(x + 11.0, y + 9.0, 4.5, 4.0, rgb(44, 104, 50));
            gfx::ellipse(x + 7.0, y + 12.0, 5.0, 3.0, rgb(72, 146, 68));
            if h > 0.72 {
                gfx::rect(x + 3.0 + h * 9.0, y + 2.0, 1.5, 1.5, rgb(112, 188, 96));
            }
        }
        world::SAND => {
            gfx::rect(x, y, 16.6, 16.6, rgb(224, 200, 142));
            gfx::rect(x, y + 13.5, 16.6, 3.1, rgb(196, 170, 116));
            for i in 0..4 {
                let hh = hash2(tx + i, ty - i, v.seed);
                gfx::rect(x + 2.0 + hh * 12.0, y + 2.0 + hh * 11.0, 1.6, 1.2, if i % 2 == 0 { rgb(198, 172, 116) } else { rgb(244, 222, 168) });
            }
        }
        world::SNOW => {
            gfx::rect(x, y, 16.6, 16.6, rgb(224, 238, 250));
            gfx::rect(x, y + 13.5, 16.6, 3.1, rgb(186, 206, 228));
            gfx::rect(x, y, 16.6, 1.6, rgb(246, 252, 255));
            if h > 0.5 {
                gfx::rect(x + 3.0 + h * 9.0, y + 4.0 + h * 8.0, 2.0, 1.4, rgb(198, 218, 240));
            }
        }
        world::CACTUS => {
            gfx::rect(x, y, 16.6, 16.6, rgb(66, 126, 70));
            gfx::rect(x + 2.0, y, 1.8, 16.6, rgb(50, 104, 56));
            gfx::rect(x + 12.0, y, 1.5, 16.6, rgb(50, 104, 56));
            gfx::rect(x, y, 16.6, 1.5, rgb(88, 152, 88));
            if h > 0.45 {
                gfx::rect(x + 5.0 + h * 6.0, y + 4.0, 1.0, 2.4, rgb(196, 218, 160));
            }
        }
        _ => {
            gfx::rect(x, y, 16.6, 16.6, rgb(120, 120, 128));
        }
    }
}

fn draw_water(v: &View, tx: i32, ty: i32, x: f32, y: f32, w: &World) {
    let above = w.get(tx, ty - 1);
    let surf = above != world::WATER;
    let wave = if surf { (v.t * 2.4 + tx as f32 * 0.7).sin() * 1.2 } else { 0.0 };
    gfx::rect(x, y + 1.0, 16.6, 15.6, rgba(46, 104, 176, 168));
    gfx::rect(x, y + 8.0, 16.6, 8.6, rgba(30, 76, 148, 170));
    if surf {
        gfx::rect(x, y + 1.0 + wave, 16.6, 2.0, rgba(150, 214, 255, 190));
        gfx::rect(x, y + 3.0 + wave, 16.6, 1.0, rgba(112, 180, 236, 150));
        if hash2(tx, ty, v.seed) > 0.7 {
            gfx::rect(x + 4.0, y + 1.0 + wave, 3.0, 1.0, rgba(220, 246, 255, 210));
        }
    } else if hash2(tx, ty, v.seed) > 0.83 {
        gfx::rect(x + 3.0, y + 5.0 + (v.t + tx as f32).sin() * 1.0, 1.6, 1.6, rgba(130, 190, 240, 100));
    }
}

fn draw_torch(v: &View, tx: i32, ty: i32, x: f32, y: f32) {
    let flick = (v.t * 9.0 + tx as f32 * 1.7 + ty as f32).sin();
    gfx::rect(x + 7.0, y + 6.0, 2.0, 9.0, rgb(118, 78, 42));
    gfx::rect(x + 6.0, y + 4.0, 4.0, 3.0, rgb(88, 58, 32));
    let flame_h = 5.5 + flick * 1.2;
    gfx::circle(x + 8.0, y + 3.0, 3.2, rgba(255, 152, 48, 220));
    gfx::circle(x + 8.0 + flick * 0.5, y + 2.0, 2.0, rgba(255, 224, 120, 245));
    gfx::rect(x + 7.0, y + 1.0 - flame_h * 0.2, 2.0, flame_h, rgba(255, 190, 80, 220));
    if hash2(tx, ty, v.seed) > 0.5 {
        gfx::circle(x + 8.0 + flick, y - 2.0, 0.9, rgba(255, 240, 180, 180));
    }
}

fn draw_platform(x: f32, y: f32) {
    gfx::rect(x, y + 1.0, 16.6, 4.5, rgb(172, 122, 68));
    gfx::rect(x, y + 1.0, 16.6, 1.5, rgb(206, 156, 92));
    gfx::rect(x + 2.0, y + 5.0, 2.0, 3.0, rgb(122, 84, 46));
    gfx::rect(x + 12.0, y + 5.0, 2.0, 3.0, rgb(122, 84, 46));
}

fn draw_bench(x: f32, y: f32) {
    gfx::rect(x + 1.0, y + 4.0, 14.0, 3.0, rgb(178, 130, 74));
    gfx::rect(x + 1.0, y + 3.0, 14.0, 1.5, rgb(214, 166, 104));
    gfx::rect(x + 3.0, y + 7.0, 2.0, 8.0, rgb(122, 84, 46));
    gfx::rect(x + 11.0, y + 7.0, 2.0, 8.0, rgb(122, 84, 46));
    gfx::rect(x + 4.0, y + 1.0, 3.0, 2.0, rgb(140, 148, 158));
    gfx::rect(x + 9.0, y + 1.0, 1.4, 3.0, rgb(178, 138, 88));
    gfx::rect(x + 11.0, y + 1.0, 2.5, 1.4, rgb(160, 164, 172));
}

fn draw_mushroom(x: f32, y: f32) {
    let pul = 0.7 + 0.3 * (y * 0.1).sin();
    gfx::rect(x + 7.0, y + 7.0, 2.2, 6.0, rgb(198, 216, 224));
    gfx::ellipse(x + 8.0, y + 6.0, 5.5, 4.0, rgba(92, 196, 238, 230));
    gfx::ellipse(x + 6.0, y + 4.5, 2.0, 1.4, rgba(186, 246, 255, (200.0 * pul) as u8));
}

// ---------------------------------------------------------------------------
// Entities
// ---------------------------------------------------------------------------

fn draw_entities(game: &Game, v: &View) {
    // Item drops.
    for d in &game.drops {
        let bob = (d.bob * 1.6).sin() * 2.0;
        let x = v.sx(d.pos.x);
        let y = v.sy(d.pos.y) + bob;
        let a = if d.life < 5.0 { ((d.life / 5.0) * 190.0 + 40.0) as u8 } else { 230 };
        gfx::glow(x + 6.0, y + 6.0, 14.0, alpha(d.item.color(), 45));
        draw_item_icon(d.item, x + 6.0, y + 6.0, 12.0, a);
        if d.count > 1 {
            gfx::text(&d.count.to_string(), x + 10.0, y + 13.0, 5.0, rgb(255, 255, 255), 2, 700);
        }
    }

    // Enemies.
    for e in &game.enemies {
        if e.dead && e.death_timer > 0.6 {
            continue;
        }
        let a = if e.dead {
            ((1.0 - e.death_timer / 0.8).max(0.0) * 220.0) as u8
        } else {
            (e.spawned * 255.0) as u8
        };
        match e.kind {
            EnemyKind::Slime => draw_slime(v, e, a, false),
            EnemyKind::KingSlime => draw_slime(v, e, a, true),
            EnemyKind::Zombie => draw_zombie(v, e, a),
            EnemyKind::Bat => draw_bat(v, e, a),
        }
        if !e.dead && e.hp < e.max_hp && !e.kind.is_boss() {
            let x = v.sx(e.pos.x + e.w * 0.5);
            let y = v.sy(e.pos.y) - 7.0;
            gfx::rect(x - 9.0, y, 18.0, 2.4, rgba(20, 16, 24, 190));
            gfx::rect(x - 9.0, y, 18.0 * (e.hp / e.max_hp), 2.4, rgb(222, 88, 88));
        }
    }

    // Bombs.
    for b in &game.bombs {
        let x = v.sx(b.pos.x + 4.0);
        let y = v.sy(b.pos.y + 4.0);
        let pulse = if b.fuse < 0.6 { 0.6 + 0.4 * (v.t * 18.0).sin() } else { 1.0 };
        gfx::glow(x, y, 16.0 * pulse, rgba(255, 120, 60, 50));
        gfx::circle(x, y, 5.5, rgb(48, 50, 58));
        gfx::circle(x - 1.5, y - 1.5, 1.8, rgb(92, 96, 108));
        gfx::rect(x + 2.0, y - 7.0, 1.2, 4.0, rgb(130, 96, 52));
        gfx::circle(x + 2.5, y - 8.0, 1.8 * pulse, rgb(255, 196, 80));
    }

    // Player.
    draw_player(game, v);
}

fn draw_slime(v: &View, e: &Enemy, a: u8, king: bool) {
    let cx = v.sx(e.pos.x + e.w * 0.5);
    let feet = v.sy(e.pos.y + e.h);
    let speed = e.vel.y;
    let squash = if e.on_ground { (1.0 - speed.abs() / 500.0).clamp(0.0, 1.0) } else { 0.0 };
    let rx = e.w * 0.5 * (1.0 + 0.16 * squash);
    let ry = e.h * 0.62 * (1.0 - 0.18 * squash);
    let cy = feet - ry - 1.0;
    let (body, dark, light) = if king {
        (rgba(52, 130, 178, a), rgba(30, 86, 130, a), rgba(118, 202, 230, a))
    } else {
        (rgba(76, 190, 138, a), rgba(36, 128, 98, a), rgba(150, 236, 188, a))
    };
    gfx::ellipse(cx, cy + 2.0, rx + 1.0, ry + 1.0, rgba(10, 16, 22, (a as f32 * 0.3) as u8));
    gfx::ellipse(cx, cy, rx, ry, body);
    gfx::ellipse(cx, cy + ry * 0.28, rx * 0.82, ry * 0.46, dark);
    gfx::ellipse(cx - rx * 0.25, cy - ry * 0.35, rx * 0.34, ry * 0.24, light);
    // Eyes.
    let ex = rx * 0.32;
    let ey = -ry * 0.08;
    gfx::circle(cx - ex, cy + ey, if king { 3.4 } else { 2.2 }, rgba(248, 252, 250, a));
    gfx::circle(cx + ex, cy + ey, if king { 3.4 } else { 2.2 }, rgba(248, 252, 250, a));
    let look = if e.facing > 0.0 { 0.8 } else { -0.8 };
    gfx::circle(cx - ex + look, cy + ey + 0.3, if king { 1.8 } else { 1.1 }, rgba(24, 30, 34, a));
    gfx::circle(cx + ex + look, cy + ey + 0.3, if king { 1.8 } else { 1.1 }, rgba(24, 30, 34, a));
    if king {
        // Crown.
        let top = cy - ry - 2.0;
        gfx::rect(cx - 12.0, top, 24.0, 6.0, rgba(250, 202, 62, a));
        gfx::tri(cx - 12.0, top, cx - 8.0, top - 9.0, cx - 4.0, top, rgba(255, 218, 92, a));
        gfx::tri(cx - 4.0, top, cx, top - 12.0, cx + 4.0, top, rgba(255, 218, 92, a));
        gfx::tri(cx + 4.0, top, cx + 8.0, top - 9.0, cx + 12.0, top, rgba(255, 218, 92, a));
        gfx::rect(cx - 1.6, top + 1.5, 3.2, 2.8, rgba(240, 96, 90, a));
        // Angry brows.
        gfx::rect(cx - ex - 4.0, cy + ey - 5.0, 7.0, 1.6, rgba(22, 34, 48, a));
        gfx::rect(cx + ex - 3.0, cy + ey - 5.0, 7.0, 1.6, rgba(22, 34, 48, a));
    } else {
        // Mouth.
        gfx::rect(cx - 3.5, cy + ry * 0.38, 7.0, 1.5, rgba(24, 86, 68, a));
    }
}

fn draw_zombie(v: &View, e: &Enemy, a: u8) {
    let cx = v.sx(e.pos.x + e.w * 0.5);
    let feet = v.sy(e.pos.y + e.h);
    let walk = (e.anim * 7.0).sin();
    let dir = e.facing;
    let skin = rgba(122, 154, 94, a);
    let skin_dark = rgba(84, 112, 66, a);
    let shirt = rgba(74, 86, 112, a);
    let pants = rgba(52, 58, 78, a);
    // Shadow.
    gfx::ellipse(cx, feet + 1.0, 8.0, 2.0, rgba(10, 16, 22, (a as f32 * 0.28) as u8));
    // Legs.
    let l1 = walk * 3.0;
    let l2 = -walk * 3.0;
    gfx::rect(cx - 4.0 + l1, feet - 12.0, 3.2, 12.0, pants);
    gfx::rect(cx + 1.0 + l2, feet - 12.0, 3.2, 12.0, pants);
    gfx::rect(cx - 4.5 + l1, feet - 2.0, 4.4, 2.4, rgba(40, 34, 34, a));
    gfx::rect(cx + 0.4 + l2, feet - 2.0, 4.4, 2.4, rgba(40, 34, 34, a));
    // Torso.
    gfx::rect(cx - 5.0, feet - 24.0, 10.0, 13.0, shirt);
    gfx::rect(cx - 5.0, feet - 13.0, 10.0, 1.6, rgba(44, 52, 70, a));
    gfx::rect(cx - 2.0, feet - 23.0, 1.2, 9.0, rgba(96, 108, 136, a));
    // Arms reaching forward.
    let ax = cx + dir * 7.0;
    gfx::rect(ax - 2.0, feet - 22.0, 8.0, 3.0, shirt);
    gfx::rect(ax + dir * 4.0, feet - 22.0, 5.0, 2.6, skin);
    gfx::rect(ax + dir * 7.0, feet - 23.0, 2.0, 2.0, skin_dark);
    // Head.
    gfx::rect(cx - 4.5, feet - 32.0, 9.0, 9.0, skin);
    gfx::rect(cx - 4.5, feet - 33.0, 9.0, 2.0, rgba(48, 58, 44, a));
    gfx::rect(cx + dir * 1.0 - 1.5, feet - 29.0, 2.0, 2.0, rgba(22, 26, 24, a));
    gfx::rect(cx + dir * 1.0 + 2.0, feet - 29.0, 1.6, 1.6, rgba(22, 26, 24, a));
    gfx::rect(cx - 2.0, feet - 26.0, 4.0, 1.2, rgba(58, 40, 42, a));
}

fn draw_bat(v: &View, e: &Enemy, a: u8) {
    let cx = v.sx(e.pos.x + e.w * 0.5);
    let cy = v.sy(e.pos.y + e.h * 0.5);
    let flap = (e.anim * 16.0).sin();
    let wing = 7.0 + flap * 4.0;
    let body = rgba(78, 64, 92, a);
    let wing_c = rgba(58, 46, 74, a);
    gfx::tri(cx - 2.0, cy, cx - 8.0 - wing, cy - 3.0, cx - 3.0, cy + 3.0, wing_c);
    gfx::tri(cx + 2.0, cy, cx + 8.0 + wing, cy - 3.0, cx + 3.0, cy + 3.0, wing_c);
    gfx::ellipse(cx, cy, 4.0, 3.2, body);
    gfx::circle(cx - 1.5, cy - 1.0, 0.9, rgba(255, 120, 100, a));
    gfx::circle(cx + 1.5, cy - 1.0, 0.9, rgba(255, 120, 100, a));
    gfx::tri(cx - 3.0, cy - 2.0, cx - 2.0, cy - 6.0, cx - 0.5, cy - 2.5, body);
    gfx::tri(cx + 3.0, cy - 2.0, cx + 2.0, cy - 6.0, cx + 0.5, cy - 2.5, body);
}

fn draw_player(game: &Game, v: &View) {
    let p = &game.player;
    if p.dead && game.mode != Mode::Dead {
        return;
    }
    let cx = v.sx(p.pos.x + p.width * 0.5);
    let feet = v.sy(p.pos.y + p.height);
    let dir = p.facing;
    let walk = (p.walk_phase).sin();
    let moving = p.moving > 0.5 && p.on_ground;
    let bob = if moving { (p.walk_phase * 2.0).sin().abs() * 1.1 } else { 0.0 };
    let blink = p.invuln > 0.0 && ((p.invuln * 22.0) as i32 % 2 == 0);
    let a = if blink { 140 } else { 255 };
    let flash = p.hurt_flash > 0.0;
    let skin = if flash { rgba(255, 190, 170, a) } else { rgba(232, 184, 142, a) };
    let skin_dark = if flash { rgba(220, 150, 130, a) } else { rgba(196, 144, 104, a) };
    let hair = if flash { rgba(255, 220, 200, a) } else { rgba(66, 44, 34, a) };
    let shirt = if flash { rgba(220, 240, 255, a) } else { rgba(52, 122, 138, a) };
    let shirt_light = if flash { rgba(240, 250, 255, a) } else { rgba(78, 166, 172, a) };
    let pants = if flash { rgba(220, 220, 230, a) } else { rgba(58, 62, 86, a) };
    let boots = rgba(54, 38, 32, a);

    // Shadow.
    if p.on_ground {
        gfx::ellipse(cx, feet + 1.0, 8.0, 2.2, rgba(8, 14, 18, 80));
    }
    let base = feet - bob;
    let swimming = p.in_water > 0.05;
    // Legs.
    let l1 = if moving { walk * 3.5 } else { 0.0 };
    let l2 = -l1;
    let leg_h = if p.on_ground { 11.0 } else { 9.0 };
    gfx::rect(cx - 4.2 + l1, base - leg_h, 3.4, leg_h, pants);
    gfx::rect(cx + 0.8 + l2, base - leg_h, 3.4, leg_h, pants);
    gfx::rect(cx - 5.0 + l1, base - 2.2, 4.6, 2.6, boots);
    gfx::rect(cx + 0.4 + l2, base - 2.2, 4.6, 2.6, boots);
    // Torso.
    let torso_y = base - 23.0;
    gfx::rect(cx - 5.2, torso_y, 10.4, 13.0, shirt);
    gfx::rect(cx - 5.2, torso_y, 3.0, 13.0, shirt_light);
    gfx::rect(cx - 5.2, torso_y + 10.5, 10.4, 2.0, rgba(40, 92, 104, a));
    gfx::rect(cx - 1.0, torso_y + 2.0, 1.6, 8.0, rgba(230, 216, 170, a));
    // Backpack.
    gfx::rect(cx - dir * 7.0, torso_y + 2.0, 4.0, 7.0, rgba(122, 82, 48, a));
    gfx::rect(cx - dir * 7.0, torso_y + 2.0, 4.0, 2.0, rgba(160, 112, 66, a));
    // Head.
    let head_y = torso_y - 9.5;
    gfx::rect(cx - 4.8, head_y, 9.6, 9.6, skin);
    gfx::rect(cx - 4.8, head_y + 7.4, 9.6, 2.0, skin_dark);
    // Hair.
    gfx::rect(cx - 5.4, head_y - 2.0, 10.8, 3.6, hair);
    gfx::rect(cx - 5.2 - dir * 0.5, head_y, 2.2, 6.0, hair);
    gfx::rect(cx + 3.0 - dir * 0.5, head_y, 2.0, 5.0, hair);
    // Scarf.
    gfx::rect(cx - 4.6, head_y + 9.3, 9.2, 2.0, rgba(196, 72, 78, a));
    gfx::rect(cx + dir * 3.2, head_y + 10.0, 2.0, 5.0, rgba(168, 60, 66, a));
    // Face.
    let eye_x = cx + dir * 1.4;
    gfx::rect(eye_x - 0.9, head_y + 4.2, 1.8, 1.9, rgba(30, 28, 32, a));
    gfx::rect(eye_x + 2.8, head_y + 4.2, 1.4, 1.7, rgba(30, 28, 32, a));
    gfx::rect(eye_x + 3.0, head_y + 4.6, 0.6, 0.7, rgba(240, 240, 240, a));
    // Arms and held item.
    draw_player_arms(game, v, cx, torso_y, dir, a, skin, shirt);

    // Swim ripples.
    if swimming {
        gfx::ellipse(cx, base + 1.0, 11.0, 3.0, rgba(150, 210, 255, 80));
    }
}

fn draw_player_arms(game: &Game, v: &View, cx: f32, torso_y: f32, dir: f32, a: u8, skin: u32, shirt: u32) {
    let p = &game.player;
    let item = game.inventory.selected_item();
    let use_anim = p.use_anim;
    let is_sword = item.map(|i| i.is_sword()).unwrap_or(false);
    let is_pick = item.map(|i| i.is_pickaxe()).unwrap_or(false);
    // Back arm.
    gfx::rect(cx - dir * 5.0, torso_y + 2.0, 3.0, 8.0, shirt);
    gfx::rect(cx - dir * 5.4, torso_y + 9.0, 3.2, 2.4, skin);
    // Front arm angle.
    let swing = if use_anim > 0.0 {
        let t = 1.0 - use_anim;
        if is_pick {
            lerp(0.7, -1.2, (t * 3.0).min(1.0))
        } else if is_sword {
            lerp(1.4, -2.0, (t * 2.4).min(1.0))
        } else {
            lerp(0.5, -0.3, t)
        }
    } else {
        0.18 + if p.moving > 0.5 { (p.walk_phase).sin() * 0.35 } else { (v.t * 1.5).sin() * 0.04 }
    };
    let shoulder = V2::new(cx + dir * 2.0, torso_y + 3.0);
    let arm_len = 9.0;
    let hand = V2::new(
        shoulder.x + dir * (arm_len * swing.cos()),
        shoulder.y + arm_len * swing.sin() + arm_len * 0.35,
    );
    // Arm drawn as two segments.
    let mid = V2::new(
        shoulder.x + dir * (arm_len * 0.55 * swing.cos()),
        shoulder.y + arm_len * 0.55 * swing.sin() + 2.5,
    );
    gfx::line(shoulder.x, shoulder.y, mid.x, mid.y, 3.4, shirt);
    gfx::line(mid.x, mid.y, hand.x, hand.y, 2.9, shirt);
    gfx::circle(hand.x, hand.y, 1.8, skin);
    if let Some(item) = item {
        if item != ItemId::Pickaxe && !item.is_sword() && item.is_placeable() {
            // A small held block.
            draw_item_icon(item, hand.x, hand.y - 2.0, 7.0, a);
        }
    }
    if is_sword || is_pick {
        draw_held_tool(item.unwrap(), hand, swing, dir, a);
        if use_anim > 0.55 && is_sword {
            let rr = 23.0;
            let arc = 1.5;
            gfx::ring(shoulder.x, shoulder.y, rr, 2.0, rgba(230, 244, 255, (use_anim * 80.0) as u8));
            let _ = arc;
        }
    }
}

fn draw_held_tool(item: ItemId, hand: V2, angle: f32, dir: f32, a: u8) {
    let (blade, edge, handle) = match item {
        ItemId::Pickaxe => (rgb(150, 160, 172), rgb(196, 204, 214), rgb(122, 82, 46)),
        ItemId::CopperBlade => (rgb(214, 126, 74), rgb(255, 186, 116), rgb(110, 72, 42)),
        ItemId::IronBlade => (rgb(198, 208, 220), rgb(240, 248, 255), rgb(100, 66, 40)),
        ItemId::GoldBlade => (rgb(244, 202, 70), rgb(255, 240, 150), rgb(110, 72, 42)),
        _ => (rgb(176, 136, 88), rgb(210, 170, 112), rgb(110, 72, 42)),
    };
    let ang = angle * dir;
    let hx = hand.x + dir * (ang.cos() * 5.0);
    let hy = hand.y + ang.sin() * 5.0;
    if item == ItemId::Pickaxe {
        gfx::line(hand.x, hand.y, hx, hy, 1.6, alpha(handle, a));
        let tipx = hx + dir * 3.0;
        let tipy = hy - 2.0;
        gfx::line(hx, hy - 3.0, tipx, tipy, 2.0, alpha(blade, a));
        gfx::line(hx, hy - 3.0, hx - dir * 3.0, hy - 1.0, 1.6, alpha(edge, a));
    } else {
        gfx::line(hand.x, hand.y, hx, hy, 1.4, alpha(handle, a));
        let bx = hx + dir * 8.0;
        let by = hy - ang.sin() * 3.0;
        gfx::line(hx, hy, bx, by, 2.2, alpha(blade, a));
        gfx::rect(hand.x - 1.5, hand.y - 1.0, 3.0, 1.8, alpha(rgb(210, 180, 90), a));
    }
}

fn draw_particles(game: &Game, v: &View) {
    for p in &game.particles {
        let f = (p.life / p.max_life).clamp(0.0, 1.0);
        let a = (f * 255.0) as u8;
        let x = v.sx(p.pos.x);
        let y = v.sy(p.pos.y);
        match p.kind {
            PKind::Square => {
                let s = p.size * (0.5 + f * 0.7);
                gfx::rect(x - s * 0.5, y - s * 0.5, s, s, alpha(p.color, a));
            }
            PKind::Circle => {
                gfx::circle(x, y, p.size * (0.4 + f * 0.7), alpha(p.color, a));
            }
            PKind::Spark => {
                let vx = p.vel.x * 0.02;
                let vy = p.vel.y * 0.02;
                gfx::line(x, y, x - vx, y - vy, 1.2, alpha(p.color, a));
            }
            PKind::Leaf => {
                gfx::rect_rot(x, y, p.size * 1.5, p.size * 0.8, p.angle, alpha(p.color, a));
            }
            PKind::Flame => {
                gfx::circle(x, y, p.size * (0.5 + f), alpha(p.color, a));
                gfx::glow(x, y, p.size * 3.0, alpha(p.color, (a as f32 * 0.35) as u8));
            }
            PKind::Smoke => {
                gfx::circle(x, y, p.size * (1.0 + (1.0 - f) * 2.0), alpha(p.color, (a as f32 * 0.6) as u8));
            }
        }
    }
}

fn draw_floating_text(game: &Game, v: &View) {
    for t in &game.texts {
        let a = ((t.life / t.max_life).min(1.0) * 255.0) as u8;
        let x = v.sx(t.pos.x);
        let y = v.sy(t.pos.y);
        if t.outline {
            gfx::text(&t.text, x + 1.0, y + 1.0, t.size, rgba(10, 12, 18, (a as f32 * 0.75) as u8), 1, 900);
        }
        gfx::text(&t.text, x, y, t.size, alpha(t.color, a), 1, 900);
    }
}

// ---------------------------------------------------------------------------
// Lighting
// ---------------------------------------------------------------------------

struct LightMap {
    x0: i32,
    y0: i32,
    w: i32,
    h: i32,
    v: Vec<u8>,
}

impl LightMap {
    fn get(&self, x: i32, y: i32) -> f32 {
        let lx = x - self.x0;
        let ly = y - self.y0;
        if lx < 0 || ly < 0 || lx >= self.w || ly >= self.h {
            return 0.0;
        }
        self.v[(ly * self.w + lx) as usize] as f32
    }
}

fn light_cost(t: u8) -> u8 {
    match t {
        world::AIR | world::PLATFORM | world::TORCH | world::MUSHROOM => 1,
        world::WATER | world::LEAVES => 2,
        _ => 3,
    }
}

fn compute_light(game: &Game, x0: i32, y0: i32, x1: i32, y1: i32) -> LightMap {
    let pad = 5;
    let x0 = (x0 - pad).max(0);
    let y0 = (y0 - pad).max(0);
    let x1 = (x1 + pad).min(world::W - 1);
    let y1 = (y1 + pad).min(world::H - 1);
    let w = x1 - x0 + 1;
    let h = y1 - y0 + 1;
    let mut v = vec![0u8; (w * h) as usize];
    let sky = (game.sky_light() * 15.0).round().clamp(0.0, 15.0) as u8;
    let night_floor = 2u8;
    for ly in 0..h {
        for lx in 0..w {
            let x = x0 + lx;
            let y = y0 + ly;
            let t = game.world.get(x, y);
            let mut l = 0u8;
            if y < game.world.sky_top[x as usize] || t == world::WATER {
                l = (sky + if t == world::WATER { 1 } else { 0 }).max(night_floor).min(15);
            }
            if t == world::TORCH {
                l = l.max(14);
            } else if t == world::MUSHROOM {
                l = l.max(7);
            } else if t == world::GEM {
                l = l.max(3);
            }
            v[(ly * w + lx) as usize] = l;
        }
    }
    // Relaxation. Small light values make this cheap and robust.
    for _ in 0..16 {
        let mut changed = false;
        for ly in 0..h {
            for lx in 0..w {
                let i = (ly * w + lx) as usize;
                let l = v[i];
                if l <= 1 {
                    continue;
                }
                let neigh = [(lx - 1, ly), (lx + 1, ly), (lx, ly - 1), (lx, ly + 1)];
                for (nx, ny) in neigh.iter() {
                    if *nx < 0 || *ny < 0 || *nx >= w || *ny >= h {
                        continue;
                    }
                    let ni = (*ny * w + *nx) as usize;
                    let nt = game.world.get(x0 + *nx, y0 + *ny);
                    let cost = light_cost(nt);
                    let nl = l.saturating_sub(cost);
                    if nl > v[ni] {
                        v[ni] = nl;
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    LightMap { x0, y0, w, h, v }
}

fn draw_lighting(game: &Game, v: &View) {
    let x0 = ((v.x / TILE as f32).floor() as i32 - 1).max(0);
    let y0 = ((v.y / TILE as f32).floor() as i32 - 1).max(0);
    let x1 = (((v.x + v.w) / TILE as f32).ceil() as i32 + 1).min(world::W - 1);
    let y1 = (((v.y + v.h) / TILE as f32).ceil() as i32 + 1).min(world::H - 1);
    let light = compute_light(game, x0, y0, x1, y1);
    let dark = rgba(5, 7, 16, 255);
    for ty in y0..=y1 {
        for tx in x0..=x1 {
            let px = v.tile_x(tx).floor();
            let py = v.tile_y(ty).floor();
            let t00 = light.get(tx, ty);
            let t10 = light.get(tx + 1, ty);
            let t01 = light.get(tx, ty + 1);
            let t11 = light.get(tx + 1, ty + 1);
            // Four sub-cells give a soft gradient instead of hard tile steps.
            for qi in 0..4 {
                let qx = (qi % 2) as f32 * 0.5 + 0.25;
                let qy = (qi / 2) as f32 * 0.5 + 0.25;
                let l = t00 * (1.0 - qx) * (1.0 - qy)
                    + t10 * qx * (1.0 - qy)
                    + t01 * (1.0 - qx) * qy
                    + t11 * qx * qy;
                let f = (1.0 - l / 15.0).clamp(0.0, 1.0);
                let a = f.powf(1.45) * 0.965;
                if a > 0.015 {
                    gfx::rect(
                        px + (qi % 2) as f32 * 8.0,
                        py + (qi / 2) as f32 * 8.0,
                        8.25,
                        8.25,
                        alpha(dark, (a * 255.0) as u8),
                    );
                }
            }
        }
    }
}

fn draw_glows(game: &Game, v: &View) {
    let x0 = ((v.x / TILE as f32).floor() as i32 - 2).max(0);
    let y0 = ((v.y / TILE as f32).floor() as i32 - 2).max(0);
    let x1 = (((v.x + v.w) / TILE as f32).ceil() as i32 + 2).min(world::W - 1);
    let y1 = (((v.y + v.h) / TILE as f32).ceil() as i32 + 2).min(world::H - 1);
    for ty in y0..=y1 {
        for tx in x0..=x1 {
            let t = game.world.get(tx, ty);
            if t == world::TORCH {
                let x = v.tile_x(tx) + 8.0;
                let y = v.tile_y(ty) + 4.0;
                let flick = 0.82 + 0.18 * (v.t * 8.0 + tx as f32 * 1.7).sin();
                gfx::glow(x, y, 64.0 * flick, rgba(255, 168, 70, 70));
                gfx::glow(x, y, 26.0 * flick, rgba(255, 218, 140, 60));
            } else if t == world::MUSHROOM {
                let x = v.tile_x(tx) + 8.0;
                let y = v.tile_y(ty) + 4.0;
                gfx::glow(x, y, 28.0, rgba(80, 190, 255, 40));
            } else if t == world::GEM {
                let x = v.tile_x(tx) + 8.0;
                let y = v.tile_y(ty) + 8.0;
                gfx::glow(x, y, 20.0, rgba(160, 90, 245, 30));
            }
        }
    }
    // Player carries a faint aura so they never vanish entirely in the dark.
    let px = v.sx(game.player.center().x);
    let py = v.sy(game.player.center().y);
    gfx::glow(px, py, 30.0, rgba(255, 226, 170, 18));
}

// ---------------------------------------------------------------------------
// Cursor / placement
// ---------------------------------------------------------------------------

fn draw_cursor(game: &Game, v: &View) {
    if game.mode != Mode::Playing || game.inv_open || game.map_open || game.touch_mode {
        return;
    }
    let mw = game.mouse_world;
    let tx = (mw.x / TILE as f32).floor() as i32;
    let ty = (mw.y / TILE as f32).floor() as i32;
    if !World::in_bounds(tx, ty) {
        return;
    }
    let x = v.tile_x(tx).floor();
    let y = v.tile_y(ty).floor();
    let t = game.world.get(tx, ty);
    let item = game.inventory.selected_item();
    if let Some(item) = item {
        if item.is_placeable() && (t == world::AIR || t == world::WATER) {
            draw_item_icon(item, x + 8.0, y + 8.0, 11.0, 90);
        }
    }
    if t != world::AIR && t != world::WATER {
        let col = if t == world::BEDROCK { rgba(255, 120, 120, 180) } else { rgba(255, 244, 200, 200) };
        let len = 4.0;
        // Corner brackets.
        gfx::rect(x - 0.5, y - 0.5, len, 1.3, col);
        gfx::rect(x - 0.5, y - 0.5, 1.3, len, col);
        gfx::rect(x + 16.0 - len + 0.5, y - 0.5, len, 1.3, col);
        gfx::rect(x + 16.0 + 0.2, y - 0.5, 1.3, len, col);
        gfx::rect(x - 0.5, y + 16.0 - len + 0.5, len, 1.3, col);
        gfx::rect(x - 0.5, y + 16.0 + 0.2, 1.3, len, col);
        gfx::rect(x + 16.0 - len + 0.5, y + 16.0 - len + 0.5, len, 1.3, col);
        gfx::rect(x + 16.0 + 0.2, y + 16.0 + 0.2, 1.3, len, col);
    }
    // Mining cracks.
    if let Some(m) = &game.mining {
        if m.x == tx && m.y == ty {
            let p = clamp(m.progress, 0.0, 1.0);
            let cracks = (p * 5.0).floor() as i32;
            for i in 0..cracks {
                let h = hash2(tx + i * 5, ty - i * 3, game.seed);
                let cx = x + 2.0 + h * 12.0;
                let cy = y + 2.0 + hash2(tx - i, ty + i * 2, game.seed) * 12.0;
                gfx::line(cx, cy, cx + (h - 0.5) * 10.0, cy + 6.0, 1.2, rgba(18, 16, 16, 200));
            }
            // Progress arc.
            gfx::rect(x + 1.0, y + 17.0, (14.0 * p) as f32, 1.4, rgba(255, 220, 140, 190));
        }
    }
}

// ---------------------------------------------------------------------------
// Item icons (shared with the UI)
// ---------------------------------------------------------------------------

pub fn draw_item_icon(item: ItemId, cx: f32, cy: f32, size: f32, a: u8) {
    let s = size / 16.0;
    let c = |col: u32| alpha(col, a);
    match item {
        ItemId::Pickaxe => {
            let handle = c(rgb(132, 90, 50));
            let metal = c(rgb(172, 184, 198));
            gfx::line(cx - 5.0 * s, cy + 6.0 * s, cx + 4.0 * s, cy - 4.0 * s, 2.0 * s.max(1.0), handle);
            gfx::line(cx + 1.0 * s, cy - 7.0 * s, cx + 7.0 * s, cy - 3.0 * s, 2.2 * s.max(1.0), metal);
            gfx::line(cx + 1.0 * s, cy - 7.0 * s, cx - 3.0 * s, cy - 8.0 * s, 1.6 * s.max(1.0), metal);
        }
        ItemId::RustyBlade | ItemId::CopperBlade | ItemId::IronBlade | ItemId::GoldBlade => {
            let (blade, edge, hilt) = match item {
                ItemId::CopperBlade => (rgb(214, 126, 74), rgb(255, 186, 116), rgb(120, 76, 44)),
                ItemId::IronBlade => (rgb(198, 208, 220), rgb(245, 250, 255), rgb(104, 70, 42)),
                ItemId::GoldBlade => (rgb(244, 202, 70), rgb(255, 240, 150), rgb(120, 76, 44)),
                _ => (rgb(176, 136, 88), rgb(214, 174, 116), rgb(104, 70, 42)),
            };
            gfx::line(cx - 4.0 * s, cy + 7.0 * s, cx + 5.0 * s, cy - 6.0 * s, 2.6 * s.max(1.2), c(blade));
            gfx::line(cx + 1.0 * s, cy - 2.0 * s, cx + 6.5 * s, cy - 8.0 * s, 1.2 * s.max(0.8), c(edge));
            gfx::line(cx - 6.0 * s, cy + 4.0 * s, cx - 1.0 * s, cy + 8.0 * s, 2.0 * s.max(1.0), c(hilt));
            gfx::line(cx - 4.0 * s, cy + 2.0 * s, cx + 1.0 * s, cy + 5.0 * s, 1.4 * s.max(0.8), c(rgb(220, 190, 100)));
        }
        ItemId::Torch => {
            gfx::rect(cx - 1.2 * s, cy - 1.0 * s, 2.4 * s, 8.0 * s, c(rgb(126, 84, 46)));
            gfx::circle(cx, cy - 3.0 * s, 3.2 * s, c(rgb(255, 148, 46)));
            gfx::circle(cx, cy - 4.5 * s, 1.8 * s, c(rgb(255, 230, 140)));
        }
        ItemId::Dirt => block_icon(cx, cy, size, c(rgb(122, 84, 52)), c(rgb(86, 58, 36)), c(rgb(150, 108, 66))),
        ItemId::Stone => block_icon(cx, cy, size, c(rgb(112, 116, 124)), c(rgb(82, 86, 94)), c(rgb(140, 144, 152))),
        ItemId::Wood => block_icon(cx, cy, size, c(rgb(126, 88, 48)), c(rgb(92, 62, 34)), c(rgb(158, 112, 64))),
        ItemId::Sand => block_icon(cx, cy, size, c(rgb(224, 200, 142)), c(rgb(190, 164, 110)), c(rgb(244, 224, 172))),
        ItemId::Snow => block_icon(cx, cy, size, c(rgb(228, 240, 252)), c(rgb(186, 206, 228)), c(rgb(250, 255, 255))),
        ItemId::Clay => block_icon(cx, cy, size, c(rgb(160, 100, 78)), c(rgb(124, 74, 56)), c(rgb(188, 128, 100))),
        ItemId::Platform => {
            gfx::rect(cx - 7.0 * s, cy - 1.0 * s, 14.0 * s, 3.0 * s, c(rgb(182, 132, 76)));
            gfx::rect(cx - 7.0 * s, cy - 1.0 * s, 14.0 * s, 1.2 * s, c(rgb(214, 166, 104)));
        }
        ItemId::Bench => {
            gfx::rect(cx - 7.0 * s, cy - 2.0 * s, 14.0 * s, 3.0 * s, c(rgb(186, 138, 80)));
            gfx::rect(cx - 5.0 * s, cy + 1.0 * s, 2.0 * s, 6.0 * s, c(rgb(124, 86, 46)));
            gfx::rect(cx + 3.0 * s, cy + 1.0 * s, 2.0 * s, 6.0 * s, c(rgb(124, 86, 46)));
        }
        ItemId::CopperOre | ItemId::IronOre | ItemId::GoldOre | ItemId::Gem => {
            let (ore, light) = match item {
                ItemId::CopperOre => (rgb(218, 126, 70), rgb(255, 184, 116)),
                ItemId::IronOre => (rgb(214, 202, 184), rgb(248, 240, 226)),
                ItemId::GoldOre => (rgb(250, 208, 80), rgb(255, 242, 154)),
                _ => (rgb(176, 104, 255), rgb(226, 178, 255)),
            };
            gfx::ellipse(cx, cy, 6.5 * s, 5.5 * s, c(rgb(104, 108, 116)));
            gfx::circle(cx - 2.0 * s, cy, 2.2 * s, c(ore));
            gfx::circle(cx + 2.5 * s, cy + 1.0 * s, 1.8 * s, c(ore));
            gfx::circle(cx - 1.5 * s, cy - 1.0 * s, 0.8 * s, c(light));
        }
        ItemId::Gel => {
            gfx::ellipse(cx, cy + 1.0 * s, 6.0 * s, 5.0 * s, c(rgb(72, 198, 160)));
            gfx::ellipse(cx - 1.5 * s, cy - 1.0 * s, 2.4 * s, 1.6 * s, c(rgb(160, 245, 210)));
        }
        ItemId::Mushroom => {
            gfx::rect(cx - 1.0 * s, cy, 2.0 * s, 6.0 * s, c(rgb(206, 222, 228)));
            gfx::ellipse(cx, cy - 1.0 * s, 5.5 * s, 4.0 * s, c(rgb(92, 196, 238)));
            gfx::circle(cx - 1.0 * s, cy - 2.0 * s, 1.0 * s, c(rgb(196, 248, 255)));
        }
        ItemId::Bomb => {
            gfx::circle(cx, cy + 1.0 * s, 5.0 * s, c(rgb(48, 50, 58)));
            gfx::circle(cx - 1.5 * s, cy - 1.0 * s, 1.5 * s, c(rgb(92, 96, 108)));
            gfx::rect(cx + 2.0 * s, cy - 6.0 * s, 1.0 * s, 4.0 * s, c(rgb(130, 96, 52)));
            gfx::circle(cx + 2.5 * s, cy - 7.0 * s, 1.4 * s, c(rgb(255, 206, 100)));
        }
        ItemId::Crown => {
            gfx::rect(cx - 6.0 * s, cy - 1.0 * s, 12.0 * s, 4.0 * s, c(rgb(250, 202, 62)));
            gfx::tri(cx - 6.0 * s, cy - 1.0 * s, cx - 3.0 * s, cy - 7.0 * s, cx, cy - 1.0 * s, c(rgb(255, 218, 92)));
            gfx::tri(cx - 1.0 * s, cy - 1.0 * s, cx + 1.0 * s, cy - 9.0 * s, cx + 3.0 * s, cy - 1.0 * s, c(rgb(255, 218, 92)));
            gfx::tri(cx + 2.0 * s, cy - 1.0 * s, cx + 5.0 * s, cy - 6.0 * s, cx + 6.0 * s, cy - 1.0 * s, c(rgb(255, 218, 92)));
            gfx::circle(cx, cy + 0.8 * s, 1.0 * s, c(rgb(230, 88, 88)));
        }
        ItemId::Trophy => {
            gfx::rect(cx - 4.0 * s, cy - 5.0 * s, 8.0 * s, 5.0 * s, c(rgb(244, 198, 70)));
            gfx::tri(cx - 4.0 * s, cy, cx, cy + 4.0 * s, cx + 4.0 * s, cy, c(rgb(210, 158, 48)));
            gfx::rect(cx - 1.2 * s, cy + 3.0 * s, 2.4 * s, 3.0 * s, c(rgb(200, 150, 50)));
            gfx::rect(cx - 3.5 * s, cy + 6.0 * s, 7.0 * s, 2.0 * s, c(rgb(150, 104, 42)));
        }
        ItemId::Potion => {
            gfx::rect(cx - 1.2 * s, cy - 6.0 * s, 2.4 * s, 3.0 * s, c(rgb(220, 220, 228)));
            gfx::ellipse(cx, cy + 1.0 * s, 4.2 * s, 4.8 * s, c(rgb(226, 72, 120)));
            gfx::rect(cx - 2.4 * s, cy - 4.0 * s, 4.8 * s, 2.4 * s, c(rgb(196, 202, 214)));
            gfx::circle(cx - 1.0 * s, cy - 0.5 * s, 1.0 * s, c(rgb(255, 150, 180)));
        }
    }
}

fn block_icon(cx: f32, cy: f32, size: f32, base: u32, dark: u32, light: u32) {
    let s = size * 0.5;
    gfx::rect(cx - s * 0.7, cy - s * 0.7, s * 1.4, s * 1.4, base);
    gfx::rect(cx - s * 0.7, cy + s * 0.35, s * 1.4, s * 0.35, dark);
    gfx::rect(cx - s * 0.7, cy - s * 0.7, s * 1.4, s * 0.25, light);
    gfx::rect(cx - s * 0.25, cy - s * 0.15, s * 0.25, s * 0.25, dark);
}
