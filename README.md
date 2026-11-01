# 🧙‍♂️ InventoryWizard for Horizon

InventoryWizard's one-click sorting as a native mod for [Horizon](https://github.com/horizon-rust/horizon), the Rust
reimplementation of Minecraft: Java Edition 26.3. Java mod jars don't run in Horizon, so this is a port of the mod's
logic to Rust, built into the game by the Horizon launcher.

## Features

- **Smart sorting** by category (stone, earth, wood, ores, blocks, tools, weapons, armor, food, redstone,
  transportation, decoration), from item tags and components, so other mods' items are sorted too
- **Survival-friendly hotbar**: weapons, tools, food, building blocks, utility items, then the rest
- **Stacks merged**, but enchanted or named items are never combined
- **Three modes**: Default, Alphabetical and Stack-based
- **Sort buttons** by the inventory and container titles, plus a mode button (it shows the mode's letter)
- **Locked slots**: **Alt+Click** a hotbar or inventory slot to lock it, and sorting leaves it alone
- **Keys**: **R** (or **Shift+Right Click**) sorts the part under the cursor: the hotbar sorts everything, the
  inventory the inventory, a container the container. Outside a screen, R sorts the whole inventory.
  **Shift+Right Click** on the middle hotbar slot cycles the mode. The mode also has its own key, unbound by default.
- **Commands**: `/iwiz [hotbar|inventory|all|mode]` (also `/inventorywizard`)
- **Works on** the inventory, hotbar, chests, barrels, shulker boxes and ender chests, **in singleplayer and on any
  server**: every sort is made of ordinary left clicks, as a player could click them, so a vanilla server accepts it

## Install

In the Horizon launcher: Mods → InventoryWizard → Install, then Settings → Game → Native mods (experimental) → Build.
Settings are under Options → Mods → InventoryWizard, and the keys can be rebound there.

## Differences from the Java mod

- **Works on servers without the Paper plugin.** The Java mod sorts on the integrated server, or sends `/iwiz` to the
  plugin. This port clicks the slots instead, which works everywhere.
- **Not ported**: scrolling to adjust a held stack (Horizon's own Mouse Tweaks-style wheel extra does something
  similar), the update checker, usage stats, and the sound choice (it's always the button click).
- **On a server**, chests, barrels and ender chests all open the same menu type, so they share the *Chest Sorting*
  toggle and the "Chest sorted" message.
- **Hotbar priorities** aren't on the settings page, as in the Java mod. Edit the `hotbar_*` files in
  `horizon-mods/.horizon-mod-data/inventory_wizard/` (lower goes further left).
- A sort needs an empty hand (no stack on the cursor). In Stack-based mode, a few layouts can't be reached with
  left clicks alone: a region holding nothing but partial stacks of one item, with no empty slot. Nothing moves then,
  and the action bar says so.

## Made on

`compatibility.json` pins the Horizon commit the mod was made on. Besides Horizon's native hook points it hooks
Horizon's inventory extras in `crates/horizon-client/src/game/tweaks.rs` (`sort_key_press`, `sort_mouse_press`,
`sort_slots`, `sort_press`, `draw_sort_buttons`), because the hook points have no inventory screen input yet. Those
functions aren't a stable API, so a Horizon release that changes them needs a new version of this mod.

| File | What it is |
|---|---|
| `mixins/inventory_wizard/horizon-mixin.toml` | The bundle: two modules and the hooks |
| `mixins/inventory_wizard/runtime.rs` | The logic: categories, the three modes, the click planner, settings. Added to the game as `game::mod_inventory_wizard`; also this crate's library, for its tests |
| `mixins/inventory_wizard/glue.rs` | Between Horizon's `Game` and the logic: slots, items, clicks, feedback, buttons and locks |
| `mixins/inventory_wizard/*.rs` | One small helper per hook |

## Build and test

```sh
cargo test                                                           # the logic
python3 native/tools/native.py check /path/to/InventoryWizard-Horizon     # from horizon-mods: apply on Horizon, cargo check
horizon-launcher native-build /path/to/InventoryWizard-Horizon/mixins/inventory_wizard --trust-native-code
```

`native.py` needs Python 3.11+, Rust and rsync (WSL on Windows).

## Package for the registry

```sh
python3 package.py      # target/inventory_wizard-<version>.hnmod, and its SHA-256
```

The same deterministic file as horizon-mods' `native.py package`, and it runs on Windows. Submit it at
[playhorizon.dev/mods/submit](https://playhorizon.dev/mods/submit).
