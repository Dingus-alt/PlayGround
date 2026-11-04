//! Items, stacks and the player's inventory.

use crate::world;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ItemId {
    Pickaxe = 0,
    RustyBlade = 1,
    CopperBlade = 2,
    IronBlade = 3,
    GoldBlade = 4,
    Torch = 5,
    Dirt = 6,
    Stone = 7,
    Wood = 8,
    Platform = 9,
    Bench = 10,
    CopperOre = 11,
    IronOre = 12,
    GoldOre = 13,
    Gel = 14,
    Mushroom = 15,
    Bomb = 16,
    Crown = 17,
    Trophy = 18,
    Potion = 19,
    Gem = 20,
    Sand = 21,
    Snow = 22,
    Clay = 23,
}

pub const ITEM_COUNT: usize = 24;

impl ItemId {
    pub fn from_u8(v: u8) -> Option<ItemId> {
        if (v as usize) < ITEM_COUNT {
            Some(unsafe { std::mem::transmute::<u8, ItemId>(v) })
        } else {
            None
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            ItemId::Pickaxe => "Stone Pickaxe",
            ItemId::RustyBlade => "Rusty Shortsword",
            ItemId::CopperBlade => "Copper Broadsword",
            ItemId::IronBlade => "Iron Broadsword",
            ItemId::GoldBlade => "Gilded Broadsword",
            ItemId::Torch => "Torch",
            ItemId::Dirt => "Dirt Block",
            ItemId::Stone => "Stone Block",
            ItemId::Wood => "Wood",
            ItemId::Platform => "Wood Platform",
            ItemId::Bench => "Workbench",
            ItemId::CopperOre => "Copper Ore",
            ItemId::IronOre => "Iron Ore",
            ItemId::GoldOre => "Gold Ore",
            ItemId::Gel => "Slime Gel",
            ItemId::Mushroom => "Glow Mushroom",
            ItemId::Bomb => "Bomb",
            ItemId::Crown => "Slime Crown",
            ItemId::Trophy => "Slime King Trophy",
            ItemId::Potion => "Healing Draught",
            ItemId::Gem => "Amethyst",
            ItemId::Sand => "Sand Block",
            ItemId::Snow => "Snow Block",
            ItemId::Clay => "Clay Block",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            ItemId::Pickaxe => "Pickaxe",
            ItemId::RustyBlade => "Rusty Sword",
            ItemId::CopperBlade => "Copper Sword",
            ItemId::IronBlade => "Iron Sword",
            ItemId::GoldBlade => "Gold Sword",
            ItemId::CopperOre => "Copper",
            ItemId::IronOre => "Iron",
            ItemId::GoldOre => "Gold",
            ItemId::Platform => "Platform",
            ItemId::Bench => "Workbench",
            ItemId::Crown => "Slime Crown",
            ItemId::Potion => "Draught",
            ItemId::Trophy => "Trophy",
            other => other.name(),
        }
    }

    pub fn max_stack(self) -> u32 {
        match self {
            ItemId::Pickaxe | ItemId::RustyBlade | ItemId::CopperBlade | ItemId::IronBlade | ItemId::GoldBlade | ItemId::Trophy => 1,
            _ => 999,
        }
    }

    pub fn color(self) -> u32 {
        use crate::gfx::rgb;
        match self {
            ItemId::Pickaxe => rgb(150, 160, 172),
            ItemId::RustyBlade => rgb(178, 138, 92),
            ItemId::CopperBlade => rgb(218, 128, 72),
            ItemId::IronBlade => rgb(205, 214, 224),
            ItemId::GoldBlade => rgb(248, 204, 72),
            ItemId::Torch => rgb(255, 168, 66),
            ItemId::Dirt => rgb(126, 88, 54),
            ItemId::Stone => rgb(128, 132, 138),
            ItemId::Wood => rgb(126, 88, 48),
            ItemId::Platform => rgb(176, 126, 70),
            ItemId::Bench => rgb(178, 132, 78),
            ItemId::CopperOre => rgb(220, 126, 70),
            ItemId::IronOre => rgb(210, 196, 176),
            ItemId::GoldOre => rgb(250, 208, 80),
            ItemId::Gel => rgb(86, 196, 158),
            ItemId::Mushroom => rgb(120, 210, 255),
            ItemId::Bomb => rgb(64, 68, 78),
            ItemId::Crown => rgb(250, 210, 74),
            ItemId::Trophy => rgb(250, 196, 60),
            ItemId::Potion => rgb(224, 74, 120),
            ItemId::Gem => rgb(180, 104, 255),
            ItemId::Sand => rgb(224, 200, 142),
            ItemId::Snow => rgb(226, 240, 252),
            ItemId::Clay => rgb(174, 112, 88),
        }
    }

    pub fn place_tile(self) -> Option<u8> {
        match self {
            ItemId::Torch => Some(world::TORCH),
            ItemId::Dirt => Some(world::DIRT),
            ItemId::Stone => Some(world::STONE),
            ItemId::Wood => Some(world::WOOD),
            ItemId::Platform => Some(world::PLATFORM),
            ItemId::Bench => Some(world::BENCH),
            ItemId::CopperOre => Some(world::COPPER),
            ItemId::IronOre => Some(world::IRON),
            ItemId::GoldOre => Some(world::GOLD),
            ItemId::Gem => Some(world::GEM),
            ItemId::Sand => Some(world::SAND),
            ItemId::Snow => Some(world::SNOW),
            ItemId::Clay => Some(world::CLAY),
            _ => None,
        }
    }

    pub fn sword_damage(self) -> i32 {
        match self {
            ItemId::RustyBlade => 16,
            ItemId::CopperBlade => 26,
            ItemId::IronBlade => 38,
            ItemId::GoldBlade => 54,
            _ => 0,
        }
    }

    pub fn is_sword(self) -> bool {
        self.sword_damage() > 0
    }

    pub fn is_pickaxe(self) -> bool {
        self == ItemId::Pickaxe
    }

    pub fn is_placeable(self) -> bool {
        self.place_tile().is_some()
    }

    pub fn use_kind(self) -> UseKind {
        if self.is_pickaxe() {
            UseKind::Mine
        } else if self.is_sword() {
            UseKind::Swing
        } else if self == ItemId::Bomb {
            UseKind::Bomb
        } else if self == ItemId::Potion {
            UseKind::Heal
        } else if self == ItemId::Crown {
            UseKind::Summon
        } else if self.is_placeable() {
            UseKind::Place
        } else {
            UseKind::None
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UseKind {
    Mine,
    Swing,
    Place,
    Bomb,
    Heal,
    Summon,
    None,
}

#[derive(Clone, Copy, Debug)]
pub struct ItemStack {
    pub id: ItemId,
    pub count: u32,
}

#[derive(Clone, Debug)]
pub struct Inventory {
    pub slots: Vec<Option<ItemStack>>,
    pub selected: usize,
}

pub const HOTBAR: usize = 10;
pub const INV_SLOTS: usize = 30;

impl Default for Inventory {
    fn default() -> Self {
        Self::new()
    }
}

impl Inventory {
    pub fn new() -> Self {
        let mut slots = vec![None; INV_SLOTS];
        slots[0] = Some(ItemStack { id: ItemId::Pickaxe, count: 1 });
        slots[1] = Some(ItemStack { id: ItemId::RustyBlade, count: 1 });
        slots[2] = Some(ItemStack { id: ItemId::Torch, count: 8 });
        Inventory { slots, selected: 0 }
    }

    pub fn selected_stack(&self) -> Option<ItemStack> {
        self.slots.get(self.selected).copied().flatten()
    }

    pub fn selected_item(&self) -> Option<ItemId> {
        self.selected_stack().map(|s| s.id)
    }

    /// Adds items to existing stacks first, then to empty slots. Returns the
    /// amount that did not fit.
    pub fn add(&mut self, id: ItemId, mut count: u32) -> u32 {
        if count == 0 {
            return 0;
        }
        let max = id.max_stack();
        if max > 1 {
            for slot in self.slots.iter_mut() {
                if count == 0 {
                    break;
                }
                if let Some(stack) = slot {
                    if stack.id == id && stack.count < max {
                        let room = max - stack.count;
                        let take = room.min(count);
                        stack.count += take;
                        count -= take;
                    }
                }
            }
        }
        for slot in self.slots.iter_mut() {
            if count == 0 {
                break;
            }
            if slot.is_none() {
                let take = max.min(count);
                *slot = Some(ItemStack { id, count: take });
                count -= take;
            }
        }
        count
    }

    pub fn count(&self, id: ItemId) -> u32 {
        self.slots
            .iter()
            .flatten()
            .filter(|s| s.id == id)
            .map(|s| s.count)
            .sum()
    }

    pub fn consume(&mut self, id: ItemId, mut count: u32) -> bool {
        if self.count(id) < count {
            return false;
        }
        for slot in self.slots.iter_mut() {
            if count == 0 {
                break;
            }
            if let Some(stack) = slot {
                if stack.id == id {
                    let take = stack.count.min(count);
                    stack.count -= take;
                    count -= take;
                    if stack.count == 0 {
                        *slot = None;
                    }
                }
            }
        }
        true
    }

    pub fn remove_selected_one(&mut self) -> Option<ItemId> {
        let id = self.slots.get(self.selected).and_then(|s| *s).map(|s| s.id)?;
        if let Some(stack) = self.slots[self.selected].as_mut() {
            stack.count = stack.count.saturating_sub(1);
            if stack.count == 0 {
                self.slots[self.selected] = None;
            }
        }
        Some(id)
    }

    pub fn has_room(&self, id: ItemId) -> bool {
        let max = id.max_stack();
        self.slots.iter().any(|slot| match slot {
            None => true,
            Some(s) => s.id == id && s.count < max,
        })
    }

    pub fn swap_slot(&mut self, index: usize) {
        if index >= INV_SLOTS || index == self.selected {
            return;
        }
        self.slots.swap(index, self.selected);
    }

    pub fn best_sword(&self) -> ItemId {
        let mut best = ItemId::RustyBlade;
        for stack in self.slots.iter().flatten() {
            if stack.id.is_sword() && stack.id.sword_damage() > best.sword_damage() {
                best = stack.id;
            }
        }
        best
    }
}
