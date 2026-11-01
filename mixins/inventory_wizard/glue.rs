// Between Horizon's `Game` and the mod's logic (`runtime.rs`, added as
// `game::mod_inventory_wizard`): which slots a sort covers, the items in them
// as `Kind`s, the clicks, the feedback, and the screen's buttons and locks.
// Added as `game::mod_inventory_wizard_glue`, so the game's own (module
// private) fields and methods are in reach.
use crate::game::mod_inventory_wizard as m;
use crate::game::native_hooks::NativeMod;
use crate::game::{Game, Screen};
use core::ops::ControlFlow;
use horizon_client::{
    containers::BlockEntity,
    hud::{HudBatch, rgba},
    items::ItemStack,
    keybinds,
    menu::Input,
    text::Text,
};
use winit::{event::MouseButton, keyboard::KeyCode};

/// Game slots of the open container.
const CONTAINER: std::ops::Range<usize> = 300..500;
/// The mode button's side, as Horizon's sort buttons.
const BUTTON: f32 = 9.0;

fn read(game: &Game<'_>) -> m::Settings {
    m::settings(|key| game.native_setting(m::ID, key))
}

fn save(game: &Game<'_>, settings: m::Settings) {
    for (key, value) in settings.stored() {
        game.native_set_setting(m::ID, key, &value);
    }
    m::set_settings(settings);
}

/// The sort sound and the action bar line, as the Java mod's `show`.
fn show(game: &mut Game<'_>, settings: &m::Settings, feedback: m::Feedback) {
    if settings.sound {
        game.audio
            .play(m::SOUND, None, m::SOUND_VOLUME, m::SOUND_PITCH + feedback.pitch);
    }
    if settings.messages {
        game.titles.set_overlay(Text::literal(feedback.text), false);
    }
}

/// The open screen's slots take ordinary clicks: the player's inventory (with
/// or without its screen) or a chest-like container, not a station's menu.
fn clickable(game: &Game<'_>) -> bool {
    if game.station.is_some() || game.mount_active() || game.screen == Screen::Creative {
        return false;
    }
    if game.screen != Screen::Inventory {
        return true;
    }
    if game.remote.is_some() {
        game.remote_storage_open()
    } else {
        matches!(game.container_kind(), None | Some(BlockEntity::Chest(_)))
    }
}

/// The open container: None without one, `Some(None)` for one that isn't sorted.
fn open_container(game: &Game<'_>) -> Option<Option<m::ContainerKind>> {
    if game.screen != Screen::Inventory {
        return None;
    }
    if game.remote.is_some() {
        // A server's menu type doesn't tell chests, barrels and ender chests apart
        return match game.remote_menu_type() {
            None | Some("inventory") => None,
            Some("minecraft:shulker_box") => Some(Some(m::ContainerKind::ShulkerBox)),
            Some(t) if t.starts_with("minecraft:generic_9x") => Some(Some(m::ContainerKind::Chest)),
            Some(_) => Some(None),
        };
    }
    let k = game.container?;
    if game.open_cart_kind() == Some(crate::game::minecarts::CartKind::Chest) {
        return Some(Some(m::ContainerKind::Chest));
    }
    if game.chest_rows().is_none() && !game.container_block_is_shulker() {
        return Some(None);
    }
    let block = game.res.facts().get(game.world.block(k.0, k.1, k.2)).name.clone();
    Some(m::ContainerKind::of_block(&block))
}

/// The item's facts and kind, from the world's item tables.
fn kind(game: &Game<'_>, stack: &ItemStack) -> m::Kind {
    let items = game.items();
    let info = items.get(&stack.item);
    let tag = |tag: &str| items.item_in_tag(tag, &stack.item);
    let facts = m::Facts {
        id: &stack.item,
        block: info.is_some_and(|i| i.block.is_some()),
        food: info.is_some_and(|i| i.food.is_some()),
        tool: info.is_some_and(|i| i.tool_damage_per_block.is_some()),
        glider: info.is_some_and(|i| i.glider),
        blocks_attacks: info.is_some_and(|i| i.blocks_attacks.is_some()),
        tag: &tag,
    };
    m::Kind::new(&facts, game.max_stack(stack))
}

/// Sorts `slots` (game slots, in order) as `region`: Err when the clicks
/// couldn't do it.
fn sort_slots(
    game: &mut Game<'_>,
    slots: &[usize],
    region: m::Region,
    settings: &m::Settings,
) -> Result<(), ()> {
    if game.carried.is_some() {
        return Err(());
    }
    let views: Vec<Option<ItemStack>> = slots.iter().map(|&s| game.slot_view(s)).collect();
    let mut stacks: Vec<ItemStack> = Vec::new();
    let mut kinds: Vec<m::Kind> = Vec::new();
    let mut cells: Vec<m::Slot> = Vec::with_capacity(views.len());
    for view in &views {
        let Some(stack) = view else {
            cells.push(None);
            continue;
        };
        let i = match stacks.iter().position(|s| s.same_item_same_components(stack)) {
            Some(i) => i,
            None => {
                kinds.push(kind(game, stack));
                stacks.push(stack.clone());
                stacks.len() - 1
            }
        };
        cells.push(Some((i, stack.count)));
    }
    let target = m::arrange(&cells, &kinds, settings.mode, region, settings).ok_or(())?;
    let merge = settings.mode != m::SortMode::StackBased;
    let clicks = m::plan(&cells, &target, &kinds, merge).ok_or(())?;
    for i in clicks {
        if !game.tweak_click(slots[i], 0, Input::Pickup) {
            return Err(());
        }
    }
    Ok(())
}

/// `SortActions.sortPlayer`: the main inventory, then the hotbar, each
/// without its locked slots.
fn sort_player(game: &mut Game<'_>, hotbar: bool, inventory: bool) {
    let settings = read(game);
    let hotbar = hotbar && settings.hotbar;
    let inventory = inventory && settings.inventory;
    if !hotbar && !inventory {
        return show(game, &settings, m::Feedback::disabled());
    }
    if !clickable(game) {
        return show(game, &settings, m::Feedback::unsupported_container());
    }
    let mut done = Ok(());
    if inventory {
        let slots = settings.unlocked(m::INVENTORY);
        done = sort_slots(game, &slots, m::Region::Inventory, &settings);
    }
    if hotbar && done.is_ok() {
        let slots = settings.unlocked(m::HOTBAR);
        done = sort_slots(game, &slots, m::Region::Hotbar, &settings);
    }
    let feedback = match done {
        Ok(()) => m::Feedback::player_sorted(hotbar, inventory, settings.mode),
        Err(()) => m::Feedback::could_not_sort(),
    };
    show(game, &settings, feedback);
}

/// `SortActions.sortContainer`: the open container's slots.
fn sort_container(game: &mut Game<'_>) {
    let settings = read(game);
    let kind = match open_container(game) {
        None => return,
        Some(None) => return show(game, &settings, m::Feedback::unsupported_container()),
        Some(Some(kind)) => kind,
    };
    if !settings.container_enabled(kind) {
        return show(game, &settings, m::Feedback::disabled());
    }
    let slots: Vec<usize> = game.slots().into_iter().filter(|s| CONTAINER.contains(s)).collect();
    let feedback = match sort_slots(game, &slots, m::Region::Container, &settings) {
        Ok(()) => m::Feedback::container_sorted(kind, settings.mode),
        Err(()) => m::Feedback::could_not_sort(),
    };
    show(game, &settings, feedback);
}

/// `SortActions.sortSlot`: whatever `slot` belongs to (the hotbar sorts the
/// whole inventory). Without a slot, the open container, else everything.
fn sort_at(game: &mut Game<'_>, slot: Option<usize>) {
    match slot {
        Some(s) if m::HOTBAR.contains(&s) => sort_player(game, true, true),
        Some(s) if m::INVENTORY.contains(&s) => sort_player(game, false, true),
        Some(s) if CONTAINER.contains(&s) => sort_container(game),
        _ if open_container(game).is_some() => sort_container(game),
        _ => sort_player(game, true, true),
    }
}

fn cycle_mode(game: &mut Game<'_>) {
    let mut settings = read(game);
    settings.mode = settings.mode.next();
    let feedback = m::Feedback::mode_changed(settings.mode);
    save(game, settings.clone());
    show(game, &settings, feedback);
}

fn toggle_lock(game: &mut Game<'_>, slot: usize) {
    let mut settings = read(game);
    let locked = settings.toggle_lock(slot);
    save(game, settings.clone());
    show(game, &settings, m::Feedback::lock(locked));
}

/// A key or mouse button by its vanilla name: the sort key sorts what's under
/// the cursor (everything while playing), the mode key cycles the mode.
fn on_input(game: &mut Game<'_>, name: &str) -> bool {
    let playing = game.screen == Screen::Playing;
    if !(playing || (game.screen == Screen::Inventory && !game.recipe_search_focused())) {
        return false;
    }
    if name == game.native_mod_key(m::ID, m::KEY_MODE, m::KEY_MODE_DEFAULT) {
        cycle_mode(game);
        return true;
    }
    if name != game.native_mod_key(m::ID, m::KEY_SORT, m::KEY_SORT_DEFAULT) || !clickable(game) {
        return false;
    }
    if playing {
        sort_player(game, true, true);
    } else if game.carried.is_none() {
        let slot = game.inventory_target().0;
        sort_at(game, slot);
    }
    true
}

/// Hook: `tweaks::Game::sort_key_press`, before Horizon's own sort key.
pub(super) fn on_key(game: &mut Game<'_>, key: KeyCode) -> ControlFlow<bool> {
    if !game.native_on(m::ID) {
        return ControlFlow::Continue(());
    }
    match keybinds::key_name(key) {
        Some(name) if on_input(game, name) => ControlFlow::Break(true),
        _ => ControlFlow::Continue(()),
    }
}

/// Hook: `tweaks::Game::sort_mouse_press`, every button press in a screen
/// before it clicks a slot. Alt+Left Click locks a player slot, Shift+Right
/// Click sorts the part under the cursor (on the middle hotbar slot: cycles
/// the mode), and the sort and mode keys may be mouse buttons.
pub(super) fn on_mouse(game: &mut Game<'_>, button: MouseButton) -> ControlFlow<bool> {
    if !game.native_on(m::ID) || game.screen != Screen::Inventory || !clickable(game) {
        return ControlFlow::Continue(());
    }
    let slot = game.inventory_target().0;
    let player_slot = slot.filter(|s| (0..36).contains(s));
    if button == MouseButton::Left && game.modifiers.alt_key() {
        let Some(slot) = player_slot else {
            return ControlFlow::Continue(());
        };
        toggle_lock(game, slot);
        return ControlFlow::Break(true);
    }
    if button == MouseButton::Right && game.has_shift_down() {
        let sortable = player_slot.is_some()
            || (slot.is_some_and(|s| CONTAINER.contains(&s)) && open_container(game).is_some());
        if !sortable {
            return ControlFlow::Continue(());
        }
        if slot == Some(m::CYCLE_MODE_HOTBAR_SLOT) {
            cycle_mode(game);
        } else if game.carried.is_none() {
            sort_at(game, slot);
        }
        return ControlFlow::Break(true);
    }
    if on_input(game, &keybinds::mouse_name(button)) {
        return ControlFlow::Break(true);
    }
    ControlFlow::Continue(())
}

/// Hook: `tweaks::Game::sort_slots`, Horizon's sort buttons: the
/// container's sorts it, the inventory's the main inventory.
pub(super) fn on_sort_slots(game: &mut Game<'_>, slots: &[usize]) -> ControlFlow<()> {
    if !game.native_on(m::ID) {
        return ControlFlow::Continue(());
    }
    if slots.iter().any(|s| CONTAINER.contains(s)) {
        sort_container(game);
    } else {
        sort_player(game, false, true);
    }
    ControlFlow::Break(())
}

/// The mode button's corner, left of Horizon's inventory sort button, when
/// that button shows.
fn mode_button(game: &Game<'_>, ox: f32, oy: f32) -> Option<(f32, f32)> {
    let shown = game.settings.extras.sort
        && game.screen == Screen::Inventory
        && game.station.is_none()
        && !game.mount_active()
        && if game.remote.is_some() {
            game.remote_storage_open()
        } else {
            matches!(game.container_kind(), None | Some(BlockEntity::Chest(_)))
        };
    shown.then(|| (ox + 160.0 - BUTTON - 2.0, oy + game.screen_height() - 95.0))
}

/// Hook: `tweaks::Game::sort_press`, a left press in the inventory: the mode
/// button cycles the mode.
pub(super) fn on_sort_press(game: &mut Game<'_>) -> ControlFlow<bool> {
    if !game.native_on(m::ID) {
        return ControlFlow::Continue(());
    }
    let Some(r) = game.renderer.as_ref() else {
        return ControlFlow::Continue(());
    };
    let batch = HudBatch::new([r.config.width, r.config.height]);
    let (ox, oy) = game.inventory_origin(&batch);
    let (cx, cy) = game.cursor;
    match mode_button(game, ox, oy) {
        Some((x, y)) if cx >= x && cx < x + BUTTON && cy >= y && cy < y + BUTTON => {
            cycle_mode(game);
            ControlFlow::Break(true)
        }
        _ => ControlFlow::Continue(()),
    }
}

/// Hook: `tweaks::Game::draw_sort_buttons`, after the slots are drawn: a
/// lock over each locked slot, and the mode button with its mode's letter.
pub(super) fn on_draw_buttons(game: &Game<'_>, batch: &mut HudBatch, ox: f32, oy: f32) {
    if !game.native_on(m::ID) || game.screen != Screen::Inventory || !clickable(game) {
        return;
    }
    let settings = read(game);
    let atlas = &game.gui.atlas;
    for &slot in &settings.locked {
        let (sx, sy) = game.slot_position(slot);
        let (x, y) = (ox + sx + 9.0, oy + sy);
        // A small padlock in the slot's top-right corner
        batch.fill(atlas, x + 1.0, y, 5.0, 1.0, rgba(255, 210, 60, 255));
        batch.fill(atlas, x + 1.0, y, 1.0, 3.0, rgba(255, 210, 60, 255));
        batch.fill(atlas, x + 5.0, y, 1.0, 3.0, rgba(255, 210, 60, 255));
        batch.fill(atlas, x, y + 3.0, 7.0, 4.0, rgba(255, 210, 60, 255));
        batch.fill(atlas, x + 3.0, y + 4.0, 1.0, 2.0, rgba(90, 60, 0, 255));
    }
    let Some((x, y)) = mode_button(game, ox, oy) else {
        return;
    };
    let (cx, cy) = game.cursor;
    let hovered = cx >= x && cx < x + BUTTON && cy >= y && cy < y + BUTTON;
    let face = if hovered { rgba(220, 220, 220, 255) } else { rgba(160, 160, 160, 255) };
    batch.fill(atlas, x, y, BUTTON, BUTTON, rgba(55, 55, 55, 255));
    batch.fill(atlas, x + 1.0, y + 1.0, BUTTON - 2.0, BUTTON - 2.0, face);
    let letter = settings.mode.letter();
    let lx = x + ((BUTTON - game.native_text_width(letter)) / 2.0).floor() + 1.0;
    game.native_text(batch, letter, [lx, y + 1.0], 0x37_3737, false);
}

/// Hook: `native_frame`. Horizon's sort buttons (the extra) show while the
/// mod's buttons are on; the player's own choice comes back when the mod is
/// turned off.
pub(super) fn on_frame(game: &mut Game<'_>) {
    if !game.native_on(m::ID) {
        if let Some(own) = m::own_sort_buttons(false, true) {
            game.settings.extras.sort = own;
        }
        return;
    }
    let settings = read(game);
    m::own_sort_buttons(game.settings.extras.sort, false);
    game.settings.extras.sort = settings.buttons;
}

/// Hook: `native_command`: `/iwiz [hotbar|inventory|all|mode]`.
pub(super) fn on_command(game: &mut Game<'_>, command: &str, ran: &mut bool) {
    if !game.native_on(m::ID) {
        return;
    }
    let Some(parsed) = m::parse_command(command) else {
        return;
    };
    *ran = true;
    match parsed {
        Err(usage) => game.native_chat(usage),
        Ok(m::Command::Mode) => cycle_mode(game),
        Ok(m::Command::Hotbar) => sort_player(game, true, false),
        Ok(m::Command::Inventory) => sort_player(game, false, true),
        Ok(m::Command::All) => sort_player(game, true, true),
    }
}

/// Hook: `native_commands`: the chat box completes them.
pub(super) fn on_commands(commands: &mut Vec<&'static str>) {
    commands.extend(["iwiz", "inventorywizard"]);
}

/// Hook: `native_mods`: the row in Options > Mods, with the two keys.
pub(super) fn on_list(mods: &mut Vec<NativeMod>) {
    mods.push(NativeMod {
        id: m::ID.into(),
        title: format!("{} {}", m::NAME, m::VERSION),
        description: m::DESCRIPTION.into(),
        keys: vec![
            [m::KEY_SORT.into(), m::KEY_SORT_LABEL.into(), m::KEY_SORT_DEFAULT.into()],
            [m::KEY_MODE.into(), m::KEY_MODE_LABEL.into(), m::KEY_MODE_DEFAULT.into()],
        ],
    });
}

/// Hook: `native_settings`: the settings page's buttons.
pub(super) fn on_settings(game: &mut Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    if mod_id == m::ID {
        buttons.extend(read(game).labels());
    }
}

/// Hook: `native_settings_click`.
pub(super) fn on_click(game: &mut Game<'_>, mod_id: &str, button: usize) {
    if mod_id != m::ID {
        return;
    }
    let mut settings = read(game);
    settings.click(button);
    save(game, settings);
}
