//! WebAssembly entry points for Terraforge.
//!
//! The browser owns the canvas; Rust owns the entire simulation and draws via
//! a tiny immediate-mode bridge.

mod entities;
mod game;
mod gfx;
mod input;
mod items;
mod math;
mod physics;
mod player;
mod render;
mod rng;
mod ui;
mod world;

use std::cell::RefCell;
use game::Game;
use input::Input;
use math::V2;

thread_local! {
    static GAME: RefCell<Option<Game>> = RefCell::new(None);
    static INPUT: RefCell<Input> = RefCell::new(Input::new());
    static DEBUG: RefCell<Vec<u8>> = RefCell::new(Vec::new());
}

fn with_game<R>(f: impl FnOnce(&mut Game) -> R) -> Option<R> {
    GAME.with(|g| g.borrow_mut().as_mut().map(f))
}

fn with_input<R>(f: impl FnOnce(&mut Input) -> R) -> R {
    INPUT.with(|i| f(&mut i.borrow_mut()))
}

#[no_mangle]
pub extern "C" fn game_init(view_w: f32, view_h: f32, seed: u32) {
    let view = V2::new(view_w.max(200.0), view_h.max(200.0));
    let game = Game::new(view, seed);
    gfx::minimap(game.world.minimap.as_ptr(), world::W as u32, world::H as u32);
    GAME.with(|g| *g.borrow_mut() = Some(game));
    INPUT.with(|i| *i.borrow_mut() = Input::new());
}

#[no_mangle]
pub extern "C" fn game_resize(view_w: f32, view_h: f32) {
    with_game(|g| g.resize(V2::new(view_w.max(200.0), view_h.max(200.0))));
}

#[no_mangle]
pub extern "C" fn game_tick(dt: f32) {
    with_input(|input| {
        with_game(|g| {
            g.update(dt, input);
            g.render();
        });
        input.end_frame();
    });
}

#[no_mangle]
pub extern "C" fn game_key(key: u32, down: u32) {
    let k = key as usize;
    if k >= input::KEY_COUNT {
        return;
    }
    with_input(|i| {
        let was = i.keys[k];
        i.keys[k] = down != 0;
        if down != 0 && !was {
            i.pressed[k] = true;
        }
        i.any_input = true;
    });
}

#[no_mangle]
pub extern "C" fn game_pointer_move(x: f32, y: f32) {
    with_input(|i| {
        i.mouse = V2::new(x, y);
        i.any_input = true;
    });
}

#[no_mangle]
pub extern "C" fn game_pointer_down(button: u32) {
    with_input(|i| {
        if button == 0 {
            i.mouse_down = true;
            i.mouse_pressed = true;
        } else if button == 2 || button == 1 {
            i.right_down = true;
            i.right_pressed = true;
        }
        i.any_input = true;
    });
}

#[no_mangle]
pub extern "C" fn game_pointer_up(button: u32) {
    with_input(|i| {
        if button == 0 {
            i.mouse_down = false;
            i.mouse_released = true;
        } else if button == 2 || button == 1 {
            i.right_down = false;
            i.right_released = true;
        }
        i.any_input = true;
    });
}

#[no_mangle]
pub extern "C" fn game_wheel(delta: f32) {
    with_input(|i| i.wheel += delta);
}

#[no_mangle]
pub extern "C" fn game_set_touch_mode(touch: u32) {
    with_game(|g| g.set_touch_mode(touch != 0));
    with_input(|i| i.touch_mode = touch != 0);
}

#[no_mangle]
pub extern "C" fn game_play() {
    with_game(|g| g.play());
}

#[no_mangle]
pub extern "C" fn game_new_world(seed: u32) {
    with_game(|g| g.new_world(seed));
}

#[no_mangle]
pub extern "C" fn game_has_save() -> u32 {
    with_game(|g| g.has_save as u32).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn game_toggle_inventory() {
    with_game(|g| g.toggle_inventory());
}

#[no_mangle]
pub extern "C" fn game_toggle_map() {
    with_game(|g| g.toggle_map());
}

#[no_mangle]
pub extern "C" fn game_toggle_pause() {
    with_game(|g| g.toggle_pause());
}

#[no_mangle]
pub extern "C" fn game_toggle_place() {
    with_game(|g| g.toggle_place());
}

#[no_mangle]
pub extern "C" fn game_toggle_mute() {
    with_game(|g| g.toggle_mute());
}

#[no_mangle]
pub extern "C" fn game_save() {
    with_game(|g| g.save_game());
}

// ---------------------------------------------------------------------------
// Debug / verification helpers. These are harmless in normal play.
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn game_debug_state() -> u32 {
    let json = with_game(|g| g.debug_json()).unwrap_or_else(|| "{}".to_string());
    DEBUG.with(|d| {
        let mut d = d.borrow_mut();
        *d = json.into_bytes();
        d.len() as u32
    })
}

#[no_mangle]
pub extern "C" fn game_debug_ptr() -> *const u8 {
    DEBUG.with(|d| d.borrow().as_ptr())
}

#[no_mangle]
pub extern "C" fn game_debug_give(item: u32, count: u32) {
    with_game(|g| {
        if let Some(id) = items::ItemId::from_u8(item as u8) {
            g.inventory.add(id, count);
        }
    });
}

#[no_mangle]
pub extern "C" fn game_debug_teleport(x: f32, y: f32) {
    with_game(|g| {
        g.player.pos = V2::new(x, y);
        g.player.vel = V2::ZERO;
        g.player.respawn = g.player.pos;
        g.center_camera();
    });
}

#[no_mangle]
pub extern "C" fn game_debug_time(t: f32) {
    with_game(|g| g.time = t.rem_euclid(1.0));
}

#[no_mangle]
pub extern "C" fn game_debug_heal() {
    with_game(|g| {
        g.player.hp = g.player.max_hp;
        g.player.dead = false;
    });
}

#[no_mangle]
pub extern "C" fn game_debug_hurt(amount: f32) {
    with_game(|g| {
        g.player.hp = (g.player.hp - amount.max(0.0)).max(0.0);
        g.player.invuln = 0.0;
    });
}

#[no_mangle]
pub extern "C" fn game_debug_spawn_boss() {
    with_game(|g| {
        g.inventory.add(items::ItemId::Crown, 1);
        g.flags.summoned_boss = true;
        g.boss_active = true;
        let pc = g.player.center();
        g.enemies.push(entities::Enemy::new(
            entities::EnemyKind::KingSlime,
            V2::new(pc.x + 120.0, pc.y - 80.0),
        ));
    });
}

#[no_mangle]
pub extern "C" fn game_debug_kill_boss() {
    with_game(|g| {
        for i in 0..g.enemies.len() {
            if g.enemies[i].kind.is_boss() && !g.enemies[i].dead {
                g.enemies[i].hp = 0.0;
                g.enemies[i].dead = true;
                g.boss_active = false;
                g.boss_defeated = true;
                g.flags.boss_defeated = true;
                g.victory_timer = 0.2;
            }
        }
    });
}

#[no_mangle]
pub extern "C" fn game_debug_craft(index: u32) {
    with_game(|g| g.craft(index as usize));
}
