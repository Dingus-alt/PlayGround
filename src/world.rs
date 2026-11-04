//! Procedural world generation, tile storage and terrain queries.

use crate::items::ItemId;
use crate::math::{clamp, clamp_i, V2};
use crate::rng::{fbm, hash2, hash_i, Rng};
use crate::gfx;

pub const TILE: i32 = 16;
pub const W: i32 = 900;
pub const H: i32 = 300;

// Tile ids
pub const AIR: u8 = 0;
pub const DIRT: u8 = 1;
pub const GRASS: u8 = 2;
pub const STONE: u8 = 3;
pub const WOOD: u8 = 4;
pub const LEAVES: u8 = 5;
pub const COPPER: u8 = 6;
pub const IRON: u8 = 7;
pub const GOLD: u8 = 8;
pub const GEM: u8 = 9;
pub const SAND: u8 = 10;
pub const SNOW: u8 = 11;
pub const TORCH: u8 = 12;
pub const PLATFORM: u8 = 13;
pub const BENCH: u8 = 14;
pub const CLAY: u8 = 15;
pub const MUSHROOM: u8 = 16;
pub const WATER: u8 = 17;
pub const BEDROCK: u8 = 18;
pub const CACTUS: u8 = 19;
pub const TILE_TYPES: u8 = 20;

// Wall ids
pub const WALL_NONE: u8 = 0;
pub const WALL_DIRT: u8 = 1;
pub const WALL_STONE: u8 = 2;
pub const WALL_WOOD: u8 = 3;

// Biomes
pub const BIOME_FOREST: u8 = 0;
pub const BIOME_DESERT: u8 = 1;
pub const BIOME_SNOW: u8 = 2;

pub fn is_solid(t: u8) -> bool {
    matches!(
        t,
        DIRT | GRASS | STONE | WOOD | COPPER | IRON | GOLD | GEM | SAND | SNOW | CLAY | BEDROCK | CACTUS
    )
}

pub fn is_platform(t: u8) -> bool {
    t == PLATFORM
}

pub fn is_water(t: u8) -> bool {
    t == WATER
}

pub fn blocks_light(t: u8) -> bool {
    is_solid(t)
}

pub fn hardness(t: u8) -> f32 {
    match t {
        AIR => 0.0,
        GRASS | DIRT | SAND | SNOW | CLAY => 0.22,
        STONE => 0.52,
        COPPER => 0.72,
        IRON => 0.9,
        GOLD => 1.1,
        GEM => 1.25,
        WOOD | CACTUS => 0.65,
        LEAVES => 0.14,
        TORCH => 0.05,
        PLATFORM => 0.3,
        BENCH => 0.35,
        MUSHROOM => 0.08,
        WATER => 0.0,
        BEDROCK => 9999.0,
        _ => 0.4,
    }
}

pub fn tile_name(t: u8) -> &'static str {
    match t {
        DIRT => "Dirt",
        GRASS => "Grass",
        STONE => "Stone",
        WOOD => "Wood",
        LEAVES => "Leaves",
        COPPER => "Copper Ore",
        IRON => "Iron Ore",
        GOLD => "Gold Ore",
        GEM => "Amethyst",
        SAND => "Sand",
        SNOW => "Snow",
        TORCH => "Torch",
        PLATFORM => "Platform",
        BENCH => "Workbench",
        CLAY => "Clay",
        MUSHROOM => "Glow Mushroom",
        WATER => "Water",
        BEDROCK => "Bedrock",
        CACTUS => "Cactus",
        _ => "Air",
    }
}

pub fn tile_drop(t: u8) -> Option<(ItemId, u32)> {
    match t {
        DIRT | GRASS => Some((ItemId::Dirt, 1)),
        STONE => Some((ItemId::Stone, 1)),
        WOOD | CACTUS => Some((ItemId::Wood, 1)),
        COPPER => Some((ItemId::CopperOre, 1)),
        IRON => Some((ItemId::IronOre, 1)),
        GOLD => Some((ItemId::GoldOre, 1)),
        GEM => Some((ItemId::Gem, 1)),
        SAND => Some((ItemId::Sand, 1)),
        SNOW => Some((ItemId::Snow, 1)),
        CLAY => Some((ItemId::Clay, 1)),
        TORCH => Some((ItemId::Torch, 1)),
        PLATFORM => Some((ItemId::Platform, 1)),
        BENCH => Some((ItemId::Bench, 1)),
        MUSHROOM => Some((ItemId::Mushroom, 1)),
        LEAVES => {
            Some((ItemId::Wood, 1))
        }
        _ => None,
    }
}

fn tile_map_color(t: u8) -> (u8, u8, u8) {
    match t {
        DIRT => (120, 82, 52),
        GRASS => (84, 142, 60),
        STONE => (108, 112, 120),
        WOOD => (118, 82, 46),
        LEAVES => (62, 126, 66),
        COPPER => (196, 118, 68),
        IRON => (176, 166, 154),
        GOLD => (214, 180, 70),
        GEM => (148, 94, 212),
        SAND => (214, 192, 138),
        SNOW => (216, 230, 244),
        TORCH => (230, 160, 60),
        PLATFORM => (166, 120, 68),
        BENCH => (176, 130, 76),
        CLAY => (158, 100, 78),
        MUSHROOM => (120, 196, 236),
        WATER => (52, 120, 190),
        BEDROCK => (48, 48, 56),
        CACTUS => (70, 142, 80),
        _ => (14, 18, 30),
    }
}

#[derive(Clone, Debug)]
pub struct World {
    pub tiles: Vec<u8>,
    pub walls: Vec<u8>,
    pub surface: Vec<i32>,
    pub sky_top: Vec<i32>,
    pub biome: Vec<u8>,
    pub seed: u32,
    pub spawn: V2,
    pub modified: Vec<(i32, i32, u8)>,
    pub minimap: Vec<u8>,
    pub map_dirty: bool,
}

impl World {
    pub fn generate(seed: u32) -> World {
        generate(seed)
    }

    #[inline]
    pub fn idx(x: i32, y: i32) -> usize {
        (y * W + x) as usize
    }

    #[inline]
    pub fn in_bounds(x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < W && y < H
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> u8 {
        if Self::in_bounds(x, y) {
            self.tiles[Self::idx(x, y)]
        } else if y >= H {
            BEDROCK
        } else {
            AIR
        }
    }

    #[inline]
    pub fn get_wall(&self, x: i32, y: i32) -> u8 {
        if Self::in_bounds(x, y) {
            self.walls[Self::idx(x, y)]
        } else {
            WALL_NONE
        }
    }

    #[inline]
    pub fn solid(&self, x: i32, y: i32) -> bool {
        is_solid(self.get(x, y))
    }

    #[inline]
    pub fn platform(&self, x: i32, y: i32) -> bool {
        self.get(x, y) == PLATFORM
    }

    #[inline]
    pub fn water(&self, x: i32, y: i32) -> bool {
        self.get(x, y) == WATER
    }

    #[inline]
    pub fn solid_px(&self, px: f32, py: f32) -> bool {
        self.solid((px / TILE as f32).floor() as i32, (py / TILE as f32).floor() as i32)
    }

    #[inline]
    pub fn water_px(&self, px: f32, py: f32) -> bool {
        self.water((px / TILE as f32).floor() as i32, (py / TILE as f32).floor() as i32)
    }

    /// Records a player-made alteration and keeps derived caches fresh.
    pub fn set_tile(&mut self, x: i32, y: i32, t: u8) -> bool {
        if !Self::in_bounds(x, y) {
            return false;
        }
        let i = Self::idx(x, y);
        if self.tiles[i] == t {
            return false;
        }
        self.tiles[i] = t;
        self.modified.push((x, y, t));
        self.recompute_sky(x);
        self.update_map(x, y);
        self.map_dirty = true;
        true
    }

    pub fn set_wall(&mut self, x: i32, y: i32, w: u8) {
        if Self::in_bounds(x, y) {
            self.walls[Self::idx(x, y)] = w;
            self.update_map(x, y);
        }
    }

    pub fn recompute_sky(&mut self, x: i32) {
        if x < 0 || x >= W {
            return;
        }
        let mut top = H;
        for y in 0..H {
            let t = self.tiles[Self::idx(x, y)];
            if is_solid(t) || t == WATER {
                top = y;
                break;
            }
        }
        self.sky_top[x as usize] = top;
    }

    pub fn surface_px(&self, x: i32) -> f32 {
        self.surface[clamp_i(x, 0, W - 1) as usize] as f32 * TILE as f32
    }

    fn update_map(&mut self, x: i32, y: i32) {
        if !Self::in_bounds(x, y) {
            return;
        }
        let (r, g, b) = self.map_pixel_color(x, y);
        let i = ((y * W + x) * 4) as usize;
        self.minimap[i] = r;
        self.minimap[i + 1] = g;
        self.minimap[i + 2] = b;
        self.minimap[i + 3] = 255;
    }

    fn map_pixel_color(&self, x: i32, y: i32) -> (u8, u8, u8) {
        let t = self.get(x, y);
        let w = self.get_wall(x, y);
        let n = hash2(x, y, self.seed) * 14.0 - 7.0;
        if t != AIR && t != WATER {
            let (r, g, b) = tile_map_color(t);
            return (
                (r as f32 + n).clamp(0.0, 255.0) as u8,
                (g as f32 + n).clamp(0.0, 255.0) as u8,
                (b as f32 + n).clamp(0.0, 255.0) as u8,
            );
        }
        if w != WALL_NONE {
            let (r, g, b) = match w {
                WALL_DIRT => (74, 52, 36),
                WALL_STONE => (56, 58, 66),
                WALL_WOOD => (76, 54, 34),
                _ => (40, 40, 48),
            };
            return (
                (r as f32 + n).clamp(0.0, 255.0) as u8,
                (g as f32 + n).clamp(0.0, 255.0) as u8,
                (b as f32 + n).clamp(0.0, 255.0) as u8,
            );
        }
        if t == WATER {
            return (48, 112, 182);
        }
        // Sky: brighter at the surface, fading into the deep.
        let depth = (y as f32 / self.surface[x as usize].max(1) as f32).clamp(0.0, 1.0);
        if y < self.surface[x as usize] {
            let r = 88.0 - depth * 40.0;
            let g = 154.0 - depth * 80.0;
            let b = 218.0 - depth * 70.0;
            (r as u8, g as u8, b as u8)
        } else {
            (14, 18, 30)
        }
    }

    pub fn rebuild_minimap(&mut self) {
        for y in 0..H {
            for x in 0..W {
                self.update_map(x, y);
            }
        }
    }

    /// True if the tile at this position can support the player from above.
    pub fn solid_or_platform(&self, x: i32, y: i32) -> bool {
        let t = self.get(x, y);
        is_solid(t) || is_platform(t)
    }

    pub fn bench_near(&self, px: f32, py: f32, radius: i32) -> bool {
        let cx = (px / TILE as f32) as i32;
        let cy = (py / TILE as f32) as i32;
        for y in (cy - radius)..=(cy + radius) {
            for x in (cx - radius)..=(cx + radius) {
                if self.get(x, y) == BENCH {
                    return true;
                }
            }
        }
        false
    }

    pub fn count_near(&self, px: f32, py: f32, radius: i32, tile: u8) -> i32 {
        let cx = (px / TILE as f32) as i32;
        let cy = (py / TILE as f32) as i32;
        let mut n = 0;
        for y in (cy - radius)..=(cy + radius) {
            for x in (cx - radius)..=(cx + radius) {
                if self.get(x, y) == tile {
                    n += 1;
                }
            }
        }
        n
    }
}

// ---------------------------------------------------------------------------
// Generation
// ---------------------------------------------------------------------------

pub fn generate(seed: u32) -> World {
    let mut rng = Rng::new(seed);
    let mut tiles = vec![AIR; (W * H) as usize];
    let mut walls = vec![WALL_NONE; (W * H) as usize];
    let mut surface = vec![(H as f32 * 0.32) as i32; W as usize];
    let mut biome = vec![BIOME_FOREST; W as usize];

    let desert_center = W * 68 / 100;
    let snow_center = W * 20 / 100;

    // Terrain height and biome assignment.
    for x in 0..W {
        let base = H as f32 * 0.30;
        let broad = (fbm(x as f32 * 0.0052, seed, 4) - 0.5) * 44.0;
        let detail = (fbm(x as f32 * 0.021, seed ^ 0x51ed, 3) - 0.5) * 11.0;
        let ridge = ((x as f32) * 0.017).sin() * 4.0;
        let mut h = (base + broad + detail + ridge).round() as i32;
        h = clamp_i(h, 42, (H as f32 * 0.52) as i32);

        let b = if (x - desert_center).abs() < 52 {
            BIOME_DESERT
        } else if (x - snow_center).abs() < 40 {
            BIOME_SNOW
        } else if x < 34 || x > W - 34 {
            BIOME_SNOW
        } else {
            BIOME_FOREST
        };
        biome[x as usize] = b;
        surface[x as usize] = h;

        let top_tile = match b {
            BIOME_DESERT => SAND,
            BIOME_SNOW => SNOW,
            _ => GRASS,
        };
        let filler = match b {
            BIOME_DESERT => SAND,
            BIOME_SNOW => SNOW,
            _ => DIRT,
        };
        let dirt_depth = 9 + (fbm(x as f32 * 0.13, seed ^ 0x77aa, 2) * 8.0) as i32;

        for y in h..H {
            let t = if y == h {
                top_tile
            } else if y <= h + dirt_depth {
                if b == BIOME_FOREST && y > h + 4 && hash2(x, y, seed ^ 0x9911) > 0.86 {
                    CLAY
                } else {
                    filler
                }
            } else {
                STONE
            };
            tiles[World::idx(x, y)] = t;
        }

        // Background walls.
        for y in h..H {
            let depth = y - h;
            let w = if depth < 4 {
                WALL_NONE
            } else if depth < dirt_depth + 6 {
                WALL_DIRT
            } else {
                WALL_STONE
            };
            walls[World::idx(x, y)] = w;
        }
    }

    // Deep dirt pockets and stone variation.
    for x in 0..W {
        for y in surface[x as usize]..H {
            if tiles[World::idx(x, y)] != STONE {
                continue;
            }
            let n = fbm(x as f32 * 0.055 + y as f32 * 0.033, seed ^ 0xa53, 3);
            if n > 0.68 && hash2(x, y, seed ^ 0x441) > 0.45 {
                tiles[World::idx(x, y)] = DIRT;
            }
        }
    }

    // Carve caves with wandering worms.
    let mut worms = 58;
    while worms > 0 {
        worms -= 1;
        let mut x = rng.range_i(10, W - 10) as f32;
        let mut y = rng.range_i(0, H - 30) as f32;
        let surf = surface[x as usize] as f32;
        if y < surf + 5.0 {
            y = surf + 5.0 + rng.range(0.0, 40.0);
        }
        let mut angle = rng.range(0.0, std::f32::consts::TAU);
        let steps = rng.range_i(60, 230);
        let base_r = rng.range(0.9, 2.4);
        for i in 0..steps {
            let r = base_r + ((i as f32 / steps as f32) * std::f32::consts::PI).sin() * rng.range(0.0, 1.4);
            carve_circle(&mut tiles, &mut walls, x, y, r, seed);
            angle += rng.range(-0.32, 0.32);
            x += angle.cos() * 1.35;
            y += angle.sin() * 1.35;
            if x < 8.0 || x > (W - 8) as f32 || y > (H - 8) as f32 {
                break;
            }
        }
    }

    // A handful of larger caverns deep down.
    for _ in 0..16 {
        let cx = rng.range_i(20, W - 20) as f32;
        let cy = rng.range_i((H as f32 * 0.55) as i32, H - 22) as f32;
        let rx = rng.range(5.0, 15.0);
        let ry = rng.range(3.0, 8.0);
        for y in (cy - ry - 2.0) as i32..=(cy + ry + 2.0) as i32 {
            for x in (cx - rx - 2.0) as i32..=(cx + rx + 2.0) as i32 {
                if !World::in_bounds(x, y) {
                    continue;
                }
                let dx = (x as f32 - cx) / rx;
                let dy = (y as f32 - cy) / ry;
                if dx * dx + dy * dy <= 1.0 {
                    let i = World::idx(x, y);
                    if is_solid(tiles[i]) || tiles[i] == AIR {
                        tiles[i] = AIR;
                        if walls[i] == WALL_NONE {
                            walls[i] = WALL_STONE;
                        }
                    }
                }
            }
        }
    }

    // Ore veins.
    let mut place_vein = |tiles: &mut Vec<u8>, rng: &mut Rng, tile: u8, y0: i32, y1: i32, count: i32, size: f32| {
        for _ in 0..count {
            let cx = rng.range_i(6, W - 6);
            let cy = rng.range_i(y0.clamp(0, H - 2), y1.clamp(1, H - 2));
            let approx = rng.range_i(2, size as i32);
            for oy in -approx..=approx {
                for ox in -approx..=approx {
                    let x = cx + ox;
                    let y = cy + oy;
                    if !World::in_bounds(x, y) {
                        continue;
                    }
                    let d = (ox * ox + oy * oy) as f32;
                    if d > (approx * approx) as f32 {
                        continue;
                    }
                    let i = World::idx(x, y);
                    if tiles[i] == STONE && hash2(x, y, seed) > 0.25 {
                        tiles[i] = tile;
                    }
                }
            }
        }
    };
    place_vein(&mut tiles, &mut rng, COPPER, (H as f32 * 0.36) as i32, (H as f32 * 0.70) as i32, 74, 5.0);
    place_vein(&mut tiles, &mut rng, IRON, (H as f32 * 0.46) as i32, (H as f32 * 0.86) as i32, 62, 4.5);
    place_vein(&mut tiles, &mut rng, GOLD, (H as f32 * 0.66) as i32, H - 12, 40, 3.8);
    place_vein(&mut tiles, &mut rng, GEM, (H as f32 * 0.78) as i32, H - 8, 22, 3.0);

    // Surface ponds in forest and snow.
    for _ in 0..5 {
        let cx = rng.range_i(50, W - 50);
        if (cx - W / 2).abs() < 16 {
            continue;
        }
        if biome[cx as usize] == BIOME_DESERT {
            continue;
        }
        let half = rng.range_i(4, 9);
        let depth = rng.range_i(2, 4);
        let ground = surface[cx as usize];
        for dx in -half..=half {
            let x = cx + dx;
            if x < 2 || x >= W - 2 {
                continue;
            }
            let fall = ((half - dx.abs()) as f32 / half as f32 * depth as f32).round() as i32;
            for d in 0..=fall {
                let y = ground + d;
                if World::in_bounds(x, y) {
                    let i = World::idx(x, y);
                    if d == fall {
                        tiles[i] = if biome[x as usize] == BIOME_FOREST { CLAY } else { SNOW };
                    } else {
                        tiles[i] = WATER;
                    }
                }
            }
            surface[x as usize] = ground + fall;
            // Restore dirt below the pond bottom.
            let by = ground + fall + 1;
            for yy in by..(by + 4).min(H) {
                let i = World::idx(x, yy);
                if tiles[i] == AIR {
                    tiles[i] = STONE;
                }
            }
        }
        // Keep water from spilling at the edges by adding a lip.
        for dx in [-(half + 1), half + 1] {
            let x = cx + dx;
            if x >= 0 && x < W {
                let y = ground;
                if World::in_bounds(x, y) {
                    let i = World::idx(x, y);
                    if tiles[i] == AIR {
                        tiles[i] = DIRT;
                    }
                }
            }
        }
    }

    // Trees, cacti and giant mushrooms.
    let mut x = 6;
    while x < W - 6 {
        let b = biome[x as usize];
        let ground = surface[x as usize];
        let top = get_t(tiles.as_slice(), x, ground);
        let above_clear = get_t(tiles.as_slice(), x, ground - 1) == AIR && get_t(tiles.as_slice(), x, ground - 2) == AIR;
        if above_clear && ((b == BIOME_FOREST && top == GRASS) || (b == BIOME_SNOW && top == SNOW)) {
            if rng.chance(if b == BIOME_FOREST { 0.34 } else { 0.22 }) {
                grow_tree(&mut tiles, &mut walls, x, ground, seed, &mut rng, b);
                x += rng.range_i(3, 7);
                continue;
            }
        } else if b == BIOME_DESERT && top == SAND && above_clear && rng.chance(0.10) {
            let h = rng.range_i(2, 5);
            for i in 1..=h {
                let y = ground - i;
                if World::in_bounds(x, y) && get_t(tiles.as_slice(), x, y) == AIR {
                    set_t(&mut tiles, x, y, CACTUS);
                }
            }
            x += rng.range_i(3, 6);
            continue;
        }
        x += 1;
    }

    // Glow mushrooms in caves.
    for _ in 0..260 {
        let x = rng.range_i(4, W - 4);
        let y = rng.range_i((surface[x as usize] + 6).min(H - 4), H - 4);
        if get_t(tiles.as_slice(), x, y) == AIR
            && !is_solid(get_t(tiles.as_slice(), x, y + 1))
            && is_solid(get_t(tiles.as_slice(), x, y + 2))
            && walls[World::idx(x, y)] != WALL_NONE
        {
            set_t(&mut tiles, x, y, MUSHROOM);
        }
    }

    // Small abandoned cabins with a torch and workbench.
    for _ in 0..4 {
        let x = rng.range_i(50, W - 50);
        let y = rng.range_i((H as f32 * 0.55) as i32, H - 16);
        build_cabin(&mut tiles, &mut walls, x, y, &mut rng);
    }

    // Bedrock floor.
    for x in 0..W {
        let jag = (hash2(x, 0, seed) * 4.0) as i32;
        for y in (H - 4 - jag)..H {
            if World::in_bounds(x, y) {
                let i = World::idx(x, y);
                tiles[i] = BEDROCK;
                walls[i] = WALL_NONE;
            }
        }
    }

    // Recompute the surface cache after caves/ponds and find a spawn point.
    for x in 0..W {
        let mut top = H;
        for y in 0..H {
            let t = tiles[World::idx(x, y)];
            if is_solid(t) || t == WATER {
                top = y;
                break;
            }
        }
        surface[x as usize] = top;
    }

    let mut spawn_x = W / 2;
    let mut best = i32::MAX;
    for x in (W / 2 - 60)..(W / 2 + 60) {
        let b = biome[x as usize];
        let top = surface[x as usize];
        if b == BIOME_FOREST && top < H - 20 {
            let left = surface[(x - 1) as usize];
            let right = surface[(x + 1) as usize];
            let flatness = (left - top).abs() + (right - top).abs();
            if flatness < best {
                best = flatness;
                spawn_x = x;
            }
        }
    }
    if best == i32::MAX {
        spawn_x = W / 2;
    }
    let ground_y = surface[spawn_x as usize];
    let mut spawn = V2::new(spawn_x as f32 * TILE as f32 + TILE as f32 * 0.5, (ground_y as f32 - 2.2) * TILE as f32);

    // Ensure there is a solid pad under the spawn.
    for yy in ground_y..(ground_y + 3).min(H) {
        let i = World::idx(spawn_x, yy);
        if !is_solid(tiles[i]) {
            tiles[i] = DIRT;
        }
    }
    if get_t(tiles.as_slice(), spawn_x, ground_y - 1) != AIR {
        let i = World::idx(spawn_x, ground_y - 1);
        tiles[i] = AIR;
    }
    spawn.y = (ground_y as f32 - 2.0) * TILE as f32;

    let mut sky_top = vec![H; W as usize];
    for x in 0..W {
        let mut top = H;
        for y in 0..H {
            let t = tiles[World::idx(x, y)];
            if is_solid(t) || t == WATER {
                top = y;
                break;
            }
        }
        sky_top[x as usize] = top;
        surface[x as usize] = top;
    }

    let mut world = World {
        tiles,
        walls,
        surface,
        sky_top,
        biome,
        seed,
        spawn,
        modified: Vec::new(),
        minimap: vec![0; (W * H * 4) as usize],
        map_dirty: true,
    };
    world.rebuild_minimap();
    world
}

fn get_t(tiles: &[u8], x: i32, y: i32) -> u8 {
    if World::in_bounds(x, y) {
        tiles[World::idx(x, y)]
    } else {
        AIR
    }
}

fn set_t(tiles: &mut [u8], x: i32, y: i32, t: u8) {
    if World::in_bounds(x, y) {
        tiles[World::idx(x, y)] = t;
    }
}

fn carve_circle(tiles: &mut [u8], walls: &mut [u8], cx: f32, cy: f32, r: f32, seed: u32) {
    let x0 = (cx - r - 1.0) as i32;
    let x1 = (cx + r + 1.0) as i32;
    let y0 = (cy - r - 1.0) as i32;
    let y1 = (cy + r + 1.0) as i32;
    for y in y0..=y1 {
        for x in x0..=x1 {
            if !World::in_bounds(x, y) {
                continue;
            }
            let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
            if d <= r {
                let i = World::idx(x, y);
                if is_solid(tiles[i]) {
                    if walls[i] == WALL_NONE {
                        walls[i] = if y > (H / 2) { WALL_STONE } else { WALL_DIRT };
                    }
                    tiles[i] = AIR;
                } else if tiles[i] == AIR {
                    // Keep background walls behind natural caverns.
                    if walls[i] == WALL_NONE && y > 8 {
                        walls[i] = if y > (H / 2) { WALL_STONE } else { WALL_DIRT };
                    }
                }
            }
            // Rough the rim a little.
            if d > r && d <= r + 0.75 && hash2(x, y, seed) > 0.82 {
                let i = World::idx(x, y);
                if is_solid(tiles[i]) {
                    tiles[i] = AIR;
                }
            }
        }
    }
}

fn grow_tree(tiles: &mut [u8], walls: &mut [u8], x: i32, ground: i32, seed: u32, rng: &mut Rng, biome: u8) {
    let height = rng.range_i(5, 11);
    let pine = biome == BIOME_SNOW;
    for i in 1..=height {
        let y = ground - i;
        if !World::in_bounds(x, y) {
            return;
        }
        set_t(tiles, x, y, WOOD);
    }
    let top = ground - height;
    if pine {
        for layer in 0..4 {
            let y = top - layer;
            let r = 2 + (3 - layer) as i32;
            for dx in -r..=r {
                for dy in -1i32..=1 {
                    if dx.abs() + dy.abs() * 2 <= r + 1 {
                        let tx = x + dx;
                        let ty = y + dy;
                        if World::in_bounds(tx, ty) && get_t(tiles, tx, ty) == AIR {
                            set_t(tiles, tx, ty, LEAVES);
                        }
                    }
                }
            }
        }
        set_t(tiles, x, top - 4, LEAVES);
        set_t(tiles, x, top - 5, LEAVES);
    } else {
        let r = rng.range_i(2, 4);
        for dy in -r..=r {
            for dx in -r..=r {
                let d = (dx * dx + dy * dy) as f32;
                if d <= (r * r) as f32 + 1.0 && hash2(x + dx, top + dy, seed ^ 0x77) > 0.18 {
                    let tx = x + dx;
                    let ty = top + dy - 1;
                    if World::in_bounds(tx, ty) && get_t(tiles, tx, ty) == AIR {
                        set_t(tiles, tx, ty, LEAVES);
                    }
                }
            }
        }
        // A couple of branches.
        for side in [-1, 1] {
            if rng.chance(0.7) {
                let by = ground - height / 2;
                for step in 1..=2 {
                    set_t(tiles, x + side * step, by - step + 1, WOOD);
                }
            }
        }
    }
    // Roots / dirt collar.
    for dx in -1..=1 {
        let wx = x + dx;
        let wy = ground;
        if World::in_bounds(wx, wy) && get_t(tiles, wx, wy) == AIR {
            // no-op, keeps trunks on the ground
        }
    }
    let _ = walls;
}

fn build_cabin(tiles: &mut [u8], walls: &mut [u8], cx: i32, cy: i32, rng: &mut Rng) {
    let w = rng.range_i(7, 10);
    let h = rng.range_i(5, 6);
    let x0 = cx - w / 2;
    let y0 = cy - h;
    // Clear and build walls.
    for y in y0..=(cy + 1) {
        for x in x0..=(x0 + w) {
            if !World::in_bounds(x, y) {
                continue;
            }
            let i = World::idx(x, y);
            tiles[i] = AIR;
            walls[i] = WALL_WOOD;
        }
    }
    // Floor.
    for x in x0..=(x0 + w) {
        let y = cy + 1;
        if World::in_bounds(x, y) {
            let i = World::idx(x, y);
            if !is_solid(tiles[i]) {
                tiles[i] = WOOD;
            }
        }
    }
    // Side pillars.
    for y in y0..=cy {
        for x in [x0, x0 + w] {
            if World::in_bounds(x, y) {
                let i = World::idx(x, y);
                tiles[i] = WOOD;
            }
        }
    }
    // Roof.
    for x in x0..=(x0 + w) {
        if World::in_bounds(x, y0 - 1) {
            let i = World::idx(x, y0 - 1);
            tiles[i] = WOOD;
        }
    }
    // Doorway gaps.
    for y in (cy - 2)..=cy {
        let i = World::idx(x0, y);
        if y >= cy - 1 {
            let idx = World::idx(x0, y);
            if World::in_bounds(x0, y) {
                tiles[idx] = AIR;
                let _ = i;
            }
        }
    }
    // Furniture.
    if World::in_bounds(x0 + 2, cy) {
        let i = World::idx(x0 + 2, cy);
        if tiles[i] == AIR {
            tiles[i] = BENCH;
        }
    }
    if World::in_bounds(x0 + w - 2, cy - 3) {
        let i = World::idx(x0 + w - 2, cy - 3);
        if tiles[i] == AIR {
            tiles[i] = TORCH;
        }
    }
    let _ = rng;
}

// ---------------------------------------------------------------------------
// Headless tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_generates_with_spawn_support() {
        let w = generate(12345);
        let sx = (w.spawn.x / TILE as f32) as i32;
        let sy = (w.spawn.y / TILE as f32) as i32;
        assert!(World::in_bounds(sx, sy));
        let mut found = false;
        for y in (sy + 1)..(sy + 5) {
            if w.solid(sx, y) {
                found = true;
            }
        }
        assert!(found, "spawn must have ground below");
        assert!(w.solid(sx, sy + 3));
    }

    #[test]
    fn world_has_ores_and_trees() {
        let w = generate(777);
        let mut ores = 0;
        let mut trees = 0;
        let mut water = 0;
        for t in &w.tiles {
            if matches!(*t, COPPER | IRON | GOLD | GEM) {
                ores += 1;
            }
            if *t == WOOD {
                trees += 1;
            }
            if *t == WATER {
                water += 1;
            }
        }
        assert!(ores > 500, "ores: {ores}");
        assert!(trees > 100, "trees: {trees}");
        assert!(water > 0);
    }

    #[test]
    fn tile_set_updates_cache_and_surface() {
        let mut w = generate(5);
        let x = 100;
        let top = w.surface[x as usize];
        assert!(w.solid(x, top));
        w.set_tile(x, top, AIR);
        assert!(!w.solid(x, top));
        assert!(w.sky_top[x as usize] != top || w.solid(x, top));
        w.set_tile(x, top, STONE);
        assert!(w.solid(x, top));
    }

    #[test]
    fn hardness_ordering() {
        assert!(hardness(DIRT) < hardness(STONE));
        assert!(hardness(STONE) < hardness(GOLD));
        assert!(hardness(BEDROCK) > 1000.0);
    }
}
