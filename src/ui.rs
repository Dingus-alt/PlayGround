//! HUD, menus, inventory/crafting and the world map.

use crate::game::{Game, Mode, RECIPES};
use crate::gfx::{self, alpha, mix, rgb, rgba};
use crate::items::{ItemId, ItemStack, HOTBAR, INV_SLOTS};
use crate::math::{clamp, V2};
use crate::render::draw_item_icon;
use crate::world::{self, TILE};

#[derive(Clone, Copy, Debug, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Rect { x, y, w, h }
    }
    pub fn contains(&self, p: V2) -> bool {
        p.x >= self.x && p.y >= self.y && p.x <= self.x + self.w && p.y <= self.y + self.h
    }
    pub fn center(&self) -> V2 {
        V2::new(self.x + self.w * 0.5, self.y + self.h * 0.5)
    }
}

pub fn ui_scale(game: &Game) -> f32 {
    clamp(game.view.y / 420.0, 0.82, 1.5)
}

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

pub struct HotbarLayout {
    pub slots: Vec<Rect>,
    pub slot: f32,
    pub x: f32,
    pub y: f32,
    pub s: f32,
}

impl HotbarLayout {
    pub fn slot_at(&self, p: V2) -> Option<usize> {
        self.slots.iter().position(|r| r.contains(p))
    }
    pub fn contains(&self, p: V2) -> bool {
        self.slots.iter().any(|r| r.contains(p))
    }
}

pub fn hotbar_layout(game: &Game) -> HotbarLayout {
    let s = ui_scale(game);
    let slot = 33.0 * s;
    let gap = 3.0 * s;
    let portrait = game.view.x < 520.0;
    let cols = if portrait && game.touch_mode { 5 } else if portrait { 10 } else { 10 };
    let cols = if game.touch_mode && game.view.x < 560.0 { 5 } else { cols };
    let rows = (HOTBAR + cols - 1) / cols;
    let total_w = cols as f32 * slot + (cols - 1) as f32 * gap;
    let total_h = rows as f32 * slot + (rows - 1) as f32 * gap;
    let x = (game.view.x - total_w) * 0.5;
    let bottom = if game.touch_mode { 105.0 } else { 20.0 * s };
    let y = game.view.y - bottom - total_h;
    let mut slots = Vec::new();
    for i in 0..HOTBAR {
        let r = i / cols;
        let c = i % cols;
        slots.push(Rect::new(x + c as f32 * (slot + gap), y + r as f32 * (slot + gap), slot, slot));
    }
    HotbarLayout { slots, slot, x, y, s }
}

pub struct InventoryLayout {
    pub panel: Rect,
    pub close: Rect,
    pub slots: Vec<Rect>,
    pub recipes: Vec<Rect>,
    pub s: f32,
}

pub fn inventory_layout(game: &Game) -> InventoryLayout {
    let s = ui_scale(game);
    let panel_w = (game.view.x - 18.0 * s).min(610.0 * s).max(250.0);
    let pad = 10.0 * s;
    let gap = 3.0 * s;
    let slot = ((panel_w - pad * 2.0 - gap * 9.0) / 10.0).min(33.0 * s);
    let grid_w = slot * 10.0 + gap * 9.0;
    let header = 30.0 * s;
    let grid_h = slot * 3.0 + gap * 2.0;
    let two_cols = panel_w > 460.0 * s;
    let recipe_rows = if two_cols { 5 } else { 9 };
    let recipe_h = 25.0 * s;
    let footer = 26.0 * s;
    let panel_h = (header + grid_h + 14.0 * s + recipe_rows as f32 * recipe_h + footer)
        .min(game.view.y - 12.0 * s);
    let panel = Rect::new(
        (game.view.x - panel_w) * 0.5,
        (game.view.y - panel_h) * 0.5,
        panel_w,
        panel_h,
    );
    let close = Rect::new(panel.x + panel.w - 27.0 * s, panel.y + 6.0 * s, 20.0 * s, 20.0 * s);
    let gx = panel.x + (panel.w - grid_w) * 0.5;
    let gy = panel.y + header + 6.0 * s;
    let mut slots = Vec::new();
    for i in 0..INV_SLOTS {
        let r = i / 10;
        let c = i % 10;
        slots.push(Rect::new(
            gx + c as f32 * (slot + gap),
            gy + r as f32 * (slot + gap),
            slot,
            slot,
        ));
    }
    let recipe_top = gy + grid_h + 10.0 * s;
    let recipe_area_w = panel.w - pad * 2.0;
    let mut recipes = Vec::new();
    if two_cols {
        let col_w = (recipe_area_w - gap) * 0.5;
        for i in 0..9 {
            let c = i / 5;
            let r = i % 5;
            recipes.push(Rect::new(
                panel.x + pad + c as f32 * (col_w + gap),
                recipe_top + r as f32 * recipe_h,
                col_w,
                recipe_h - 2.0 * s,
            ));
        }
    } else {
        for i in 0..9 {
            recipes.push(Rect::new(
                panel.x + pad,
                recipe_top + i as f32 * recipe_h,
                recipe_area_w,
                recipe_h - 2.0 * s,
            ));
        }
    }
    InventoryLayout { panel, close, slots, recipes, s }
}

pub struct PauseLayout {
    pub panel: Rect,
    pub resume: Rect,
    pub help: Rect,
    pub mute: Rect,
    pub quit: Rect,
    pub s: f32,
}

pub fn pause_layout(game: &Game) -> PauseLayout {
    let s = ui_scale(game);
    let w = (game.view.x - 24.0 * s).min(430.0 * s).max(250.0);
    let h = (game.view.y - 24.0 * s).min(330.0 * s).max(250.0);
    let panel = Rect::new((game.view.x - w) * 0.5, (game.view.y - h) * 0.5, w, h);
    let bw = w - 30.0 * s;
    let bh = 26.0 * s;
    let bx = panel.x + 15.0 * s;
    let by = panel.y + h - 15.0 * s - bh * 4.0 - 7.0 * s * 3.0;
    PauseLayout {
        panel,
        resume: Rect::new(bx, by, bw, bh),
        help: Rect::new(bx, by + (bh + 7.0 * s), bw, bh),
        mute: Rect::new(bx, by + (bh + 7.0 * s) * 2.0, bw, bh),
        quit: Rect::new(bx, by + (bh + 7.0 * s) * 3.0, bw, bh),
        s,
    }
}

pub struct DeathLayout {
    pub panel: Rect,
    pub respawn: Rect,
    pub quit: Rect,
    pub s: f32,
}

pub fn death_layout(game: &Game) -> DeathLayout {
    let s = ui_scale(game);
    let w = (game.view.x - 40.0 * s).min(360.0 * s).max(240.0);
    let h = (game.view.y - 40.0 * s).min(210.0 * s).max(190.0);
    let panel = Rect::new((game.view.x - w) * 0.5, (game.view.y - h) * 0.5, w, h);
    let bw = w - 30.0 * s;
    let bh = 27.0 * s;
    DeathLayout {
        panel,
        respawn: Rect::new(panel.x + 15.0 * s, panel.y + h - 15.0 * s - bh * 2.0 - 7.0 * s, bw, bh),
        quit: Rect::new(panel.x + 15.0 * s, panel.y + h - 15.0 * s - bh, bw, bh),
        s,
    }
}

pub struct VictoryLayout {
    pub panel: Rect,
    pub continue_btn: Rect,
    pub quit: Rect,
    pub s: f32,
}

pub fn victory_layout(game: &Game) -> VictoryLayout {
    let s = ui_scale(game);
    let w = (game.view.x - 36.0 * s).min(430.0 * s).max(250.0);
    let h = (game.view.y - 36.0 * s).min(250.0 * s).max(220.0);
    let panel = Rect::new((game.view.x - w) * 0.5, (game.view.y - h) * 0.5, w, h);
    let bw = w - 30.0 * s;
    let bh = 27.0 * s;
    VictoryLayout {
        panel,
        continue_btn: Rect::new(panel.x + 15.0 * s, panel.y + h - 15.0 * s - bh * 2.0 - 7.0 * s, bw, bh),
        quit: Rect::new(panel.x + 15.0 * s, panel.y + h - 15.0 * s - bh, bw, bh),
        s,
    }
}

pub fn top_bar_contains(game: &Game, p: V2) -> bool {
    let s = ui_scale(game);
    let hearts = Rect::new(10.0 * s, 8.0 * s, 132.0 * s, 22.0 * s);
    let qw = 168.0 * s;
    let qx = game.view.x - qw - 10.0 * s;
    let qy = if game.touch_mode { 34.0 * s } else { 8.0 * s };
    let quest = Rect::new(qx, qy, qw, 54.0 * s);
    hearts.contains(p) || quest.contains(p)
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

pub fn draw(game: &Game) {
    if game.mode == Mode::Attract {
        return;
    }
    draw_hud(game);
    if game.inv_open {
        draw_inventory(game);
    }
    if game.map_open {
        draw_map(game);
    }
    match game.mode {
        Mode::Paused => draw_pause(game),
        Mode::Dead => draw_death(game),
        Mode::Victory => draw_victory(game),
        _ => {}
    }
}

fn shadow_panel(r: Rect, fill: u32, border: u32) {
    gfx::rect(r.x + 3.0, r.y + 4.0, r.w, r.h, rgba(0, 0, 0, 120));
    gfx::rect(r.x, r.y, r.w, r.h, fill);
    gfx::rect(r.x, r.y, r.w, 1.0, alpha(border, 255));
    gfx::rect(r.x, r.y + r.h - 1.0, r.w, 1.0, rgba(0, 0, 0, 120));
    gfx::rect(r.x, r.y, 1.0, r.h, alpha(border, 200));
    gfx::rect(r.x + r.w - 1.0, r.y, 1.0, r.h, rgba(0, 0, 0, 120));
}

fn text_shadow(s: &str, x: f32, y: f32, size: f32, color: u32, align: u32, weight: u32) {
    gfx::text(s, x + 1.0, y + 1.2, size, rgba(6, 8, 14, 180), align, weight);
    gfx::text(s, x, y, size, color, align, weight);
}

fn draw_hud(game: &Game) {
    let s = ui_scale(game);
    draw_hearts(game, s);
    draw_quest(game, s);
    draw_hotbar(game, s);
    draw_clock_depth(game, s);
    draw_boss_bar(game, s);
    draw_toasts(game, s);
    draw_banner(game, s);
}

fn draw_hearts(game: &Game, s: f32) {
    let p = &game.player;
    let x0 = 12.0 * s;
    let y = 18.0 * s;
    let step = 12.3 * s;
    let hp = p.hp.max(0.0);
    for i in 0..10 {
        let frac = clamp((hp - i as f32 * 10.0) / 10.0, 0.0, 1.0);
        let x = x0 + i as f32 * step;
        let col = if frac > 0.0 {
            if p.hp < 30.0 {
                let pulse = 0.75 + 0.25 * (game.title_phase * 8.0).sin();
                mix(rgb(210, 54, 68), rgb(255, 120, 120), pulse)
            } else {
                rgb(222, 64, 78)
            }
        } else {
            rgb(44, 40, 50)
        };
        draw_heart(x, y, 9.0 * s, col, frac > 0.0);
        if frac > 0.0 && frac < 1.0 {
            // Partial heart is approximated by a smaller filled heart.
            draw_heart(x, y, 9.0 * s * (0.55 + frac * 0.45), mix(rgb(222, 64, 78), rgb(255, 150, 150), 0.4), true);
        }
    }
    let hp_text = format!("{}/{}", p.hp.ceil() as i32, p.max_hp as i32);
    gfx::text(&hp_text, x0 + 10.0 * step + 4.0 * s, y + 3.5 * s, 6.0 * s, rgba(240, 228, 220, 220), 0, 700);
    let hurt = game.hurt_vignette;
    if hurt > 0.05 {
        gfx::rect(0.0, 0.0, game.view.x, game.view.y, rgba(180, 20, 30, (hurt * 34.0) as u8));
    }
}

fn draw_heart(x: f32, y: f32, size: f32, color: u32, filled: bool) {
    let c = if filled { color } else { rgba(20, 18, 26, 210) };
    gfx::circle(x + size * 0.28, y + size * 0.3, size * 0.28, c);
    gfx::circle(x + size * 0.72, y + size * 0.3, size * 0.28, c);
    gfx::tri(x - size * 0.02, y + size * 0.38, x + size * 0.5, y + size, x + size * 1.02, y + size * 0.38, c);
    if filled {
        gfx::circle(x + size * 0.24, y + size * 0.24, size * 0.1, rgba(255, 190, 190, 160));
    }
}

fn draw_quest(game: &Game, s: f32) {
    let (title, sub) = game.objective();
    let w = 178.0 * s;
    let x = game.view.x - w - 10.0 * s;
    let y = if game.touch_mode { 36.0 * s } else { 8.0 * s };
    let h = 48.0 * s;
    let r = Rect::new(x, y, w, h);
    shadow_panel(r, rgba(16, 20, 30, 200), rgba(86, 118, 132, 150));
    gfx::rect(x, y, 3.0 * s, h, rgb(206, 166, 78));
    gfx::text("OBJECTIVE", x + 9.0 * s, y + 9.0 * s, 5.0 * s, rgba(220, 186, 110, 220), 0, 800);
    gfx::text(title, x + 9.0 * s, y + 21.0 * s, 7.2 * s, rgb(244, 238, 222), 0, 700);
    gfx::text(sub, x + 9.0 * s, y + 34.0 * s, 5.2 * s, rgba(178, 190, 200, 210), 0, 500);
}

fn draw_clock_depth(game: &Game, s: f32) {
    let time = game.time;
    let hours = (5.0 + time * 24.0) % 24.0;
    let hh = hours.floor() as i32;
    let mm = ((hours - hours.floor()) * 60.0) as i32;
    let label = format!("Day {}  {:02}:{:02}", game.day, hh, mm);
    let w = 92.0 * s;
    let x = game.view.x - w - 10.0 * s;
    let y = if game.touch_mode { 90.0 * s } else { 62.0 * s };
    gfx::rect(x, y, w, 17.0 * s, rgba(14, 18, 28, 185));
    gfx::rect(x, y, 2.0 * s, 17.0 * s, rgb(120, 164, 188));
    let icon_col = if game.is_night() { rgb(190, 208, 255) } else { rgb(255, 214, 110) };
    gfx::circle(x + 10.0 * s, y + 8.5 * s, 3.0 * s, icon_col);
    if game.is_night() {
        gfx::circle(x + 12.0 * s, y + 7.5 * s, 2.4 * s, rgba(14, 18, 28, 255));
    }
    gfx::text(&label, x + 18.0 * s, y + 8.7 * s, 5.8 * s, rgb(232, 232, 226), 0, 700);

    let surface = game.world.surface_px(game.player.pos.x as i32);
    let depth = ((game.player.feet().y - surface) / TILE as f32 * 2.0).round() as i32;
    let depth_label = if depth >= 0 {
        format!("Depth {} ft", depth)
    } else {
        format!("{} ft above", -depth)
    };
    let dx = game.view.x - 10.0 * s;
    let dy = game.view.y * 0.55;
    text_shadow(&depth_label, dx, dy, 6.0 * s, rgba(214, 218, 224, 210), 2, 600);
}

fn draw_hotbar(game: &Game, s: f32) {
    let layout = hotbar_layout(game);
    let hover = layout.slot_at(game.mouse_screen);
    for (i, r) in layout.slots.iter().enumerate() {
        let selected = i == game.inventory.selected;
        let fill = if selected {
            rgba(58, 66, 78, 235)
        } else {
            rgba(18, 22, 32, 205)
        };
        gfx::rect(r.x, r.y, r.w, r.h, fill);
        let border = if selected {
            rgb(238, 200, 102)
        } else if hover == Some(i) {
            rgb(130, 160, 172)
        } else {
            rgba(92, 104, 118, 180)
        };
        let bw = if selected { 1.8 * s } else { 1.0 * s };
        gfx::rect(r.x, r.y, r.w, bw, border);
        gfx::rect(r.x, r.y + r.h - bw, r.w, bw, border);
        gfx::rect(r.x, r.y, bw, r.h, border);
        gfx::rect(r.x + r.w - bw, r.y, bw, r.h, border);
        if selected {
            gfx::rect(r.x - 2.0 * s, r.y - 2.0 * s, r.w + 4.0 * s, 2.0 * s, rgb(238, 200, 102));
        }
        if let Some(st) = game.inventory.slots[i] {
            draw_item_icon(st.id, r.x + r.w * 0.5, r.y + r.h * 0.48, r.w * 0.62, 240);
            if st.count > 1 {
                gfx::text(&st.count.to_string(), r.x + r.w - 3.0 * s, r.y + r.h - 6.0 * s, 5.4 * s, rgb(246, 244, 236), 2, 800);
            }
        } else {
            gfx::text("-", r.x + r.w * 0.5, r.y + r.h * 0.5, 6.0 * s, rgba(120, 128, 140, 110), 1, 400);
        }
        gfx::text(&format!("{}", if i == 9 { 0 } else { i + 1 }), r.x + 3.0 * s, r.y + 5.5 * s, 4.6 * s, rgba(220, 220, 220, 120), 0, 700);
    }
    // Selected item name.
    if let Some(st) = game.inventory.selected_stack() {
        let y = layout.y - 13.0 * s;
        let name = st.id.name();
        let w = gfx::text_width(name, 6.5 * s, 700) + 18.0 * s;
        let x = game.view.x * 0.5 - w * 0.5;
        gfx::rect(x, y - 7.0 * s, w, 14.0 * s, rgba(12, 16, 24, 200));
        gfx::rect(x, y - 7.0 * s, w, 1.0, alpha(st.id.color(), 200));
        gfx::text(name, game.view.x * 0.5, y, 6.5 * s, rgb(240, 238, 228), 1, 700);
    }
}

fn draw_boss_bar(game: &Game, s: f32) {
    if !game.boss_active {
        return;
    }
    let Some(boss) = game.enemies.iter().find(|e| e.kind.is_boss() && !e.dead) else { return };
    let w = (game.view.x * 0.56).min(390.0 * s);
    let h = 8.0 * s;
    let x = (game.view.x - w) * 0.5;
    let y = if game.view.x < 480.0 { 76.0 * s } else { 18.0 * s };
    text_shadow("THE SLIME KING", game.view.x * 0.5, y - 7.0 * s, 7.0 * s, rgb(198, 246, 214), 1, 800);
    gfx::rect(x - 1.0, y - 1.0, w + 2.0, h + 2.0, rgba(8, 12, 18, 220));
    gfx::rect(x, y, w, h, rgba(44, 34, 42, 220));
    let f = (boss.hp / boss.max_hp).clamp(0.0, 1.0);
    gfx::rect(x, y, w * f, h, mix(rgb(184, 54, 62), rgb(94, 212, 144), f));
    gfx::rect(x, y, w * f, h * 0.35, rgba(255, 255, 255, 45));
    for i in 1..10 {
        gfx::rect(x + w * i as f32 / 10.0, y, 1.0, h, rgba(10, 12, 18, 90));
    }
    draw_item_icon(ItemId::Crown, x - 9.0 * s, y + h * 0.5, 11.0 * s, 230);
}

fn draw_toasts(game: &Game, s: f32) {
    let layout = hotbar_layout(game);
    let mut y = layout.y - 26.0 * s;
    for t in game.toasts.iter().rev().take(3) {
        let a = ((t.life.min(0.5) / 0.5) * 255.0).clamp(0.0, 255.0) as u8;
        let x = 12.0 * s;
        let w = 210.0 * s;
        let h = 25.0 * s;
        gfx::rect(x, y - h, w, h, rgba(14, 18, 26, (a as f32 * 0.85) as u8));
        gfx::rect(x, y - h, 3.0 * s, h, alpha(t.color, a));
        gfx::text(&t.text, x + 8.0 * s, y - h + 9.0 * s, 6.2 * s, alpha(t.color, a), 0, 800);
        gfx::text(&t.sub, x + 8.0 * s, y - h + 18.0 * s, 4.9 * s, rgba(190, 196, 204, a), 0, 500);
        y -= h + 4.0 * s;
    }
}

fn draw_banner(game: &Game, s: f32) {
    let Some(b) = &game.banner else { return };
    let f = (b.life / b.max_life).clamp(0.0, 1.0);
    let fade = ((1.0 - f) * 5.0).min(1.0).min((f * 5.0).min(1.0));
    let a = (fade * 255.0) as u8;
    let y = game.view.y * 0.24;
    let w = (game.view.x * 0.72).min(520.0 * s);
    gfx::rect((game.view.x - w) * 0.5, y - 22.0 * s, w, 2.0 * s, rgba(240, 210, 120, (a as f32 * 0.5) as u8));
    gfx::rect((game.view.x - w) * 0.5, y + 22.0 * s, w, 1.0 * s, rgba(240, 210, 120, (a as f32 * 0.35) as u8));
    text_shadow(&b.text, game.view.x * 0.5, y - 6.0 * s, 19.0 * s, alpha(b.color, a), 1, 900);
    gfx::text(&b.sub, game.view.x * 0.5, y + 13.0 * s, 6.5 * s, rgba(232, 226, 210, a), 1, 600);
}

// ---------------------------------------------------------------------------
// Inventory / crafting
// ---------------------------------------------------------------------------

fn draw_inventory(game: &Game) {
    let l = inventory_layout(game);
    let s = l.s;
    gfx::rect(0.0, 0.0, game.view.x, game.view.y, rgba(4, 6, 12, 150));
    shadow_panel(l.panel, rgba(20, 24, 34, 246), rgb(118, 148, 158));
    gfx::text("INVENTORY", l.panel.x + 12.0 * s, l.panel.y + 15.0 * s, 10.0 * s, rgb(240, 236, 222), 0, 900);
    gfx::text("& CRAFTING", l.panel.x + 12.0 * s + gfx::text_width("INVENTORY ", 10.0 * s, 900), l.panel.y + 15.0 * s, 10.0 * s, rgb(202, 172, 100), 0, 900);

    // Close button.
    let cx = l.close.center();
    let close_hover = l.close.contains(game.mouse_screen);
    gfx::rect(l.close.x, l.close.y, l.close.w, l.close.h, if close_hover { rgba(160, 70, 70, 230) } else { rgba(48, 44, 52, 220) });
    gfx::line(cx.x - 3.0 * s, cx.y - 3.0 * s, cx.x + 3.0 * s, cx.y + 3.0 * s, 1.6 * s, rgb(230, 220, 210));
    gfx::line(cx.x + 3.0 * s, cx.y - 3.0 * s, cx.x - 3.0 * s, cx.y + 3.0 * s, 1.6 * s, rgb(230, 220, 210));

    let bench = game.world.bench_near(game.player.center().x, game.player.center().y, 6);
    let bench_text = if bench { "WORKBENCH NEARBY" } else { "WORKBENCH REQUIRED FOR ADVANCED ITEMS" };
    gfx::text(bench_text, l.panel.x + l.panel.w - 36.0 * s, l.panel.y + 17.0 * s, 4.8 * s, if bench { rgb(144, 214, 160) } else { rgba(210, 176, 108, 220) }, 2, 700);

    for (i, r) in l.slots.iter().enumerate() {
        let selected = i == game.inventory.selected;
        let hover = game.hover_inv_slot == Some(i);
        let fill = if selected { rgba(60, 68, 82, 240) } else if i < HOTBAR { rgba(28, 34, 46, 230) } else { rgba(22, 26, 36, 220) };
        gfx::rect(r.x, r.y, r.w, r.h, fill);
        let border = if selected { rgb(236, 198, 100) } else if hover { rgb(144, 176, 188) } else { rgba(82, 94, 108, 170) };
        stroke_rect(*r, if selected { 1.6 * s } else { 1.0 }, border);
        if let Some(st) = game.inventory.slots[i] {
            draw_item_icon(st.id, r.x + r.w * 0.5, r.y + r.h * 0.47, r.w * 0.62, 245);
            if st.count > 1 {
                gfx::text(&st.count.to_string(), r.x + r.w - 2.5 * s, r.y + r.h - 5.5 * s, 4.8 * s, rgb(248, 246, 236), 2, 800);
            }
        }
        if i < HOTBAR {
            gfx::text(&format!("{}", if i == 9 { 0 } else { i + 1 }), r.x + 2.5 * s, r.y + 5.0 * s, 4.2 * s, rgba(210, 210, 210, 110), 0, 700);
        } else if hover {
            gfx::text("swap", r.x + r.w * 0.5, r.y + r.h - 5.0 * s, 3.8 * s, rgba(170, 200, 210, 200), 1, 600);
        }
    }

    // Recipes.
    for (i, recipe) in RECIPES.iter().enumerate() {
        let Some(r) = l.recipes.get(i) else { continue };
        let hover = game.hover_recipe == Some(i);
        let mut can = true;
        for inp in recipe.inputs.iter().flatten() {
            if game.inventory.count(inp.0) < inp.1 {
                can = false;
            }
        }
        if recipe.bench && !bench {
            can = false;
        }
        let fill = if hover && can {
            rgba(48, 70, 66, 240)
        } else if hover {
            rgba(56, 46, 44, 230)
        } else {
            rgba(24, 28, 38, 215)
        };
        gfx::rect(r.x, r.y, r.w, r.h, fill);
        stroke_rect(*r, 1.0, if hover { rgba(150, 184, 176, 220) } else { rgba(70, 80, 94, 150) });
        draw_item_icon(recipe.output, r.x + 9.0 * s, r.y + r.h * 0.5, 11.0 * s, if can { 240 } else { 90 });
        let name_col = if can { rgb(238, 236, 224) } else { rgba(166, 168, 172, 220) };
        gfx::text(recipe.name, r.x + 18.0 * s, r.y + 7.0 * s, 5.8 * s, name_col, 0, 700);
        gfx::text(&format!("x{}", recipe.out_count), r.x + r.w - 5.0 * s, r.y + 7.0 * s, 5.0 * s, rgba(210, 190, 130, 210), 2, 700);
        // Ingredient string.
        let mut ingredients = Vec::new();
        for inp in recipe.inputs.iter().flatten() {
            let have = game.inventory.count(inp.0);
            ingredients.push(format!("{} {}/{}", inp.0.short(), have, inp.1));
        }
        if recipe.bench {
            ingredients.push(if bench { "bench ok".to_string() } else { "need bench".to_string() });
        }
        gfx::text(&ingredients.join("  "), r.x + 18.0 * s, r.y + 16.0 * s, 4.2 * s, if can { rgba(146, 200, 154, 230) } else { rgba(214, 134, 128, 220) }, 0, 500);
        if hover {
            gfx::rect(r.x + r.w - 32.0 * s, r.y + 2.0 * s, 28.0 * s, r.h - 4.0 * s, if can { rgba(110, 180, 132, 110) } else { rgba(120, 80, 80, 70) });
            gfx::text("CRAFT", r.x + r.w - 18.0 * s, r.y + r.h * 0.5, 4.4 * s, if can { rgb(220, 250, 220) } else { rgba(210, 170, 170, 180) }, 1, 800);
        }
    }

    // Tooltip and footer. A recipe hint replaces the footer line so it never
    // overlaps the recipe grid.
    let footer = "Click a backpack slot to swap it with the selected hotbar slot. Press E or Esc to close.";
    let footer_text = if let Some(i) = game.hover_recipe {
        RECIPES[i].hint
    } else {
        footer
    };
    gfx::text(footer_text, l.panel.x + l.panel.w * 0.5, l.panel.y + l.panel.h - 9.0 * s, 4.4 * s, rgba(190, 198, 190, 215), 1, 500);
    if let Some(i) = game.hover_inv_slot {
        if let Some(st) = game.inventory.slots[i] {
            draw_tooltip(game, st, game.mouse_screen, s);
        }
    }
}

fn stroke_rect(r: Rect, w: f32, color: u32) {
    gfx::rect(r.x, r.y, r.w, w, color);
    gfx::rect(r.x, r.y + r.h - w, r.w, w, color);
    gfx::rect(r.x, r.y, w, r.h, color);
    gfx::rect(r.x + r.w - w, r.y, w, r.h, color);
}

fn draw_tooltip(game: &Game, st: ItemStack, mouse: V2, s: f32) {
    let name = st.id.name();
    let line2 = match st.id {
        ItemId::Pickaxe => "Mines blocks. Hold left click.".to_string(),
        ItemId::RustyBlade | ItemId::CopperBlade | ItemId::IronBlade | ItemId::GoldBlade => {
            format!("{} melee damage", st.id.sword_damage())
        }
        ItemId::Torch => "Places light.".to_string(),
        ItemId::Bomb => "Careful. Blows holes in the world.".to_string(),
        ItemId::Crown => "Summons The Slime King.".to_string(),
        ItemId::Potion => "Restores 45 health.".to_string(),
        ItemId::Trophy => "Proof you toppled a king.".to_string(),
        _ if st.id.is_placeable() => "A placeable block.".to_string(),
        _ => "A crafting material.".to_string(),
    };
    let w = 150.0 * s;
    let h = 30.0 * s;
    let x = clamp(mouse.x + 12.0 * s, 4.0, game.view.x - w - 4.0);
    let y = clamp(mouse.y - h - 8.0 * s, 4.0, game.view.y - h - 4.0);
    shadow_panel(Rect::new(x, y, w, h), rgba(12, 16, 24, 245), alpha(st.id.color(), 220));
    gfx::text(name, x + 7.0 * s, y + 9.0 * s, 5.8 * s, rgb(240, 238, 228), 0, 700);
    gfx::text(&line2, x + 7.0 * s, y + 20.0 * s, 4.4 * s, rgba(178, 186, 196, 220), 0, 500);
    gfx::text(&format!("x{}", st.count), x + w - 7.0 * s, y + 9.0 * s, 5.2 * s, rgb(220, 204, 150), 2, 700);
}

// ---------------------------------------------------------------------------
// Map
// ---------------------------------------------------------------------------

fn draw_map(game: &Game) {
    let s = ui_scale(game);
    gfx::rect(0.0, 0.0, game.view.x, game.view.y, rgba(4, 6, 12, 185));
    let w = (game.view.x - 22.0 * s).min(620.0 * s);
    let map_h = w * (world::H as f32 / world::W as f32);
    let h = map_h + 58.0 * s;
    let panel = Rect::new((game.view.x - w) * 0.5, (game.view.y - h) * 0.5, w, h);
    shadow_panel(panel, rgba(18, 22, 32, 248), rgb(116, 146, 154));
    gfx::text("WORLD MAP", panel.x + 12.0 * s, panel.y + 14.0 * s, 9.0 * s, rgb(240, 234, 220), 0, 900);
    gfx::text(&format!("seed {}", game.seed), panel.x + panel.w - 12.0 * s, panel.y + 14.0 * s, 5.0 * s, rgba(168, 182, 192, 210), 2, 600);
    let mx = panel.x + 10.0 * s;
    let my = panel.y + 26.0 * s;
    let mw = w - 20.0 * s;
    let mh = map_h - 6.0 * s;
    gfx::rect(mx - 1.0, my - 1.0, mw + 2.0, mh + 2.0, rgba(8, 10, 16, 230));
    gfx::minimap_draw(mx, my, mw, mh, 1.0);
    // Grid overlay.
    for i in 1..6 {
        gfx::rect(mx + mw * i as f32 / 6.0, my, 0.6, mh, rgba(255, 255, 255, 14));
    }
    for i in 1..4 {
        gfx::rect(mx, my + mh * i as f32 / 4.0, mw, 0.6, rgba(255, 255, 255, 12));
    }
    let pw = world::W as f32 * TILE as f32;
    let ph = world::H as f32 * TILE as f32;
    let px = mx + (game.player.pos.x / pw) * mw;
    let py = my + (game.player.pos.y / ph) * mh;
    gfx::circle(px, py, 3.2 * s, rgb(255, 226, 120));
    gfx::circle(px, py, 1.5 * s, rgb(255, 250, 230));
    let sx = mx + (game.world.spawn.x / pw) * mw;
    let sy = my + (game.world.spawn.y / ph) * mh;
    gfx::rect(sx - 2.0 * s, sy - 2.0 * s, 4.0 * s, 4.0 * s, rgb(120, 232, 150));
    for e in &game.enemies {
        if e.dead {
            continue;
        }
        let ex = mx + (e.center().x / pw) * mw;
        let ey = my + (e.center().y / ph) * mh;
        if e.kind.is_boss() {
            gfx::circle(ex, ey, 4.0 * s, rgb(232, 72, 72));
            gfx::ring(ex, ey, 6.0 * s, 1.0, rgba(255, 120, 120, 200));
        } else {
            gfx::rect(ex - 1.0, ey - 1.0, 2.0, 2.0, rgba(230, 100, 100, 200));
        }
    }
    gfx::text("M or Esc to close  •  gold: you  •  green: spawn", panel.x + panel.w * 0.5, panel.y + h - 10.0 * s, 4.8 * s, rgba(176, 188, 198, 210), 1, 500);
}

// ---------------------------------------------------------------------------
// Menus
// ---------------------------------------------------------------------------

fn menu_backdrop(game: &Game, tint: u32) {
    gfx::rect(0.0, 0.0, game.view.x, game.view.y, tint);
}

fn draw_pause(game: &Game) {
    let l = pause_layout(game);
    let s = l.s;
    menu_backdrop(game, rgba(4, 6, 12, 185));
    shadow_panel(l.panel, rgba(18, 22, 32, 248), rgb(112, 142, 150));
    gfx::text("PAUSED", l.panel.x + 16.0 * s, l.panel.y + 22.0 * s, 15.0 * s, rgb(240, 234, 220), 0, 900);
    gfx::text("The world waits.", l.panel.x + 16.0 * s, l.panel.y + 36.0 * s, 5.2 * s, rgba(170, 182, 192, 220), 0, 500);

    let (obj, sub) = game.objective();
    gfx::rect(l.panel.x + 15.0 * s, l.panel.y + 46.0 * s, l.panel.w - 30.0 * s, 28.0 * s, rgba(32, 38, 48, 190));
    gfx::text("OBJECTIVE", l.panel.x + 21.0 * s, l.panel.y + 53.0 * s, 4.5 * s, rgb(214, 182, 104), 0, 800);
    gfx::text(obj, l.panel.x + 21.0 * s, l.panel.y + 63.0 * s, 6.2 * s, rgb(238, 234, 222), 0, 700);
    gfx::text(sub, l.panel.x + 21.0 * s, l.panel.y + 71.0 * s, 4.3 * s, rgba(172, 182, 192, 210), 0, 500);

    if game.show_help {
        let lines = [
            "A / D or arrows   Move",
            "Space / W        Jump (swim in water)",
            "Left click       Mine, attack or place",
            "Right click      Place selected block",
            "1-0 / wheel      Select hotbar",
            "E / C            Inventory & crafting",
            "M                World map",
            "Esc              Pause",
            "N                Mute sound",
        ];
        for (i, line) in lines.iter().enumerate() {
            gfx::text(line, l.panel.x + 18.0 * s, l.panel.y + (88.0 + i as f32 * 9.0) * s, 4.7 * s, rgba(198, 206, 214, 230), 0, 500);
        }
    } else {
        let stats = format!(
            "Mined {}   Placed {}   Slain {}   Deaths {}   Played {}",
            game.stats.blocks_mined,
            game.stats.blocks_placed,
            game.stats.enemies_killed,
            game.stats.deaths,
            format_time(game.stats.play_time)
        );
        gfx::text(&stats, l.panel.x + 18.0 * s, l.panel.y + 100.0 * s, 4.6 * s, rgba(186, 196, 204, 220), 0, 500);
        let tip = if game.boss_defeated {
            "The crown is yours. The world is yours."
        } else {
            "Tip: torches, platforms and a workbench make every trip easier."
        };
        gfx::text(tip, l.panel.x + 18.0 * s, l.panel.y + 112.0 * s, 4.4 * s, rgba(160, 176, 184, 200), 0, 500);
    }

    button(game, l.resume, "Resume");
    button(game, l.help, if game.show_help { "Hide Controls" } else { "Controls" });
    button(game, l.mute, if game.muted { "Sound: Off" } else { "Sound: On" });
    button(game, l.quit, "Save & Quit to Title");
}

fn button(game: &Game, r: Rect, label: &str) {
    let hover = r.contains(game.mouse_screen);
    let fill = if hover { rgba(62, 88, 90, 245) } else { rgba(34, 42, 54, 235) };
    gfx::rect(r.x, r.y, r.w, r.h, fill);
    stroke_rect(r, 1.2, if hover { rgb(232, 198, 110) } else { rgba(104, 124, 136, 200) });
    gfx::text(label, r.center().x, r.center().y, 6.2 * ui_scale(game), if hover { rgb(246, 240, 222) } else { rgb(214, 216, 214) }, 1, 700);
}

fn draw_death(game: &Game) {
    let l = death_layout(game);
    let s = l.s;
    menu_backdrop(game, rgba(28, 4, 8, 200));
    shadow_panel(l.panel, rgba(26, 16, 20, 250), rgb(150, 70, 78));
    gfx::text("YOU DIED", l.panel.x + l.panel.w * 0.5, l.panel.y + 34.0 * s, 18.0 * s, rgb(236, 96, 96), 1, 900);
    gfx::text("The world is unkind.", l.panel.x + l.panel.w * 0.5, l.panel.y + 50.0 * s, 5.8 * s, rgba(218, 190, 190, 220), 1, 500);
    let line = format!("Slain {}   Mined {}   Deaths {}", game.stats.enemies_killed, game.stats.blocks_mined, game.stats.deaths);
    gfx::text(&line, l.panel.x + l.panel.w * 0.5, l.panel.y + 68.0 * s, 4.8 * s, rgba(188, 176, 176, 210), 1, 500);
    gfx::text("Your gear is safe. The world is not.", l.panel.x + l.panel.w * 0.5, l.panel.y + 82.0 * s, 4.5 * s, rgba(170, 160, 164, 200), 1, 500);
    button(game, l.respawn, "Respawn");
    button(game, l.quit, "Save & Quit to Title");
    gfx::text("Space or Enter also respawns", l.panel.x + l.panel.w * 0.5, l.panel.y + l.panel.h - 4.0 * s, 3.8 * s, rgba(160, 152, 156, 170), 1, 500);
}

fn draw_victory(game: &Game) {
    let l = victory_layout(game);
    let s = l.s;
    menu_backdrop(game, rgba(8, 8, 4, 195));
    shadow_panel(l.panel, rgba(26, 24, 16, 250), rgb(210, 172, 76));
    gfx::text("THE KING IS DEAD", l.panel.x + l.panel.w * 0.5, l.panel.y + 30.0 * s, 15.0 * s, rgb(255, 216, 104), 1, 900);
    gfx::text("The Slime King has fallen.", l.panel.x + l.panel.w * 0.5, l.panel.y + 47.0 * s, 6.2 * s, rgb(238, 228, 198), 1, 600);
    let line = format!(
        "Slain {}   Mined {}   Deaths {}   Time {}",
        game.stats.enemies_killed,
        game.stats.blocks_mined,
        game.stats.deaths,
        format_time(game.stats.play_time)
    );
    gfx::text(&line, l.panel.x + l.panel.w * 0.5, l.panel.y + 65.0 * s, 4.8 * s, rgba(212, 202, 178, 220), 1, 500);
    gfx::text("The crown is yours. The world is not finished.", l.panel.x + l.panel.w * 0.5, l.panel.y + 78.0 * s, 4.8 * s, rgba(196, 186, 160, 210), 1, 500);
    button(game, l.continue_btn, "Continue Exploring");
    button(game, l.quit, "Save & Quit to Title");
}

fn format_time(secs: f32) -> String {
    let m = (secs / 60.0).floor() as i32;
    let s = (secs as i32) % 60;
    if m >= 60 {
        format!("{}h {:02}m", m / 60, m % 60)
    } else {
        format!("{}m {:02}s", m, s)
    }
}
