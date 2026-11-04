//! Core game state, simulation, combat, building and progression.

use crate::entities::*;
use crate::gfx;
use crate::input::*;
use crate::items::*;
use crate::math::{aabb_overlap, clamp, clamp_i, dist, ease_out, lerp, V2};
use crate::player::Player;
use crate::render;
use crate::rng::{hash2, Rng};
use crate::ui::{self, Rect};
use crate::world::{self, World, TILE};

pub const CYCLE_SECONDS: f32 = 240.0;
pub const NIGHT_START: f32 = 0.62;
pub const NIGHT_END: f32 = 0.95;

// Sound effect ids (mirrored in web/main.js).
pub const SFX_JUMP: u32 = 1;
pub const SFX_SWING: u32 = 2;
pub const SFX_HIT: u32 = 3;
pub const SFX_MINE: u32 = 4;
pub const SFX_BREAK: u32 = 5;
pub const SFX_PLACE: u32 = 6;
pub const SFX_PICKUP: u32 = 7;
pub const SFX_HURT: u32 = 8;
pub const SFX_DEATH: u32 = 9;
pub const SFX_EXPLODE: u32 = 10;
pub const SFX_CRAFT: u32 = 11;
pub const SFX_HEAL: u32 = 12;
pub const SFX_CLICK: u32 = 13;
pub const SFX_BOSS: u32 = 14;
pub const SFX_VICTORY: u32 = 15;
pub const SFX_SPLASH: u32 = 16;
pub const SFX_BANNER: u32 = 17;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Attract,
    Playing,
    Paused,
    Dead,
    Victory,
}

#[derive(Clone, Debug)]
pub struct Mining {
    pub x: i32,
    pub y: i32,
    pub progress: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct QuestFlags {
    pub got_wood: bool,
    pub crafted_bench: bool,
    pub placed_bench: bool,
    pub crafted_blade: bool,
    pub summoned_boss: bool,
    pub boss_defeated: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Stats {
    pub blocks_mined: u32,
    pub blocks_placed: u32,
    pub enemies_killed: u32,
    pub deaths: u32,
    pub play_time: f32,
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub text: String,
    pub sub: String,
    pub life: f32,
    pub max_life: f32,
    pub color: u32,
}

#[derive(Clone, Debug)]
pub struct Banner {
    pub text: String,
    pub sub: String,
    pub life: f32,
    pub max_life: f32,
    pub color: u32,
}

#[derive(Clone, Debug)]
pub struct Recipe {
    pub name: &'static str,
    pub output: ItemId,
    pub out_count: u32,
    pub inputs: [Option<(ItemId, u32)>; 3],
    pub bench: bool,
    pub hint: &'static str,
}

pub const RECIPES: [Recipe; 9] = [
    Recipe {
        name: "Torches",
        output: ItemId::Torch,
        out_count: 4,
        inputs: [Some((ItemId::Wood, 1)), Some((ItemId::Gel, 1)), None],
        bench: false,
        hint: "A little light for the dark.",
    },
    Recipe {
        name: "Wood Platforms",
        output: ItemId::Platform,
        out_count: 2,
        inputs: [Some((ItemId::Wood, 1)), None, None],
        bench: false,
        hint: "Jump through from below.",
    },
    Recipe {
        name: "Workbench",
        output: ItemId::Bench,
        out_count: 1,
        inputs: [Some((ItemId::Wood, 8)), None, None],
        bench: false,
        hint: "Unlocks advanced crafting nearby.",
    },
    Recipe {
        name: "Bombs",
        output: ItemId::Bomb,
        out_count: 2,
        inputs: [Some((ItemId::Stone, 4)), Some((ItemId::Gel, 2)), None],
        bench: false,
        hint: "Blows a hole in almost anything.",
    },
    Recipe {
        name: "Healing Draught",
        output: ItemId::Potion,
        out_count: 1,
        inputs: [Some((ItemId::Mushroom, 2)), Some((ItemId::Gel, 2)), None],
        bench: false,
        hint: "Restores 45 health.",
    },
    Recipe {
        name: "Copper Broadsword",
        output: ItemId::CopperBlade,
        out_count: 1,
        inputs: [Some((ItemId::CopperOre, 8)), Some((ItemId::Wood, 2)), None],
        bench: true,
        hint: "26 damage. A proper blade.",
    },
    Recipe {
        name: "Iron Broadsword",
        output: ItemId::IronBlade,
        out_count: 1,
        inputs: [Some((ItemId::IronOre, 8)), Some((ItemId::Wood, 2)), None],
        bench: true,
        hint: "38 damage. Cuts through the night.",
    },
    Recipe {
        name: "Gilded Broadsword",
        output: ItemId::GoldBlade,
        out_count: 1,
        inputs: [Some((ItemId::GoldOre, 8)), Some((ItemId::Wood, 2)), None],
        bench: true,
        hint: "54 damage. Fit for a king.",
    },
    Recipe {
        name: "Slime Crown",
        output: ItemId::Crown,
        out_count: 1,
        inputs: [Some((ItemId::Gel, 10)), Some((ItemId::GoldOre, 3)), None],
        bench: true,
        hint: "Use it to summon The Slime King.",
    },
];

const SAVE_MAGIC: &str = "TERRAFORGE1";

#[derive(Clone, Debug)]
pub struct Game {
    pub world: World,
    pub player: Player,
    pub inventory: Inventory,
    pub enemies: Vec<Enemy>,
    pub drops: Vec<Drop>,
    pub bombs: Vec<Bomb>,
    pub particles: Vec<Particle>,
    pub texts: Vec<FloatingText>,
    pub toasts: Vec<Toast>,
    pub banner: Option<Banner>,
    pub mode: Mode,
    pub inv_open: bool,
    pub map_open: bool,
    pub show_help: bool,
    pub view: V2,
    pub camera: V2,
    pub shake: f32,
    pub flash: f32,
    pub flash_color: u32,
    pub time: f32,
    pub day: u32,
    pub mining: Option<Mining>,
    pub swing_hit: bool,
    pub boss_active: bool,
    pub boss_defeated: bool,
    pub victory_timer: f32,
    pub death_timer: f32,
    pub flags: QuestFlags,
    pub stats: Stats,
    pub regen_timer: f32,
    pub spawn_timer: f32,
    pub ambient_timer: f32,
    pub music_mode: u32,
    pub title_phase: f32,
    pub seed: u32,
    pub rng: Rng,
    pub touch_mode: bool,
    pub has_save: bool,
    pub saved: Option<SaveData>,
    pub save_timer: f32,
    pub muted: bool,
    pub was_night: bool,
    pub boss_wave_flags: u8,
    pub hurt_vignette: f32,
    pub mouse_world: V2,
    pub mouse_screen: V2,
    pub hover_recipe: Option<usize>,
    pub hover_inv_slot: Option<usize>,
    pub last_mode_before_pause: Mode,
    pub boss_intro: f32,
    pub total_gel_earned: u32,
}

impl Game {
    pub fn new(view: V2, seed: u32) -> Game {
        let world = World::generate(seed);
        let player = Player::new(world.spawn);
        let mut game = Game {
            world,
            player,
            inventory: Inventory::new(),
            enemies: Vec::new(),
            drops: Vec::new(),
            bombs: Vec::new(),
            particles: Vec::new(),
            texts: Vec::new(),
            toasts: Vec::new(),
            banner: None,
            mode: Mode::Attract,
            inv_open: false,
            map_open: false,
            show_help: false,
            view,
            camera: V2::ZERO,
            shake: 0.0,
            flash: 0.0,
            flash_color: 0xFFFFFF80,
            time: 0.28,
            day: 1,
            mining: None,
            swing_hit: false,
            boss_active: false,
            boss_defeated: false,
            victory_timer: 0.0,
            death_timer: 0.0,
            flags: QuestFlags::default(),
            stats: Stats::default(),
            regen_timer: 0.0,
            spawn_timer: 4.0,
            ambient_timer: 0.0,
            music_mode: 0,
            title_phase: 0.0,
            seed,
            rng: Rng::new(seed ^ 0xABCD_1234),
            touch_mode: false,
            has_save: false,
            saved: None,
            save_timer: 0.0,
            muted: false,
            was_night: false,
            boss_wave_flags: 0,
            hurt_vignette: 0.0,
            mouse_world: V2::ZERO,
            mouse_screen: V2::ZERO,
            hover_recipe: None,
            hover_inv_slot: None,
            last_mode_before_pause: Mode::Playing,
            boss_intro: 0.0,
            total_gel_earned: 0,
        };
        game.center_camera();
        game.was_night = game.is_night();
        game.load_from_storage();
        game
    }

    pub fn center_camera(&mut self) {
        let target = self.player.center().sub(V2::new(self.view.x * 0.5, self.view.y * 0.5));
        self.camera = self.clamp_camera(target);
    }

    fn clamp_camera(&self, c: V2) -> V2 {
        let max_x = (world::W as f32 * TILE as f32 - self.view.x).max(0.0);
        let max_y = (world::H as f32 * TILE as f32 - self.view.y).max(0.0);
        V2::new(clamp(c.x, 0.0, max_x), clamp(c.y, 0.0, max_y))
    }

    pub fn resize(&mut self, view: V2) {
        self.view = view;
        self.camera = self.clamp_camera(self.camera);
    }

    pub fn is_night(&self) -> bool {
        self.time >= NIGHT_START || self.time < 0.03
    }

    pub fn sky_light(&self) -> f32 {
        // Smoothly dim around dusk and dawn.
        let t = self.time;
        let day = if t < 0.03 {
            lerp(0.35, 0.7, t / 0.03)
        } else if t < 0.10 {
            lerp(0.7, 1.0, (t - 0.03) / 0.07)
        } else if t < 0.52 {
            1.0
        } else if t < 0.64 {
            lerp(1.0, 0.35, (t - 0.52) / 0.12)
        } else if t < 0.90 {
            0.33
        } else {
            lerp(0.33, 0.7, (t - 0.90) / 0.05)
        };
        day
    }

    pub fn objective(&self) -> (&'static str, &'static str) {
        if !self.flags.got_wood {
            ("Chop down a tree", "Hold left click on a trunk with your pickaxe")
        } else if !self.flags.crafted_bench && self.inventory.count(ItemId::Bench) == 0 && !self.flags.placed_bench {
            ("Craft a Workbench", "Press E to open crafting (8 wood)")
        } else if !self.flags.placed_bench {
            ("Place your Workbench", "Select it on the hotbar and click the ground")
        } else if !self.flags.crafted_blade {
            ("Forge a better blade", "Mine 8 copper ore, then craft at the Workbench")
        } else if !self.flags.summoned_boss && self.inventory.count(ItemId::Crown) == 0 {
            ("Prepare the Slime Crown", "Gather 10 gel from slimes and 3 gold ore")
        } else if !self.flags.summoned_boss {
            ("Summon The Slime King", "Use the Slime Crown from your hotbar")
        } else if !self.flags.boss_defeated {
            ("Defeat The Slime King", "Dodge the hops. Aim for the crown.")
        } else {
            ("Keep exploring", "Build, mine and make the world yours")
        }
    }

    pub fn play(&mut self) {
        if let Some(save) = self.saved.clone() {
            self.apply_save(save);
        }
        self.mode = Mode::Playing;
        self.inv_open = false;
        self.map_open = false;
        self.show_help = false;
        if self.player.hp <= 0.0 {
            self.player.respawn();
        }
        self.toast("Welcome to Terraforge", "Chop trees, craft, and survive the night", 0xFFE8B0);
        self.save_timer = 18.0;
    }

    pub fn new_world(&mut self, seed: u32) {
        self.seed = seed;
        self.world = World::generate(seed);
        self.player = Player::new(self.world.spawn);
        self.inventory = Inventory::new();
        self.enemies.clear();
        self.drops.clear();
        self.bombs.clear();
        self.particles.clear();
        self.texts.clear();
        self.toasts.clear();
        self.banner = None;
        self.flags = QuestFlags::default();
        self.stats = Stats::default();
        self.time = 0.28;
        self.day = 1;
        self.boss_active = false;
        self.boss_defeated = false;
        self.victory_timer = 0.0;
        self.death_timer = 0.0;
        self.mining = None;
        self.has_save = false;
        self.saved = None;
        self.rng = Rng::new(seed ^ 0xABCD_1234);
        self.mode = Mode::Playing;
        self.inv_open = false;
        self.map_open = false;
        self.center_camera();
        self.was_night = self.is_night();
        self.toast("A new world awaits", "Every seed is different", 0x9FE8A0);
        self.save_game();
    }

    pub fn set_touch_mode(&mut self, touch: bool) {
        self.touch_mode = touch;
    }

    pub fn toggle_inventory(&mut self) {
        if self.mode != Mode::Playing {
            return;
        }
        self.inv_open = !self.inv_open;
        self.map_open = false;
        if self.inv_open {
            self.mining = None;
        }
        gfx::sfx(SFX_CLICK, 0.5);
    }

    pub fn toggle_map(&mut self) {
        if self.mode != Mode::Playing && !self.map_open {
            return;
        }
        self.map_open = !self.map_open;
        self.inv_open = false;
        gfx::sfx(SFX_CLICK, 0.4);
    }

    pub fn toggle_pause(&mut self) {
        if self.mode == Mode::Playing {
            self.last_mode_before_pause = Mode::Playing;
            self.mode = Mode::Paused;
            self.inv_open = false;
            self.map_open = false;
        } else if self.mode == Mode::Paused {
            self.mode = Mode::Playing;
        }
        gfx::sfx(SFX_CLICK, 0.5);
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        gfx::set_mute(self.muted);
    }

    pub fn toggle_place(&mut self) {
        // Touch-friendly alternative: cycle to the first placeable hotbar item.
        for i in 0..HOTBAR {
            if let Some(s) = self.inventory.slots[i] {
                if s.id.is_placeable() {
                    self.inventory.selected = i;
                    gfx::sfx(SFX_CLICK, 0.35);
                    return;
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Main update
    // ------------------------------------------------------------------

    pub fn update(&mut self, dt: f32, input: &Input) {
        let dt = clamp(dt, 0.0, 0.05);
        self.title_phase += dt;
        self.shake = (self.shake - dt * 18.0).max(0.0);
        self.flash = (self.flash - dt * 2.4).max(0.0);
        self.hurt_vignette = (self.hurt_vignette - dt * 1.8).max(0.0);
        self.touch_mode = input.touch_mode;
        self.mouse_screen = input.mouse;

        match self.mode {
            Mode::Attract => {
                self.update_attract(dt);
                if input.press(KEY_ENTER) {
                    self.play();
                }
            }
            Mode::Playing => {
                if self.inv_open {
                    self.handle_inventory_ui(input);
                    self.update_ambient(dt);
                    self.update_camera(dt);
                } else if self.map_open {
                    if input.press(KEY_M) || input.press(KEY_ESC) || input.mouse_pressed {
                        self.map_open = false;
                    }
                } else {
                    self.update_playing(dt, input);
                }
                if input.press(KEY_ESC) {
                    self.toggle_pause();
                }
            }
            Mode::Paused => {
                self.handle_pause_ui(input);
                if input.press(KEY_ESC) {
                    self.mode = Mode::Playing;
                }
            }
            Mode::Dead => {
                self.update_dead(dt, input);
            }
            Mode::Victory => {
                self.update_victory(dt, input);
            }
        }

        // Toast lifetimes tick in every mode so they never stick around.
        for t in self.toasts.iter_mut() {
            t.life -= dt;
        }
        self.toasts.retain(|t| t.life > 0.0);
        if let Some(b) = self.banner.as_mut() {
            b.life -= dt;
            if b.life <= 0.0 {
                self.banner = None;
            }
        }
        self.boss_intro = (self.boss_intro - dt).max(0.0);
    }

    fn update_attract(&mut self, dt: f32) {
        self.time += dt / CYCLE_SECONDS * 0.35;
        if self.time >= 1.0 {
            self.time -= 1.0;
            self.day += 1;
        }
        let t = self.title_phase;
        let base = self.world.spawn;
        let px = base.x + (t * 0.06).sin() * 170.0;
        let py = base.y - self.view.y * 0.28 + (t * 0.3).sin() * 10.0;
        self.camera = self.clamp_camera(V2::new(px - self.view.x * 0.5, py));
        for p in self.particles.iter_mut() {
            p.update(dt);
        }
        self.particles.retain(|p| p.life > 0.0);
        self.ambient_timer -= dt;
        if self.ambient_timer <= 0.0 {
            self.ambient_timer = 0.35;
            self.spawn_ambient();
        }
        let music = 4;
        if music != self.music_mode {
            self.music_mode = music;
            gfx::music(music);
        }
    }

    fn update_playing(&mut self, dt: f32, input: &Input) {
        self.stats.play_time += dt;
        self.time += dt / CYCLE_SECONDS;
        if self.time >= 1.0 {
            self.time -= 1.0;
            self.day += 1;
            self.banner("A NEW DAY", &format!("Day {}", self.day), 0.9, 0xF2D38A);
            gfx::sfx(SFX_BANNER, 0.5);
        }
        let night = self.is_night();
        if night != self.was_night {
            self.was_night = night;
            if night {
                self.banner("NIGHT FALLS", "The dead are stirring", 0.85, 0xB8A8FF);
                gfx::sfx(SFX_BANNER, 0.6);
            } else {
                self.banner("DAWN BREAKS", "You survived the night", 0.9, 0xFFD9A0);
                gfx::sfx(SFX_BANNER, 0.5);
            }
        }

        // Player movement.
        let hp_before = self.player.hp;
        let was_ground = self.player.on_ground;
        let vy_before = self.player.vel.y;
        let swimming = self.player.in_water > 0.05;
        let jumped = self.player.update(&self.world, input, dt);
        if jumped {
            gfx::sfx(SFX_JUMP, 0.35);
            let feet = self.player.feet();
            for i in 0..4 {
                let p = Particle::new(
                    feet,
                    V2::new((hash2(i, 2, self.seed) - 0.5) * 40.0, -10.0),
                    0.35,
                    1.5,
                    0x9A8A70CC,
                )
                .gravity(80.0)
                .drag(2.0);
                self.particles.push(p);
            }
        }
        if !was_ground && self.player.on_ground && vy_before > 380.0 {
            let feet = self.player.feet();
            for i in 0..8 {
                let dir = if i % 2 == 0 { -1.0 } else { 1.0 };
                self.particles.push(
                    Particle::new(
                        feet,
                        V2::new(dir * (20.0 + hash2(i, 9, self.seed) * 60.0), -20.0),
                        0.5,
                        2.0,
                        0x8A7A60AA,
                    )
                    .gravity(140.0)
                    .drag(2.4),
                );
            }
            if vy_before > 620.0 {
                self.shake = (self.shake + 2.5).min(8.0);
            }
        }
        if swimming && self.particles.len() < 300 && self.rng.chance(0.25) {
            self.particles.push(
                Particle::new(self.player.center(), V2::new(self.rng.range(-20.0, 20.0), -40.0), 0.5, 1.6, 0x8FD4FFCC)
                    .gravity(100.0)
                    .drag(2.0)
                    .circle(),
            );
        }
        if self.player.hp < hp_before {
            self.hurt_vignette = 1.0;
            self.regen_timer = 0.0;
        }
        self.regen_timer += dt;
        if self.regen_timer > 6.0 && self.player.hp > 0.0 && self.player.hp < self.player.max_hp {
            self.player.hp = (self.player.hp + 3.2 * dt).min(self.player.max_hp);
        }

        // Cactus contact.
        let feet = self.player.feet();
        let cx = (feet.x / TILE as f32) as i32;
        let cy = (feet.y / TILE as f32) as i32;
        for y in (cy - 1)..=(cy + 1) {
            for x in (cx - 1)..=(cx + 1) {
                if self.world.get(x, y) == world::CACTUS
                    && self.player.invuln <= 0.0
                    && self.player.intersects_tile(x, y)
                {
                    if self.player.hurt(7.0, (x as f32 + 0.5) * TILE as f32) {
                        gfx::sfx(SFX_HURT, 0.6);
                        self.shake = 3.0;
                    }
                }
            }
        }

        // Hotbar selection and HUD interaction.
        let hotbar = ui::hotbar_layout(self);
        let hover_hotbar = hotbar.contains(input.mouse);
        if let Some(slot) = hotbar.slot_at(input.mouse) {
            if input.mouse_pressed {
                self.inventory.selected = slot;
                gfx::sfx(SFX_CLICK, 0.35);
            }
        }
        if input.wheel.abs() > 0.01 && !self.inv_open {
            let step = if input.wheel > 0.0 { 1 } else { HOTBAR - 1 };
            self.inventory.selected = (self.inventory.selected + step) % HOTBAR;
            gfx::sfx(SFX_CLICK, 0.3);
        }
        for i in 0..10 {
            if input.press(KEY_DIGIT_1 + if i == 9 { 9 } else { i }) {
                // Digits 1..9 map to slots 0..8 and 0 maps to slot 9.
                let slot = if i == 9 { 9 } else { i };
                self.inventory.selected = slot;
                gfx::sfx(SFX_CLICK, 0.3);
            }
        }
        if input.press(KEY_E) || input.press(KEY_C) {
            self.inv_open = !self.inv_open;
            self.mining = None;
        }

        // Camera and cursor.
        self.update_camera(dt);
        let mouse_world = V2::new(self.camera.x + input.mouse.x, self.camera.y + input.mouse.y);
        self.mouse_world = mouse_world;
        let ui_consumed = hover_hotbar
            || (self.touch_mode && input.mouse.y > self.view.y * 0.72)
            || ui::top_bar_contains(self, input.mouse);

        if !ui_consumed {
            self.handle_use(input, mouse_world, dt);
        } else {
            self.mining = None;
        }

        // Simulation.
        self.update_enemies(dt);
        self.update_bombs(dt);
        self.update_drops(dt);
        self.update_particles(dt);
        self.update_ambient(dt);
        self.spawn_logic(dt);

        // Keep the minimap texture in sync occasionally.
        if self.world.map_dirty {
            gfx::minimap(self.world.minimap.as_ptr(), world::W as u32, world::H as u32);
            self.world.map_dirty = false;
        }

        // Quest flags.
        if self.inventory.count(ItemId::Wood) > 0 {
            self.flags.got_wood = true;
        }

        // Death.
        if self.player.hp <= 0.0 && self.mode == Mode::Playing {
            self.player.dead = true;
            self.mode = Mode::Dead;
            self.death_timer = 0.0;
            self.stats.deaths += 1;
            self.mining = None;
            self.boss_active = false;
            self.enemies.retain(|e| !e.kind.is_boss());
            self.banner("YOU DIED", "The world is unkind", 1.0, 0xFF6A6A);
            gfx::sfx(SFX_DEATH, 0.8);
            self.shake = 8.0;
            self.save_game();
            return;
        }

        // Victory transition after the boss dies.
        if self.boss_defeated && self.mode == Mode::Playing {
            self.victory_timer -= dt;
            if self.victory_timer <= 0.0 {
                self.mode = Mode::Victory;
                self.death_timer = 0.0;
                gfx::sfx(SFX_VICTORY, 0.9);
            }
        }

        // Music.
        let target = if self.mode == Mode::Victory {
            5
        } else if self.boss_active {
            3
        } else if night {
            2
        } else {
            1
        };
        if target != self.music_mode {
            self.music_mode = target;
            gfx::music(target);
        }

        // Autosave.
        self.save_timer -= dt;
        if self.save_timer <= 0.0 {
            self.save_timer = 20.0;
            self.save_game();
        }
    }

    fn update_camera(&mut self, dt: f32) {
        let look = self.mouse_world.sub(self.player.center()).scale(0.055);
        let look = V2::new(clamp(look.x, -50.0, 50.0), clamp(look.y, -35.0, 35.0));
        let target = self
            .player
            .center()
            .add(look)
            .sub(V2::new(self.view.x * 0.5, self.view.y * 0.5));
        let target = self.clamp_camera(target);
        let k = 1.0 - (-8.0 * dt).exp();
        self.camera = self.camera.lerp(target, k);
    }

    fn update_dead(&mut self, dt: f32, input: &Input) {
        self.death_timer += dt;
        for p in self.particles.iter_mut() {
            p.update(dt);
        }
        self.particles.retain(|p| p.life > 0.0);
        self.update_camera(dt);
        let layout = ui::death_layout(self);
        if (input.mouse_pressed && layout.respawn.contains(input.mouse)) || input.press(KEY_ENTER) || input.press(KEY_SPACE) {
            if self.death_timer > 0.55 {
                self.respawn_player();
            }
        }
        if input.mouse_pressed && layout.quit.contains(input.mouse) {
            self.save_game();
            self.mode = Mode::Attract;
            gfx::show_title();
        }
        let music = 0;
        if music != self.music_mode {
            self.music_mode = music;
            gfx::music(music);
        }
    }

    fn respawn_player(&mut self) {
        self.player.respawn();
        self.mode = Mode::Playing;
        self.enemies.clear();
        self.bombs.clear();
        self.mining = None;
        self.boss_active = false;
        self.boss_wave_flags = 0;
        self.victory_timer = if self.boss_defeated { f32::MAX } else { 0.0 };
        self.center_camera();
        self.banner("BACK ON YOUR FEET", "Try a torch this time", 0.8, 0xA8E8FF);
        self.save_game();
    }

    fn update_victory(&mut self, dt: f32, input: &Input) {
        self.death_timer += dt;
        for p in self.particles.iter_mut() {
            p.update(dt);
        }
        self.particles.retain(|p| p.life > 0.0);
        self.update_camera(dt);
        let layout = ui::victory_layout(self);
        if (input.mouse_pressed && layout.continue_btn.contains(input.mouse))
            || input.press(KEY_ENTER)
            || input.press(KEY_SPACE)
        {
            self.mode = Mode::Playing;
            self.victory_timer = f32::MAX;
            self.banner("THE KING IS DEAD", "Keep building your world", 1.0, 0xFFD75E);
        }
        if input.mouse_pressed && layout.quit.contains(input.mouse) {
            self.save_game();
            self.mode = Mode::Attract;
            gfx::show_title();
        }
    }

    // ------------------------------------------------------------------
    // Building / combat input
    // ------------------------------------------------------------------

    fn handle_use(&mut self, input: &Input, mouse_world: V2, dt: f32) {
        let item = self.inventory.selected_item();
        let Some(item) = item else {
            self.mining = None;
            return;
        };
        let tx = (mouse_world.x / TILE as f32).floor() as i32;
        let ty = (mouse_world.y / TILE as f32).floor() as i32;
        let pc = self.player.center();
        let tile_center = V2::new(tx as f32 * TILE as f32 + 8.0, ty as f32 * TILE as f32 + 8.0);
        let reach = dist(pc, tile_center) <= TILE as f32 * 6.2;

        let left = input.mouse_down;
        let right = input.right_down;

        match item.use_kind() {
            UseKind::Mine => {
                if left && reach {
                    let t = self.world.get(tx, ty);
                    if t != world::AIR && t != world::WATER && t != world::BEDROCK {
                        let same = self.mining.as_ref().map(|m| m.x == tx && m.y == ty).unwrap_or(false);
                        if !same {
                            self.mining = Some(Mining { x: tx, y: ty, progress: 0.0 });
                        }
                        if let Some(m) = self.mining.as_mut() {
                            let hard = world::hardness(t);
                            m.progress += dt / hard.max(0.03);
                            if m.progress >= 1.0 {
                                self.break_tile(tx, ty, true);
                                self.mining = None;
                            } else if self.rng.chance(dt * 14.0) {
                                let colors = tile_particle_colors(t);
                                let pos = V2::new(
                                    tx as f32 * TILE as f32 + self.rng.range(2.0, 14.0),
                                    ty as f32 * TILE as f32 + self.rng.range(2.0, 14.0),
                                );
                                self.particles.push(
                                    Particle::new(pos, V2::new(self.rng.range(-45.0, 45.0), self.rng.range(-70.0, -10.0)), 0.4, 1.6, colors[0])
                                        .gravity(520.0),
                                );
                                if self.rng.chance(0.25) {
                                    gfx::sfx(SFX_MINE, 0.12);
                                }
                            }
                        }
                    } else {
                        self.mining = None;
                    }
                } else {
                    self.mining = None;
                }
            }
            UseKind::Swing => {
                self.mining = None;
                if input.mouse_pressed || input.right_pressed {
                    self.swing_sword(item);
                }
            }
            UseKind::Place => {
                self.mining = None;
                if input.mouse_pressed || input.right_pressed || (left && self.touch_mode) {
                    self.place_selected(tx, ty);
                }
            }
            UseKind::Bomb => {
                self.mining = None;
                if input.mouse_pressed || input.right_pressed {
                    self.throw_bomb(mouse_world);
                }
            }
            UseKind::Heal => {
                self.mining = None;
                if input.mouse_pressed || input.right_pressed {
                    self.drink_potion();
                }
            }
            UseKind::Summon => {
                self.mining = None;
                if input.mouse_pressed || input.right_pressed {
                    self.summon_boss();
                }
            }
            UseKind::None => {
                self.mining = None;
            }
        }
    }

    fn swing_sword(&mut self, item: ItemId) {
        if self.player.attack_cooldown > 0.0 {
            return;
        }
        self.player.attack_cooldown = 0.36;
        self.player.use_anim = 1.0;
        gfx::sfx(SFX_SWING, 0.45);
        let pc = self.player.center();
        let dir = self.player.facing;
        let ax = pc.x + if dir > 0.0 { 4.0 } else { -38.0 };
        let ay = pc.y - 14.0;
        let aw = 34.0;
        let ah = 36.0;
        let mut hits = 0;
        for i in 0..self.enemies.len() {
            if self.enemies[i].dead {
                continue;
            }
            let (ex, ey, ew, eh) = self.enemies[i].aabb();
            if aabb_overlap(ax, ay, aw, ah, ex, ey, ew, eh) {
                let dmg = item.sword_damage() as f32 * self.rng.range(0.85, 1.15);
                let (kind, cx, cy) = {
                    let e = &self.enemies[i];
                    (e.kind, e.center().x, e.center().y)
                };
                let killed = self.enemies[i].hurt(dmg, 190.0, pc.x);
                self.stats_damage_text(dmg, cx, cy - 10.0);
                self.particles.push(
                    Particle::new(V2::new(cx, cy), V2::new(dir * 100.0, -30.0), 0.3, 3.0, 0xE8E0D0FF).spark(),
                );
                if killed {
                    self.kill_enemy(i);
                } else if kind.is_boss() {
                    self.shake = (self.shake + 1.5).min(6.0);
                }
                hits += 1;
                if hits >= 5 {
                    break;
                }
            }
        }
        if hits > 0 {
            gfx::sfx(SFX_HIT, 0.65);
            self.shake = (self.shake + 1.2).min(5.0);
        }
    }

    fn place_selected(&mut self, tx: i32, ty: i32) {
        let Some(stack) = self.inventory.selected_stack() else { return };
        let Some(tile) = stack.id.place_tile() else { return };
        if !World::in_bounds(tx, ty) {
            return;
        }
        let target = self.world.get(tx, ty);
        if target != world::AIR && target != world::WATER {
            return;
        }
        let pc = self.player.center();
        let tile_center = V2::new(tx as f32 * TILE as f32 + 8.0, ty as f32 * TILE as f32 + 8.0);
        if dist(pc, tile_center) > TILE as f32 * 6.2 {
            return;
        }
        // Don't place solid blocks inside the player.
        if world::is_solid(tile) && self.player.intersects_tile(tx, ty) {
            return;
        }
        let neighbor = self.world.solid(tx - 1, ty)
            || self.world.solid(tx + 1, ty)
            || self.world.solid(tx, ty - 1)
            || self.world.solid(tx, ty + 1)
            || self.world.get_wall(tx, ty) != world::WALL_NONE
            || self.world.platform(tx - 1, ty)
            || self.world.platform(tx + 1, ty)
            || self.world.platform(tx, ty - 1)
            || self.world.platform(tx, ty + 1);
        if !neighbor {
            return;
        }
        if tile == world::TORCH || tile == world::PLATFORM {
            let attached = self.world.solid(tx, ty + 1)
                || self.world.get_wall(tx, ty) != world::WALL_NONE
                || (tile == world::PLATFORM
                    && (self.world.solid(tx - 1, ty) || self.world.solid(tx + 1, ty)));
            if !attached {
                return;
            }
        }
        self.inventory.remove_selected_one();
        self.world.set_tile(tx, ty, tile);
        self.stats.blocks_placed += 1;
        if tile == world::BENCH {
            self.flags.placed_bench = true;
            self.toast("Workbench placed", "Advanced recipes are now available", 0xF2CE8A);
        }
        if tile == world::TORCH {
            gfx::sfx(SFX_PLACE, 0.6);
        } else {
            gfx::sfx(SFX_PLACE, 0.45);
        }
        let colors = tile_particle_colors(tile);
        spawn_burst(&mut self.particles, tile_center, colors, 4, 50.0, 0.3, 1.4, tx as u32 ^ ty as u32);
        self.player.use_anim = 0.55;
        self.save_timer = self.save_timer.min(6.0);
    }

    fn throw_bomb(&mut self, mouse_world: V2) {
        if self.inventory.count(ItemId::Bomb) == 0 {
            return;
        }
        self.inventory.consume(ItemId::Bomb, 1);
        let pc = self.player.center();
        let dir = mouse_world.sub(pc).norm();
        let origin = V2::new(pc.x + dir.x * 8.0, pc.y - 4.0);
        self.bombs.push(Bomb::new(origin, V2::new(dir.x * 220.0, -150.0)));
        self.player.use_anim = 0.8;
        gfx::sfx(SFX_SWING, 0.4);
    }

    fn drink_potion(&mut self) {
        if self.player.potion_cooldown > 0.0 {
            return;
        }
        if self.player.hp >= self.player.max_hp {
            return;
        }
        if self.inventory.count(ItemId::Potion) == 0 {
            return;
        }
        self.inventory.consume(ItemId::Potion, 1);
        self.player.heal(45.0);
        self.player.potion_cooldown = 2.0;
        self.regen_timer = 0.0;
        gfx::sfx(SFX_HEAL, 0.7);
        self.float_text("+45", self.player.center().add(V2::new(0.0, -20.0)), 0x8CFF9A, 7.0);
        for _ in 0..12 {
            self.particles.push(
                Particle::new(
                    self.player.center().add(V2::new(self.rng.range(-8.0, 8.0), self.rng.range(-12.0, 12.0))),
                    V2::new(self.rng.range(-20.0, 20.0), self.rng.range(-55.0, -20.0)),
                    0.7,
                    1.6,
                    0x8CFF9ACC,
                )
                .gravity(-30.0),
            );
        }
    }

    fn summon_boss(&mut self) {
        if self.boss_active {
            self.toast("The Slime King is already here", "There is no escaping now", 0xFFB070);
            return;
        }
        if self.boss_defeated {
            self.toast("The Slime King is already defeated", "The crown is spent", 0xB8B8C8);
            return;
        }
        if self.inventory.count(ItemId::Crown) == 0 {
            return;
        }
        self.inventory.consume(ItemId::Crown, 1);
        self.flags.summoned_boss = true;
        self.boss_active = true;
        self.boss_wave_flags = 0;
        let pc = self.player.center();
        let dir = if self.player.facing >= 0.0 { 1.0 } else { -1.0 };
        let bx = clamp(pc.x + dir * 180.0, 40.0, world::W as f32 * TILE as f32 - 80.0);
        let by = pc.y - 80.0;
        self.enemies.push(Enemy::new(EnemyKind::KingSlime, V2::new(bx, by)));
        self.banner("THE SLIME KING AWAKENS", "A crown of gel and greed", 1.0, 0x9CF0B0);
        self.boss_intro = 3.0;
        self.shake = 8.0;
        self.flash = 0.8;
        self.flash_color = 0x8CFFB040;
        gfx::sfx(SFX_BOSS, 0.9);
    }

    fn stats_damage_text(&mut self, dmg: f32, x: f32, y: f32) {
        let text = if dmg >= 40.0 {
            format!("{}!", dmg.round() as i32)
        } else {
            format!("{}", dmg.round() as i32)
        };
        let color = if dmg >= 40.0 { 0xFFD75E } else { 0xFFF0C8 };
        self.float_text(&text, V2::new(x, y), color, 6.0);
    }

    fn kill_enemy(&mut self, index: usize) {
        let (kind, pos, center) = {
            let e = &self.enemies[index];
            (e.kind, e.pos, e.center())
        };
        self.stats.enemies_killed += 1;
        match kind {
            EnemyKind::Slime => {
                let gel = self.rng.range_i(1, 3) as u32;
                self.spawn_drop(pos.add(V2::new(2.0, 2.0)), ItemId::Gel, gel);
                self.total_gel_earned += gel;
                self.float_text(&format!("+{} Gel", gel), center.add(V2::new(0.0, -8.0)), 0x8CFFB0, 5.0);
                spawn_burst(&mut self.particles, center, [0x5CC49EFF, 0x86E8C0FF, 0x3C9078FF], 10, 90.0, 0.5, 2.2, 77);
            }
            EnemyKind::Zombie => {
                if self.rng.chance(0.5) {
                    self.spawn_drop(pos.add(V2::new(2.0, 2.0)), ItemId::Gel, 1);
                }
                spawn_burst(&mut self.particles, center, [0x78945AFF, 0xA8B878FF, 0x4C5E3CFF], 12, 100.0, 0.55, 2.4, 88);
            }
            EnemyKind::Bat => {
                spawn_burst(&mut self.particles, center, [0x5A4A6AFF, 0x8A7290FF, 0x342A42FF], 10, 80.0, 0.5, 2.0, 99);
            }
            EnemyKind::KingSlime => {
                self.boss_active = false;
                self.boss_defeated = true;
                self.flags.boss_defeated = true;
                self.victory_timer = 3.0;
                self.spawn_drop(pos.add(V2::new(12.0, 4.0)), ItemId::Trophy, 1);
                self.spawn_drop(pos.add(V2::new(-6.0, 4.0)), ItemId::Gel, 20);
                self.total_gel_earned += 20;
                spawn_burst(&mut self.particles, center, [0x5CC49EFF, 0xFFD75EFF, 0x9CFFB0FF], 46, 220.0, 1.1, 3.2, 123);
                self.shake = 14.0;
                self.flash = 0.9;
                self.flash_color = 0x9CFFB060;
                self.banner("THE KING IS DEAD", "The crown rolls into the dirt", 1.2, 0xFFD75E);
                gfx::sfx(SFX_VICTORY, 0.9);
            }
        }
        self.enemies[index].dead = true;
        self.enemies[index].death_timer = 0.0;
        self.save_timer = self.save_timer.min(8.0);
    }

    fn spawn_drop(&mut self, pos: V2, item: ItemId, count: u32) {
        let n = count.min(12);
        for i in 0..n {
            let off = V2::new((i as f32 - n as f32 * 0.5) * 4.0, -(i as f32) * 2.0);
            self.drops.push(Drop::new(pos.add(off), item, if count > 12 { (count / 12).max(1) } else { 1 }));
        }
    }

    fn break_tile(&mut self, x: i32, y: i32, allow_tree_fell: bool) {
        let t = self.world.get(x, y);
        if t == world::AIR || t == world::WATER || t == world::BEDROCK {
            return;
        }
        // Trees fall as a unit when any trunk block is cut.
        if allow_tree_fell && (t == world::WOOD || t == world::CACTUS) && self.tree_has_leaves(x, y) {
            self.fell_tree(x, y);
            return;
        }
        self.world.set_tile(x, y, world::AIR);
        self.stats.blocks_mined += 1;
        if let Some((item, count)) = world::tile_drop(t) {
            if self.inventory.has_room(item) {
                let left = self.inventory.add(item, count);
                if left > 0 {
                    self.spawn_drop(V2::new(x as f32 * TILE as f32 + 8.0, y as f32 * TILE as f32 + 8.0), item, left);
                }
                self.float_text(&format!("+{} {}", count, item.short()), V2::new(x as f32 * TILE as f32 + 8.0, y as f32 * TILE as f32), 0xF0E6C8, 4.6);
                gfx::sfx(SFX_PICKUP, 0.28);
            } else {
                self.spawn_drop(V2::new(x as f32 * TILE as f32 + 8.0, y as f32 * TILE as f32 + 8.0), item, count);
                self.toast("Inventory full", "Items dropped on the ground", 0xFFB070);
            }
        }
        let colors = tile_particle_colors(t);
        spawn_burst(
            &mut self.particles,
            V2::new(x as f32 * TILE as f32 + 8.0, y as f32 * TILE as f32 + 8.0),
            colors,
            9,
            80.0,
            0.55,
            2.2,
            (x as u32).wrapping_mul(31) ^ (y as u32),
        );
        gfx::sfx(SFX_BREAK, 0.5);
        self.shake = (self.shake + 0.7).min(4.0);
        if t == world::WOOD {
            self.flags.got_wood = true;
        }
        self.save_timer = self.save_timer.min(5.0);
    }

    fn tree_has_leaves(&self, x: i32, y: i32) -> bool {
        for dy in -8..=3 {
            for dx in -5..=5 {
                if self.world.get(x + dx, y + dy) == world::LEAVES {
                    return true;
                }
            }
        }
        false
    }

    fn fell_tree(&mut self, sx: i32, sy: i32) {
        let mut stack = vec![(sx, sy)];
        let mut seen = std::collections::HashSet::new();
        let mut blocks: Vec<(i32, i32)> = Vec::new();
        let mut leaves: Vec<(i32, i32)> = Vec::new();
        while let Some((x, y)) = stack.pop() {
            if blocks.len() + leaves.len() > 220 || seen.contains(&(x, y)) {
                continue;
            }
            let t = self.world.get(x, y);
            if t != world::WOOD && t != world::CACTUS && t != world::LEAVES {
                continue;
            }
            seen.insert((x, y));
            if t == world::LEAVES {
                leaves.push((x, y));
                continue;
            }
            blocks.push((x, y));
            for (nx, ny) in [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1), (x + 1, y - 1), (x - 1, y - 1)] {
                stack.push((nx, ny));
            }
        }
        if blocks.len() > 90 {
            // Avoid felling huge player-built structures by accident.
            blocks.truncate(90);
        }
        let mut wood = 0u32;
        for (x, y) in blocks.iter().copied() {
            self.world.set_tile(x, y, world::AIR);
            self.stats.blocks_mined += 1;
            wood += 1;
            if self.rng.chance(0.4) {
                spawn_burst(
                    &mut self.particles,
                    V2::new(x as f32 * TILE as f32 + 8.0, y as f32 * TILE as f32 + 8.0),
                    tile_particle_colors(world::WOOD),
                    4,
                    80.0,
                    0.5,
                    2.0,
                    (x as u32) ^ (y as u32) ^ 0xAA,
                );
            }
        }
        for (x, y) in leaves.iter().copied() {
            self.world.set_tile(x, y, world::AIR);
            if self.rng.chance(0.28) {
                self.particles.push(
                    Particle::new(
                        V2::new(x as f32 * TILE as f32 + 8.0, y as f32 * TILE as f32 + 8.0),
                        V2::new(self.rng.range(-30.0, 30.0), self.rng.range(-30.0, 5.0)),
                        1.0,
                        2.0,
                        tile_particle_colors(world::LEAVES)[0],
                    )
                    .gravity(120.0)
                    .drag(1.2)
                    .leaf(),
                );
            }
        }
        if wood > 0 {
            let item = ItemId::Wood;
            if self.inventory.has_room(item) {
                self.inventory.add(item, wood);
                self.float_text(&format!("+{} Wood", wood), V2::new(sx as f32 * TILE as f32 + 8.0, sy as f32 * TILE as f32), 0xF0E6C8, 5.5);
            } else {
                self.spawn_drop(V2::new(sx as f32 * TILE as f32 + 8.0, sy as f32 * TILE as f32), item, wood);
            }
            self.flags.got_wood = true;
        }
        gfx::sfx(SFX_BREAK, 0.7);
        self.shake = (self.shake + 2.0).min(6.0);
        self.save_timer = self.save_timer.min(5.0);
    }

    fn update_enemies(&mut self, dt: f32) {
        let pc = self.player.center();
        let night = self.is_night();
        let aggression = true;
        for i in 0..self.enemies.len() {
            let kind = self.enemies[i].kind;
            if self.enemies[i].dead {
                self.enemies[i].death_timer += dt;
                continue;
            }
            self.enemies[i].update(&self.world, pc, dt, aggression);
            if kind == EnemyKind::Zombie && !night && self.player.pos.y < self.world.surface_px(self.player.pos.x as i32) + 120.0 {
                // Zombies crumble in daylight.
                if self.rng.chance(dt * 0.8) {
                    self.enemies[i].hurt(999.0, 0.0, pc.x);
                    self.kill_enemy(i);
                }
            }
            // Contact damage.
            let (ex, ey, ew, eh) = self.enemies[i].aabb();
            let (px, py, pw, ph) = self.player.aabb();
            if aabb_overlap(ex, ey, ew, eh, px, py, pw, ph) && self.enemies[i].contact_cd <= 0.0 {
                let dmg = kind.contact_damage() * if kind.is_boss() { 1.0 } else { 1.0 };
                if self.player.hurt(dmg, self.enemies[i].center().x) {
                    self.enemies[i].contact_cd = 0.85;
                    self.shake = (self.shake + 3.5).min(9.0);
                    gfx::sfx(SFX_HURT, 0.85);
                    self.hurt_vignette = 1.0;
                    self.float_text(&format!("-{}", dmg.round()), self.player.center().add(V2::new(0.0, -18.0)), 0xFF7070, 6.0);
                }
            }
            // Boss minion waves.
            if kind == EnemyKind::KingSlime {
                let hp_frac = self.enemies[i].hp / self.enemies[i].max_hp;
                let wave = if hp_frac < 0.34 {
                    2
                } else if hp_frac < 0.68 {
                    1
                } else {
                    0
                };
                if wave > self.boss_wave_flags {
                    self.boss_wave_flags = wave;
                    let cx = self.enemies[i].center().x;
                    let cy = self.enemies[i].center().y;
                    for k in 0..(2 + wave) {
                        let ox = -60.0 + k as f32 * 40.0;
                        let mut e = Enemy::new(EnemyKind::Slime, V2::new(cx + ox, cy - 10.0));
                        e.vel = V2::new(ox.signum() * 90.0, -240.0);
                        self.enemies.push(e);
                    }
                    self.toast("The King calls for help", "Slimes join the fight", 0x8CFFB0);
                    gfx::sfx(SFX_BOSS, 0.6);
                }
            }
        }
        self.enemies.retain(|e| !(e.dead && e.death_timer > 0.8));
        // Despawn far-away enemies (but never the boss).
        let pc = self.player.center();
        self.enemies.retain(|e| {
            if e.kind.is_boss() {
                true
            } else {
                let d = dist(e.center(), pc);
                d < 1500.0 && !(e.kind == EnemyKind::Zombie && !night && self.player.pos.y < self.world.surface_px(self.player.pos.x as i32) + 80.0 && e.dead)
            }
        });
    }

    fn update_bombs(&mut self, dt: f32) {
        let mut explosions: Vec<V2> = Vec::new();
        for b in self.bombs.iter_mut() {
            if b.update(&self.world, dt) {
                explosions.push(b.pos);
            }
        }
        self.bombs.retain(|b| b.fuse > 0.0);
        for pos in explosions {
            self.explode(pos);
        }
    }

    fn explode(&mut self, pos: V2) {
        gfx::sfx(SFX_EXPLODE, 1.0);
        self.shake = 13.0;
        self.flash = 0.55;
        self.flash_color = 0xFFD08040;
        let cx = (pos.x / TILE as f32) as i32;
        let cy = (pos.y / TILE as f32) as i32;
        let radius = 3;
        for y in (cy - radius)..=(cy + radius) {
            for x in (cx - radius)..=(cx + radius) {
                let d = ((x - cx).pow(2) + (y - cy).pow(2)) as f32;
                if d <= (radius * radius) as f32 && self.world.get(x, y) != world::BEDROCK {
                    let t = self.world.get(x, y);
                    if t != world::AIR && t != world::WATER {
                        if let Some((item, count)) = world::tile_drop(t) {
                            if self.inventory.has_room(item) {
                                self.inventory.add(item, count);
                            }
                        }
                        self.world.set_tile(x, y, world::AIR);
                        self.stats.blocks_mined += 1;
                    }
                }
            }
        }
        for i in 0..42 {
            let h = hash2(i, 3, self.seed);
            let a = h * std::f32::consts::TAU;
            let s = 60.0 + hash2(i, 8, self.seed) * 220.0;
            let color = if i % 3 == 0 { 0xFFE28AFF } else if i % 3 == 1 { 0xFF9A4AFF } else { 0x505050C8 };
            let mut p = Particle::new(
                pos,
                V2::new(a.cos() * s, a.sin() * s - 40.0),
                0.4 + h * 0.7,
                2.0 + h * 3.0,
                color,
            );
            p.gravity = 160.0;
            p.drag = 1.6;
            if i % 4 == 0 {
                p = p.flame();
            } else if i % 4 == 1 {
                p = p.smoke();
            }
            self.particles.push(p);
        }
        for i in 0..self.enemies.len() {
            if self.enemies[i].dead {
                continue;
            }
            let d = dist(self.enemies[i].center(), pos);
            if d < radius as f32 * TILE as f32 * 1.3 {
                let dmg = 95.0 * (1.0 - d / (radius as f32 * TILE as f32 * 1.6)).max(0.25);
                let killed = self.enemies[i].hurt(dmg, 240.0, pos.x);
                self.stats_damage_text(dmg, self.enemies[i].center().x, self.enemies[i].center().y);
                if killed {
                    self.kill_enemy(i);
                }
            }
        }
        let pd = dist(self.player.center(), pos);
        if pd < 64.0 {
            if self.player.hurt(28.0 * (1.0 - pd / 80.0).max(0.2), pos.x) {
                gfx::sfx(SFX_HURT, 0.9);
                self.hurt_vignette = 1.0;
            }
        }
        self.save_timer = self.save_timer.min(5.0);
    }

    fn update_drops(&mut self, dt: f32) {
        let pc = self.player.center();
        let mut collected: Vec<usize> = Vec::new();
        for (i, d) in self.drops.iter_mut().enumerate() {
            if d.update(&self.world, pc, dt) {
                collected.push(i);
            }
        }
        for i in collected.into_iter().rev() {
            let d = self.drops.remove(i);
            if self.inventory.has_room(d.item) {
                self.inventory.add(d.item, d.count);
                gfx::sfx(SFX_PICKUP, 0.35);
                self.float_text(&format!("+{} {}", d.count, d.item.short()), d.pos, 0xF0E8C8, 4.5);
            } else {
                // Put it back; the player must make room.
                self.drops.push(d);
            }
        }
        self.drops.retain(|d| d.life > 0.0);
        if self.drops.len() > 80 {
            self.drops.drain(0..self.drops.len() - 80);
        }
    }

    fn update_particles(&mut self, dt: f32) {
        for p in self.particles.iter_mut() {
            p.update(dt);
        }
        self.particles.retain(|p| p.life > 0.0);
        if self.particles.len() > 520 {
            let excess = self.particles.len() - 520;
            self.particles.drain(0..excess);
        }
        for t in self.texts.iter_mut() {
            t.update(dt);
        }
        self.texts.retain(|t| t.life > 0.0);
    }

    fn update_ambient(&mut self, dt: f32) {
        self.ambient_timer -= dt;
        if self.ambient_timer > 0.0 {
            return;
        }
        self.ambient_timer = if self.is_night() { 0.18 } else { 0.42 };
        self.spawn_ambient();
    }

    fn spawn_ambient(&mut self) {
        if self.particles.len() > 400 {
            return;
        }
        let pc = self.player.center();
        let night = self.is_night();
        let surface_y = self.world.surface_px(pc.x as i32);
        let underground = pc.y > surface_y + 60.0;
        if night && !underground {
            // Fireflies drift over the grass.
            let x = pc.x + self.rng.range(-self.view.x * 0.55, self.view.x * 0.55);
            let y = self.world.surface_px(x as i32) - self.rng.range(8.0, 60.0);
            if y > 0.0 {
                self.particles.push(
                    Particle::new(
                        V2::new(x, y),
                        V2::new(self.rng.range(-8.0, 8.0), self.rng.range(-9.0, 2.0)),
                        2.2,
                        1.4,
                        0xB8FF7ACC,
                    )
                    .gravity(-2.0)
                    .drag(0.1)
                    .circle(),
                );
            }
        } else if !underground && self.rng.chance(0.55) {
            // Falling leaves.
            let x = pc.x + self.rng.range(-self.view.x * 0.6, self.view.x * 0.6);
            let y = self.camera.y + self.rng.range(-20.0, 60.0);
            self.particles.push(
                Particle::new(
                    V2::new(x, y),
                    V2::new(self.rng.range(-16.0, 16.0), self.rng.range(8.0, 22.0)),
                    2.8,
                    2.0,
                    if self.rng.chance(0.5) { 0x8AC46ACC } else { 0xB8D878CC },
                )
                .gravity(2.0)
                .drag(0.2)
                .leaf(),
            );
        } else if underground && self.rng.chance(0.45) {
            // Dust motes in the caves.
            let x = pc.x + self.rng.range(-self.view.x * 0.55, self.view.x * 0.55);
            let y = pc.y + self.rng.range(-self.view.y * 0.5, self.view.y * 0.5);
            self.particles.push(
                Particle::new(
                    V2::new(x, y),
                    V2::new(self.rng.range(-4.0, 4.0), self.rng.range(-8.0, -1.0)),
                    2.5,
                    1.0,
                    0x8A90B080,
                )
                .gravity(0.0)
                .drag(0.0)
                .circle(),
            );
        }
    }

    fn spawn_logic(&mut self, dt: f32) {
        self.spawn_timer -= dt;
        if self.spawn_timer > 0.0 {
            return;
        }
        let night = self.is_night();
        let cap = if night { 10 } else { 7 };
        let alive = self.enemies.iter().filter(|e| !e.dead && !e.kind.is_boss()).count();
        if alive >= cap || self.boss_active {
            self.spawn_timer = 2.0;
            return;
        }
        self.spawn_timer = if night { 1.6 + self.rng.range(0.0, 1.4) } else { 2.6 + self.rng.range(0.0, 2.2) };
        let pc = self.player.center();
        let surface_y = self.world.surface_px(pc.x as i32);
        let underground = pc.y > surface_y + 90.0;
        let side = if self.rng.chance(0.5) { -1.0 } else { 1.0 };
        let spawn_x = pc.x + side * (self.view.x * 0.55 + self.rng.range(70.0, 260.0));
        let sx = clamp_i((spawn_x / TILE as f32) as i32, 4, world::W - 5);

        if underground {
            // Try to find a cave pocket around the player.
            for _ in 0..14 {
                let tx = clamp_i((pc.x / TILE as f32) as i32 + self.rng.range_i(-32, 32), 3, world::W - 4);
                let ty = clamp_i((pc.y / TILE as f32) as i32 + self.rng.range_i(-18, 18), 3, world::H - 4);
                if self.world.get(tx, ty) != world::AIR || self.world.get(tx, ty - 1) != world::AIR {
                    continue;
                }
                let floor = self.world.solid(tx, ty + 1);
                let kind = if floor && self.rng.chance(0.35) {
                    EnemyKind::Zombie
                } else {
                    EnemyKind::Bat
                };
                let (w, h) = kind.size();
                let e = Enemy::new(kind, V2::new(tx as f32 * TILE as f32 + (TILE as f32 - w) * 0.5, ty as f32 * TILE as f32 + TILE as f32 - h));
                self.enemies.push(e);
                return;
            }
        } else {
            let top = self.world.surface_px(sx);
            let mut ty = (top / TILE as f32) as i32 - 1;
            // Move up until there is space.
            let mut guard = 0;
            while guard < 6 && self.world.solid(sx, ty) {
                ty -= 1;
                guard += 1;
            }
            if self.world.water(sx, ty) || self.world.solid(sx, ty) || self.world.solid(sx, ty - 1) {
                return;
            }
            let kind = if night && self.rng.chance(0.62) { EnemyKind::Zombie } else { EnemyKind::Slime };
            let (w, h) = kind.size();
            let e = Enemy::new(
                kind,
                V2::new(sx as f32 * TILE as f32 + (TILE as f32 - w) * 0.5, ty as f32 * TILE as f32 + TILE as f32 - h),
            );
            self.enemies.push(e);
        }
    }

    // ------------------------------------------------------------------
    // UI interaction
    // ------------------------------------------------------------------

    fn handle_inventory_ui(&mut self, input: &Input) {
        let layout = ui::inventory_layout(self);
        if input.press(KEY_E) || input.press(KEY_C) || input.press(KEY_ESC) {
            self.inv_open = false;
            return;
        }
        self.hover_inv_slot = None;
        for (i, r) in layout.slots.iter().enumerate() {
            if r.contains(input.mouse) {
                self.hover_inv_slot = Some(i);
                if input.mouse_pressed {
                    if i < HOTBAR {
                        self.inventory.selected = i;
                    } else {
                        self.inventory.slots.swap(i, self.inventory.selected);
                        gfx::sfx(SFX_CLICK, 0.4);
                    }
                }
            }
        }
        self.hover_recipe = None;
        for (i, r) in layout.recipes.iter().enumerate() {
            if r.contains(input.mouse) {
                self.hover_recipe = Some(i);
                if input.mouse_pressed {
                    self.craft(i);
                }
            }
        }
        if input.mouse_pressed && layout.close.contains(input.mouse) {
            self.inv_open = false;
            gfx::sfx(SFX_CLICK, 0.4);
        }
    }

    fn handle_pause_ui(&mut self, input: &Input) {
        let layout = ui::pause_layout(self);
        if input.mouse_pressed {
            if layout.resume.contains(input.mouse) {
                self.mode = Mode::Playing;
                gfx::sfx(SFX_CLICK, 0.4);
            } else if layout.mute.contains(input.mouse) {
                self.toggle_mute();
            } else if layout.quit.contains(input.mouse) {
                self.save_game();
                self.mode = Mode::Attract;
                gfx::show_title();
            } else if layout.help.contains(input.mouse) {
                self.show_help = !self.show_help;
                gfx::sfx(SFX_CLICK, 0.35);
            }
        }
        if input.press(KEY_E) || input.press(KEY_C) {
            self.inv_open = true;
            self.mode = Mode::Playing;
        }
        if input.press(KEY_M) {
            self.map_open = true;
            self.mode = Mode::Playing;
        }
    }

    pub fn craft(&mut self, index: usize) {
        let Some(recipe) = RECIPES.get(index) else { return };
        let bench = self.world.bench_near(self.player.center().x, self.player.center().y, 6);
        if recipe.bench && !bench {
            self.toast("Workbench required", "Place a workbench nearby to craft this", 0xFFB070);
            gfx::sfx(SFX_CLICK, 0.3);
            return;
        }
        for input in recipe.inputs.iter().flatten() {
            if self.inventory.count(input.0) < input.1 {
                self.toast("Not enough materials", recipe.hint, 0xFFB070);
                gfx::sfx(SFX_CLICK, 0.3);
                return;
            }
        }
        for input in recipe.inputs.iter().flatten() {
            self.inventory.consume(input.0, input.1);
        }
        let left = self.inventory.add(recipe.output, recipe.out_count);
        if left > 0 {
            self.spawn_drop(
                self.player.center(),
                recipe.output,
                left,
            );
        }
        if recipe.output == ItemId::Bench {
            self.flags.crafted_bench = true;
        }
        if recipe.output.is_sword() && recipe.output != ItemId::RustyBlade {
            self.flags.crafted_blade = true;
        }
        self.toast(
            &format!("Crafted {}", recipe.name),
            &format!("{} x{}", recipe.output.name(), recipe.out_count),
            recipe.output.color(),
        );
        self.float_text(&format!("+{} {}", recipe.out_count, recipe.output.short()), self.player.center().add(V2::new(0.0, -22.0)), recipe.output.color(), 5.5);
        gfx::sfx(SFX_CRAFT, 0.7);
        self.particles.push(
            Particle::new(self.player.center(), V2::new(0.0, -30.0), 0.6, 2.2, recipe.output.color()).spark(),
        );
        self.save_timer = self.save_timer.min(5.0);
    }

    pub fn toast(&mut self, text: &str, sub: &str, color: u32) {
        self.toasts.push(Toast {
            text: text.to_string(),
            sub: sub.to_string(),
            life: 3.4,
            max_life: 3.4,
            color,
        });
        if self.toasts.len() > 4 {
            self.toasts.remove(0);
        }
    }

    pub fn banner(&mut self, text: &str, sub: &str, time: f32, color: u32) {
        self.banner = Some(Banner {
            text: text.to_string(),
            sub: sub.to_string(),
            life: time,
            max_life: time,
            color,
        });
    }

    pub fn float_text(&mut self, text: &str, pos: V2, color: u32, size: f32) {
        self.texts.push(FloatingText::new(text, pos, color, size));
        if self.texts.len() > 40 {
            self.texts.remove(0);
        }
    }

    // ------------------------------------------------------------------
    // Persistence
    // ------------------------------------------------------------------

    pub fn save_game(&mut self) {
        let mut s = String::new();
        s.push_str(SAVE_MAGIC);
        s.push('\n');
        s.push_str(&format!("seed={}\n", self.seed));
        s.push_str(&format!("time={:.4}\n", self.time));
        s.push_str(&format!("day={}\n", self.day));
        s.push_str(&format!(
            "player={:.2},{:.2},{:.2}\n",
            self.player.pos.x, self.player.pos.y, self.player.hp
        ));
        s.push_str(&format!(
            "flags={},{},{},{},{},{}\n",
            self.flags.got_wood as u8,
            self.flags.crafted_bench as u8,
            self.flags.placed_bench as u8,
            self.flags.crafted_blade as u8,
            self.flags.summoned_boss as u8,
            self.flags.boss_defeated as u8
        ));
        s.push_str(&format!(
            "stats={},{},{},{},{:.1}\n",
            self.stats.blocks_mined,
            self.stats.blocks_placed,
            self.stats.enemies_killed,
            self.stats.deaths,
            self.stats.play_time
        ));
        s.push_str(&format!("sel={}\n", self.inventory.selected));
        s.push_str("inv=");
        let mut first = true;
        for (i, slot) in self.inventory.slots.iter().enumerate() {
            if let Some(st) = slot {
                if !first {
                    s.push(';');
                }
                first = false;
                // slot:id:count keeps every item in its original position.
                s.push_str(&format!("{}:{}:{}", i, st.id as u8, st.count));
            }
        }
        s.push('\n');
        s.push_str("mods=");
        for (i, (x, y, t)) in self.world.modified.iter().enumerate() {
            if i > 0 {
                s.push(';');
            }
            s.push_str(&format!("{x},{y},{t}"));
        }
        s.push('\n');
        gfx::save(s.as_bytes());
        self.has_save = true;
        self.saved = Some(SaveData::from_game(self));
    }

    fn load_from_storage(&mut self) {
        let len = gfx::load_len();
        if len == 0 || len > 4_000_000 {
            return;
        }
        let mut buf = vec![0u8; len as usize];
        let n = gfx::load_read(&mut buf);
        buf.truncate(n as usize);
        let Ok(text) = String::from_utf8(buf) else { return };
        if let Some(save) = SaveData::parse(&text) {
            self.has_save = true;
            self.saved = Some(save);
        }
    }

    fn apply_save(&mut self, save: SaveData) {
        self.seed = save.seed;
        self.world = World::generate(save.seed);
        for &(x, y, t) in &save.mods {
            self.world.set_tile(x, y, t);
        }
        // set_tile records each restored edit in `modified`; replace that list
        // with the saved one so it is neither duplicated nor lost.
        self.world.modified = save.mods;
        self.time = save.time;
        self.day = save.day;
        self.flags = save.flags;
        if self.flags.summoned_boss && !self.flags.boss_defeated {
            // Boss fights are not persisted; let the player summon again.
            self.flags.summoned_boss = false;
        }
        self.stats = save.stats;
        self.inventory = Inventory::new();
        self.inventory.slots = vec![None; INV_SLOTS];
        for &(slot, id, count) in &save.inv {
            if let Some(id) = ItemId::from_u8(id) {
                let slot = (slot as usize).min(INV_SLOTS - 1);
                self.inventory.slots[slot] = Some(ItemStack { id, count });
            }
        }
        self.inventory.selected = save.selected.min(HOTBAR - 1);
        self.player = Player::new(self.world.spawn);
        self.player.pos = save.player_pos;
        self.player.hp = save.hp.clamp(1.0, self.player.max_hp);
        self.player.respawn = save.player_pos;
        self.boss_defeated = save.flags.boss_defeated;
        self.boss_active = false;
        // A completed save must not replay the victory screen on load.
        self.victory_timer = if self.boss_defeated { f32::MAX } else { 0.0 };
        self.enemies.clear();
        self.drops.clear();
        self.bombs.clear();
        self.particles.clear();
        self.texts.clear();
        self.mining = None;
        self.death_timer = 0.0;
        self.boss_wave_flags = 0;
        self.boss_intro = 0.0;
        self.hurt_vignette = 0.0;
        self.center_camera();
        self.was_night = self.is_night();
        gfx::minimap(self.world.minimap.as_ptr(), world::W as u32, world::H as u32);
        self.world.map_dirty = false;
    }

    pub fn render(&self) {
        render::draw(self);
        ui::draw(self);
    }

    pub fn debug_json(&self) -> String {
        let (ox, oy) = (self.player.pos.x, self.player.pos.y);
        let mtx = (self.mouse_world.x / TILE as f32).floor() as i32;
        let mty = (self.mouse_world.y / TILE as f32).floor() as i32;
        let mtile = self.world.get(mtx, mty);
        let boss_hp = self
            .enemies
            .iter()
            .find(|e| e.kind.is_boss() && !e.dead)
            .map(|e| e.hp)
            .unwrap_or(0.0);
        let inv: Vec<String> = self
            .inventory
            .slots
            .iter()
            .flatten()
            .map(|s| format!("{{\"id\":\"{}\",\"count\":{}}}", s.id.name(), s.count))
            .collect();
        format!(
            "{{\"mode\":\"{:?}\",\"x\":{:.1},\"y\":{:.1},\"vx\":{:.1},\"vy\":{:.1},\"hp\":{:.1},\"onGround\":{},\"time\":{:.3},\"day\":{},\"night\":{},\"enemies\":{},\"drops\":{},\"selected\":{},\"inv\":[{}],\"boss\":{},\"bossDefeated\":{},\"mined\":{},\"kills\":{},\"bossHp\":{:.0},\"view\":[{:.0},{:.0}],\"seed\":{},\"flags\":{{\"wood\":{},\"bench\":{},\"placed\":{},\"blade\":{},\"summoned\":{}}},\"cam\":[{:.1},{:.1}],\"mouse\":[{:.1},{:.1}],\"tile\":[{},{},{}]}}",
            self.mode,
            ox,
            oy,
            self.player.vel.x,
            self.player.vel.y,
            self.player.hp,
            self.player.on_ground,
            self.time,
            self.day,
            self.is_night(),
            self.enemies.len(),
            self.drops.len(),
            self.inventory.selected,
            inv.join(","),
            self.boss_active,
            self.boss_defeated,
            self.stats.blocks_mined,
            self.stats.enemies_killed,
            boss_hp,
            self.view.x,
            self.view.y,
            self.seed,
            self.flags.got_wood,
            self.flags.crafted_bench,
            self.flags.placed_bench,
            self.flags.crafted_blade,
            self.flags.summoned_boss,
            self.camera.x,
            self.camera.y,
            self.mouse_world.x,
            self.mouse_world.y,
            mtx,
            mty,
            mtile,
        )
    }
}

// ---------------------------------------------------------------------------
// Save data
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct SaveData {
    pub seed: u32,
    pub time: f32,
    pub day: u32,
    pub player_pos: V2,
    pub hp: f32,
    pub flags: QuestFlags,
    pub stats: Stats,
    /// (slot, item id, count)
    pub inv: Vec<(u8, u8, u32)>,
    pub selected: usize,
    pub mods: Vec<(i32, i32, u8)>,
}

impl SaveData {
    fn parse(text: &str) -> Option<SaveData> {
        let mut lines = text.lines();
        if lines.next()? != SAVE_MAGIC {
            return None;
        }
        let mut save = SaveData {
            seed: 1337,
            time: 0.3,
            day: 1,
            player_pos: V2::new(0.0, 0.0),
            hp: 100.0,
            flags: QuestFlags::default(),
            stats: Stats::default(),
            inv: Vec::new(),
            selected: 0,
            mods: Vec::new(),
        };
        let mut has_seed = false;
        let mut has_player = false;
        let mut next_legacy_slot: u8 = 0;
        for line in lines {
            if let Some(v) = line.strip_prefix("seed=") {
                save.seed = v.parse().ok()?;
                has_seed = true;
            } else if let Some(v) = line.strip_prefix("time=") {
                save.time = v.parse().unwrap_or(0.3);
            } else if let Some(v) = line.strip_prefix("day=") {
                save.day = v.parse().unwrap_or(1);
            } else if let Some(v) = line.strip_prefix("player=") {
                let p: Vec<&str> = v.split(',').collect();
                if p.len() >= 3 {
                    save.player_pos = V2::new(p[0].parse().ok()?, p[1].parse().ok()?);
                    save.hp = p[2].parse().unwrap_or(100.0);
                    has_player = true;
                }
            } else if let Some(v) = line.strip_prefix("flags=") {
                let f: Vec<u8> = v.split(',').filter_map(|x| x.parse().ok()).collect();
                if f.len() >= 6 {
                    save.flags = QuestFlags {
                        got_wood: f[0] != 0,
                        crafted_bench: f[1] != 0,
                        placed_bench: f[2] != 0,
                        crafted_blade: f[3] != 0,
                        summoned_boss: f[4] != 0,
                        boss_defeated: f[5] != 0,
                    };
                }
            } else if let Some(v) = line.strip_prefix("stats=") {
                let s: Vec<&str> = v.split(',').collect();
                if s.len() >= 5 {
                    save.stats = Stats {
                        blocks_mined: s[0].parse().unwrap_or(0),
                        blocks_placed: s[1].parse().unwrap_or(0),
                        enemies_killed: s[2].parse().unwrap_or(0),
                        deaths: s[3].parse().unwrap_or(0),
                        play_time: s[4].parse().unwrap_or(0.0),
                    };
                }
            } else if let Some(v) = line.strip_prefix("sel=") {
                save.selected = v.parse().unwrap_or(0);
            } else if let Some(v) = line.strip_prefix("inv=") {
                for part in v.split(';') {
                    if part.is_empty() {
                        continue;
                    }
                    let kv: Vec<&str> = part.split(':').collect();
                    if kv.len() == 3 {
                        // New format: slot:id:count
                        if let (Ok(slot), Ok(id), Ok(count)) = (
                            kv[0].parse::<u8>(),
                            kv[1].parse::<u8>(),
                            kv[2].parse::<u32>(),
                        ) {
                            save.inv.push((slot, id, count));
                            next_legacy_slot = slot.saturating_add(1);
                        }
                    } else if kv.len() == 2 {
                        // Legacy format: id:count, packed in order.
                        if let (Ok(id), Ok(count)) = (kv[0].parse::<u8>(), kv[1].parse::<u32>()) {
                            save.inv.push((next_legacy_slot, id, count));
                            next_legacy_slot = next_legacy_slot.saturating_add(1);
                        }
                    }
                }
                if save.inv.iter().any(|&(slot, _, _)| slot as usize >= INV_SLOTS) {
                    save.inv.retain(|&(slot, _, _)| (slot as usize) < INV_SLOTS);
                    for (i, entry) in save.inv.iter_mut().enumerate() {
                        entry.0 = i as u8;
                    }
                }
            } else if let Some(v) = line.strip_prefix("mods=") {
                for part in v.split(';') {
                    if part.is_empty() {
                        continue;
                    }
                    let nums: Vec<&str> = part.split(',').collect();
                    if nums.len() == 3 {
                        if let (Ok(x), Ok(y), Ok(t)) =
                            (nums[0].parse::<i32>(), nums[1].parse::<i32>(), nums[2].parse::<u8>())
                        {
                            if World::in_bounds(x, y) {
                                save.mods.push((x, y, t));
                            }
                        }
                    }
                }
            }
        }
        if !has_seed {
            return None;
        }
        if !has_player {
            save.player_pos = V2::new(world::W as f32 * TILE as f32 * 0.5, 100.0);
        }
        Some(save)
    }

    fn from_game(game: &Game) -> SaveData {
        let inv = game
            .inventory
            .slots
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.as_ref().map(|s| (i as u8, s.id as u8, s.count)))
            .collect();
        SaveData {
            seed: game.seed,
            time: game.time,
            day: game.day,
            player_pos: game.player.pos,
            hp: game.player.hp,
            flags: game.flags.clone(),
            stats: game.stats.clone(),
            inv,
            selected: game.inventory.selected,
            mods: game.world.modified.clone(),
        }
    }
}
