# PlayGround

<!-- omgithub:readme:start -->
## 🚀 Build, play, and remix with OMGithub

**Created using [OMGithub.com](https://go.ondev.store/).**

[![OMGithub](https://img.shields.io/badge/OMGithub-Open%20project-orange?style=for-the-badge)](https://go.ondev.store/Dingus-alt/PlayGround)
[![GitHub](https://img.shields.io/badge/GitHub-Source-181717?logo=github&style=for-the-badge)](https://github.com/Dingus-alt/PlayGround)

- 🎮 [Open the project](https://go.ondev.store/Dingus-alt/PlayGround).
- ✨ [Remix this project](https://go.ondev.store/Dingus-alt/PlayGround?remix=1).
- 💻 [Explore the source](https://github.com/Dingus-alt/PlayGround).
- 🛠️ [Check build runs](https://github.com/Dingus-alt/PlayGround/actions).
- 🐛 [Report an issue](https://github.com/Dingus-alt/PlayGround/issues).
- 👤 [Explore the creator's projects](https://go.ondev.store/Dingus-alt).
- 🌍 [Create with OMGithub](https://go.ondev.store/).
<!-- omgithub:readme:end -->

# Terraforge

A complete, playable Terraria-style 2D sandbox adventure written in **Rust**, compiled to
**WebAssembly**, and rendered with **Canvas2D**. The whole simulation — world generation,
physics, lighting, enemies, crafting, the boss fight, UI, and saving — lives in Rust. A small
JavaScript bridge owns the canvas, keyboard/mouse/touch input, procedural audio, and
`localStorage`.

## Play

```bash
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/terraforge.wasm pkg/terraforge.wasm
python3 -m http.server 3002 --bind 127.0.0.1
```

Open <http://127.0.0.1:3002>. No Node build step, package manager, or external assets are
required. Serve the folder over HTTP; opening `index.html` via `file://` will not work because
the browser blocks the WASM fetch.

### Controls

| Action | Desktop | Touch |
| --- | --- | --- |
| Move | `A` / `D` | ◀ / ▶ |
| Jump / swim | `Space` | ▲ |
| Mine / attack / use | Hold left mouse | Tap or drag on the world |
| Place block | Right mouse | Select a block, tap the world |
| Hotbar | `1`–`0` or mouse wheel | Tap a hotbar slot |
| Inventory & crafting | `E` or `C` | ▤ |
| World map | `M` | ◎ |
| Pause / save and quit | `Esc` | Ⅱ |
| Mute | `N` | ♪ |

### The loop

1. Chop a tree for wood.
2. Craft a workbench (`E`), place it, and stand nearby.
3. Forge a copper broadsword and better gear.
4. Gather gel from slimes and gold ore from caves.
5. Craft the Slime Crown, summon **The Slime King**, and defeat it.
6. Keep exploring — or start a fresh seed from the title screen.

The world is 900 × 300 tiles and procedurally includes forests, deserts, snow, caves, ponds,
ores, cabins, mushrooms, and bedrock. Day/night changes enemy behavior. Torches, bombs,
platforms, potions, and three sword tiers are craftable.

## Saves

Progress is saved automatically every ~20 seconds and whenever you save and quit. Saves include
the seed, world edits, player position/health, quest flags, statistics, every inventory slot, and
the selected hotbar slot. Data stays in the browser under `terraforge.save.v1`; there is no
server or account.

## Project layout

- `src/world.rs` — procedural generation and tile queries
- `src/player.rs` — movement, collision, mining, health
- `src/entities.rs` — enemies, drops, bombs, particles
- `src/items.rs` — items, inventory, recipes
- `src/game.rs` — simulation loop, quests, boss, save/load
- `src/render.rs`, `src/ui.rs` — world rendering, lighting, HUD and menus
- `src/gfx.rs`, `src/lib.rs` — Canvas2D bridge and WASM exports
- `web/main.js`, `web/style.css`, `index.html` — browser shell, input, audio, title/touch UI
- `pkg/terraforge.wasm` — built game artifact served by the page

## Assumptions

- “Terraria-style” means a side-view tile sandbox with mining, building, crafting, survival, and
  a boss goal, not a clone of Terraria’s content or assets.
- The game uses a fixed 900 × 300 world per seed and simple pixel-art primitives drawn at runtime,
  so it has no image or audio asset downloads.
- Touch controls are shown automatically on coarse-pointer devices and remain available on
  hybrid touch laptops; portrait phones get a compact utility row below the objective HUD.
- The debug API (`window.__terra`) is left in for testing; it is not part of normal gameplay.
