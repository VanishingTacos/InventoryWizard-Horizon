// InventoryWizard for Horizon: the Java mod's sorting (category, alphabetical
// and stack-based modes, a survival-friendly hotbar, locked slots), worked
// out here and done in the game with ordinary left clicks, so it runs in
// singleplayer and on any server. This file is both the mod's Cargo library
// (its tests) and the module the bundle adds to the game; it knows nothing of
// Horizon's types; glue.rs turns the game's slots into `Slot`s and `Kind`s.

use std::cell::RefCell;
use std::collections::BTreeSet;

/// The mod's id (its settings folder), name and version.
pub const ID: &str = "inventory_wizard";
pub const NAME: &str = "InventoryWizard";
pub const VERSION: &str = "2.3.3";
pub const DESCRIPTION: &str = "One-click inventory sorting: R or Shift+Right Click sorts the part under the cursor, Alt+Click locks a slot, /iwiz [hotbar|inventory|all|mode].";

pub const KEY_SORT: &str = "sort";
pub const KEY_SORT_LABEL: &str = "Sort";
pub const KEY_SORT_DEFAULT: &str = "key.keyboard.r";
pub const KEY_MODE: &str = "cycle_mode";
pub const KEY_MODE_LABEL: &str = "Cycle sort mode";
pub const KEY_MODE_DEFAULT: &str = "key.keyboard.unknown";

/// The Java mod's Shift+Right Click on this hotbar slot cycles the mode.
pub const CYCLE_MODE_HOTBAR_SLOT: usize = 4;
pub const HOTBAR: std::ops::Range<usize> = 0..9;
pub const INVENTORY: std::ops::Range<usize> = 9..36;

/// The sort sound (`InventoryWizardConfig.Sound`'s defaults).
pub const SOUND: &str = "minecraft:ui.button.click";
pub const SOUND_VOLUME: f32 = 0.5;
pub const SOUND_PITCH: f32 = 1.2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortMode {
    Default,
    Alphabetical,
    StackBased,
}

impl SortMode {
    pub const ALL: [SortMode; 3] = [SortMode::Default, SortMode::Alphabetical, SortMode::StackBased];

    pub fn key(self) -> &'static str {
        match self {
            SortMode::Default => "default",
            SortMode::Alphabetical => "alphabetical",
            SortMode::StackBased => "stack_based",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            SortMode::Default => "Default",
            SortMode::Alphabetical => "Alphabetical",
            SortMode::StackBased => "Stack-based",
        }
    }
    /// The letter on the mode button.
    pub fn letter(self) -> &'static str {
        match self {
            SortMode::Default => "D",
            SortMode::Alphabetical => "A",
            SortMode::StackBased => "S",
        }
    }
    pub fn parse(s: &str) -> Option<SortMode> {
        SortMode::ALL.into_iter().find(|m| m.key() == s.trim())
    }
    pub fn next(self) -> SortMode {
        let i = SortMode::ALL.iter().position(|&m| m == self).unwrap_or(0);
        SortMode::ALL[(i + 1) % SortMode::ALL.len()]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Region {
    Container,
    Inventory,
    Hotbar,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainerKind {
    Chest,
    ShulkerBox,
    Barrel,
    EnderChest,
}

impl ContainerKind {
    pub fn name(self) -> &'static str {
        match self {
            ContainerKind::Chest => "Chest",
            ContainerKind::ShulkerBox => "Shulker Box",
            ContainerKind::Barrel => "Barrel",
            ContainerKind::EnderChest => "Ender Chest",
        }
    }
    /// The open block's kind, from its id; None for blocks that aren't sorted.
    pub fn of_block(block: &str) -> Option<ContainerKind> {
        match block {
            "minecraft:barrel" => Some(ContainerKind::Barrel),
            "minecraft:ender_chest" => Some(ContainerKind::EnderChest),
            b if b.ends_with("shulker_box") => Some(ContainerKind::ShulkerBox),
            b if b.ends_with("chest") => Some(ContainerKind::Chest),
            _ => None,
        }
    }
}

/// The hotbar groups, in `HotbarPriorities`' order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotbarGroup {
    Weapons,
    Tools,
    Food,
    BuildingBlocks,
    UtilityItems,
    Other,
}

impl HotbarGroup {
    pub const ALL: [HotbarGroup; 6] = [
        HotbarGroup::Weapons,
        HotbarGroup::Tools,
        HotbarGroup::Food,
        HotbarGroup::BuildingBlocks,
        HotbarGroup::UtilityItems,
        HotbarGroup::Other,
    ];
    /// The setting holding the group's priority (lower goes further left).
    pub fn setting(self) -> &'static str {
        match self {
            HotbarGroup::Weapons => "hotbar_weapons",
            HotbarGroup::Tools => "hotbar_tools",
            HotbarGroup::Food => "hotbar_food",
            HotbarGroup::BuildingBlocks => "hotbar_building_blocks",
            HotbarGroup::UtilityItems => "hotbar_utility_items",
            HotbarGroup::Other => "hotbar_other",
        }
    }
    pub fn default_priority(self) -> i32 {
        match self {
            HotbarGroup::Weapons => 100,
            HotbarGroup::Tools => 200,
            HotbarGroup::Food => 300,
            HotbarGroup::BuildingBlocks => 400,
            HotbarGroup::UtilityItems => 500,
            HotbarGroup::Other => 999,
        }
    }
}

/// What the categories ask of an item (its registry entry, not the stack's
/// own components).
pub struct Facts<'a> {
    /// `minecraft:stone`.
    pub id: &'a str,
    /// A `BlockItem`.
    pub block: bool,
    pub food: bool,
    /// Has the `tool` component.
    pub tool: bool,
    pub glider: bool,
    pub blocks_attacks: bool,
    /// Whether the item is in an item tag (`minecraft:swords`, no `#`).
    pub tag: &'a dyn Fn(&str) -> bool,
}

impl Facts<'_> {
    pub fn path(&self) -> &str {
        self.id.split_once(':').map_or(self.id, |(_, p)| p)
    }
    pub fn namespace(&self) -> &str {
        self.id.split_once(':').map_or("minecraft", |(n, _)| n)
    }
    fn is(&self, item: &str) -> bool {
        self.id == item
    }
    fn tagged(&self, tag: &str) -> bool {
        (self.tag)(tag)
    }
    fn path_contains(&self, parts: &[&str]) -> bool {
        let path = self.path();
        parts.iter().any(|p| path.contains(p))
    }

    /// `ItemCategories.hotbarPriority`'s group.
    pub fn hotbar_group(&self) -> HotbarGroup {
        if self.is_weapon() {
            HotbarGroup::Weapons
        } else if self.is_tool() {
            HotbarGroup::Tools
        } else if self.food {
            HotbarGroup::Food
        } else if self.is_quick_build_block() {
            HotbarGroup::BuildingBlocks
        } else if self.is_utility_item() {
            HotbarGroup::UtilityItems
        } else {
            HotbarGroup::Other
        }
    }

    /// `ItemCategories.inventoryPriority`.
    pub fn inventory_priority(&self) -> i32 {
        // Ores first: deepslate and redstone ores would otherwise match the stone names
        if self.is_ore() {
            400
        } else if self.is_stone() {
            100
        } else if self.is_earth() {
            200
        } else if self.is_wood() {
            300
        } else if self.block {
            500
        } else if self.is_tool() {
            600
        } else if self.is_weapon() {
            700
        } else if self.is_armor() {
            800
        } else if self.food {
            900
        } else if self.is_redstone() {
            1000
        } else if self.is_transportation() {
            1100
        } else if self.is_decoration() {
            1200
        } else {
            9999
        }
    }

    fn is_weapon(&self) -> bool {
        self.tagged("minecraft:swords")
            || self.tagged("minecraft:spears")
            || self.tagged("minecraft:arrows")
            || ["minecraft:bow", "minecraft:crossbow", "minecraft:trident", "minecraft:mace"]
                .iter()
                .any(|i| self.is(i))
    }

    fn is_tool(&self) -> bool {
        if ["minecraft:pickaxes", "minecraft:axes", "minecraft:shovels", "minecraft:hoes"]
            .iter()
            .any(|t| self.tagged(t))
        {
            return true;
        }
        // Swords carry a tool component too (for cobwebs), so exclude weapons here
        (self.tool && !self.is_weapon())
            || [
                "minecraft:fishing_rod",
                "minecraft:flint_and_steel",
                "minecraft:brush",
                "minecraft:compass",
                "minecraft:clock",
            ]
            .iter()
            .any(|i| self.is(i))
    }

    fn is_armor(&self) -> bool {
        [
            "minecraft:head_armor",
            "minecraft:chest_armor",
            "minecraft:leg_armor",
            "minecraft:foot_armor",
        ]
        .iter()
        .any(|t| self.tagged(t))
            || self.blocks_attacks
            || self.glider
    }

    fn is_quick_build_block(&self) -> bool {
        self.block
            && (["minecraft:planks", "minecraft:dirt", "minecraft:sand", "minecraft:wool"]
                .iter()
                .any(|t| self.tagged(t))
                || (self.path_contains(&["stone", "gravel"]) && !self.is_stone_lookalike()))
    }

    // Flint and steel and compasses count as tools, which are checked first
    fn is_utility_item(&self) -> bool {
        self.is("minecraft:ender_pearl") || self.path_contains(&["bucket", "torch", "map"])
    }

    fn is_stone(&self) -> bool {
        self.block
            && (self.tagged("minecraft:stone_crafting_materials")
                || self.tagged("minecraft:stone_bricks")
                || (self.path_contains(&[
                    "stone", "granite", "diorite", "andesite", "basalt", "tuff", "deepslate",
                ]) && !self.is_stone_lookalike()))
    }

    /// Names containing "stone" that aren't stone.
    fn is_stone_lookalike(&self) -> bool {
        self.path_contains(&["redstone", "glowstone", "lodestone"])
    }

    fn is_earth(&self) -> bool {
        self.block
            && (["minecraft:dirt", "minecraft:sand", "minecraft:terracotta"]
                .iter()
                .any(|t| self.tagged(t))
                || self.path_contains(&[
                    "grass",
                    "gravel",
                    "clay",
                    "concrete",
                    "netherrack",
                    "soul_sand",
                    "soul_soil",
                ]))
    }

    fn is_wood(&self) -> bool {
        ["minecraft:logs", "minecraft:planks", "minecraft:leaves", "minecraft:saplings"]
            .iter()
            .any(|t| self.tagged(t))
            || (self.block && self.path_contains(&["_wood", "_hyphae"]))
    }

    fn is_ore(&self) -> bool {
        if [
            "minecraft:coal_ores",
            "minecraft:iron_ores",
            "minecraft:gold_ores",
            "minecraft:diamond_ores",
        ]
        .iter()
        .any(|t| self.tagged(t))
            || self.path_contains(&["_ore"])
        {
            return true;
        }
        self.block
            && self.path_contains(&["_block"])
            && self.path_contains(&[
                "iron", "gold", "diamond", "emerald", "coal", "copper", "redstone", "lapis",
                "netherite",
            ])
    }

    fn is_redstone(&self) -> bool {
        self.tagged("minecraft:rails")
            || self.path_contains(&[
                "redstone",
                "repeater",
                "comparator",
                "piston",
                "dispenser",
                "dropper",
                "hopper",
                "pressure_plate",
                "lever",
                "observer",
                "button",
                "door",
            ])
    }

    fn is_transportation(&self) -> bool {
        self.tagged("minecraft:boats")
            || self.tagged("minecraft:chest_boats")
            || self.path_contains(&["minecart", "saddle"])
    }

    fn is_decoration(&self) -> bool {
        [
            "minecraft:wool_carpets",
            "minecraft:banners",
            "minecraft:candles",
            "minecraft:skulls",
        ]
        .iter()
        .any(|t| self.tagged(t))
            || self.path_contains(&[
                "painting",
                "item_frame",
                "flower_pot",
                "flower",
                "tulip",
                "torch",
                "lantern",
                "head",
            ])
    }
}

/// One kind of stack in a sort: stacks that merge (the same item and the same
/// components) share a kind.
#[derive(Clone, Debug)]
pub struct Kind {
    pub path: String,
    pub namespace: String,
    pub max: u32,
    pub inventory_priority: i32,
    pub hotbar_group: HotbarGroup,
}

impl Kind {
    pub fn new(facts: &Facts<'_>, max: u32) -> Kind {
        Kind {
            path: facts.path().to_owned(),
            namespace: facts.namespace().to_owned(),
            max: max.max(1),
            inventory_priority: facts.inventory_priority(),
            hotbar_group: facts.hotbar_group(),
        }
    }
}

/// A slot: its kind (an index into the sort's kinds) and count, or empty.
pub type Slot = Option<(usize, u32)>;

/// `InventorySorter.sort`: where the stacks in `slots` go, as many slots
/// long (empty ones last). None when the stacks wouldn't fit back (stacks over
/// their maximum).
pub fn arrange(
    slots: &[Slot],
    kinds: &[Kind],
    mode: SortMode,
    region: Region,
    settings: &Settings,
) -> Option<Vec<Slot>> {
    // Groups in the order their kind first appears; the sorts below are stable
    let mut groups: Vec<(usize, Vec<u32>)> = Vec::new();
    for &(kind, count) in slots.iter().flatten() {
        match groups.iter_mut().find(|(k, _)| *k == kind) {
            Some((_, counts)) => counts.push(count),
            None => groups.push((kind, vec![count])),
        }
    }
    // By item name; the mod id only breaks ties
    let by_name = |a: &(usize, Vec<u32>), b: &(usize, Vec<u32>)| {
        let (a, b) = (&kinds[a.0], &kinds[b.0]);
        a.path.cmp(&b.path).then_with(|| a.namespace.cmp(&b.namespace))
    };
    let sorted: Vec<Slot> = match mode {
        SortMode::StackBased => {
            groups.sort_by(by_name);
            groups
                .into_iter()
                .flat_map(|(kind, mut counts)| {
                    counts.sort_by(|a, b| b.cmp(a));
                    counts.into_iter().map(move |c| Some((kind, c)))
                })
                .collect()
        }
        SortMode::Alphabetical => {
            groups.sort_by(by_name);
            stack(&groups, kinds, settings.allow_partial(region))
        }
        SortMode::Default => {
            let priority = |kind: usize| match region {
                Region::Hotbar => settings.hotbar_priority(kinds[kind].hotbar_group),
                _ => kinds[kind].inventory_priority,
            };
            groups.sort_by(|a, b| priority(a.0).cmp(&priority(b.0)).then_with(|| by_name(a, b)));
            stack(&groups, kinds, settings.allow_partial(region))
        }
    };
    if sorted.len() > slots.len() {
        return None;
    }
    let mut target = sorted;
    target.resize(slots.len(), None);
    Some(target)
}

/// Merges each group into full stacks. Leftover partial stacks follow their
/// group, or, when partial stacks aren't allowed, go after every full stack.
fn stack(groups: &[(usize, Vec<u32>)], kinds: &[Kind], allow_partial: bool) -> Vec<Slot> {
    let mut result = Vec::new();
    let mut leftovers = Vec::new();
    for (kind, counts) in groups {
        let max = kinds[*kind].max;
        let mut total: u32 = counts.iter().sum();
        while total >= max {
            result.push(Some((*kind, max)));
            total -= max;
        }
        if total > 0 {
            if allow_partial { &mut result } else { &mut leftovers }.push(Some((*kind, total)));
        }
    }
    result.extend(leftovers);
    result
}

/// A hand and slots under vanilla's left `PICKUP` click.
#[derive(Clone)]
struct Sim<'a> {
    slots: Vec<Slot>,
    hand: Slot,
    kinds: &'a [Kind],
    clicks: Vec<usize>,
}

impl Sim<'_> {
    fn click(&mut self, i: usize) {
        self.clicks.push(i);
        let slot = self.slots[i].take();
        match (self.hand.take(), slot) {
            // Take the stack, or put the hand's down.
            (None, s) => self.hand = s,
            (Some(h), None) => self.slots[i] = Some(h),
            // The same kind: as much as fits joins the slot.
            (Some((k, h)), Some((sk, s))) if k == sk => {
                let n = self.kinds[k].max.saturating_sub(s).min(h);
                self.slots[i] = Some((k, s + n));
                self.hand = (h > n).then_some((k, h - n));
            }
            // Another kind: they swap.
            (Some(h), Some(s)) => {
                self.slots[i] = Some(h);
                self.hand = Some(s);
            }
        }
    }
    /// Brings `from`'s stack to `to`; what `to` held goes back to `from`.
    fn bring(&mut self, from: usize, to: usize) {
        self.click(from);
        self.click(to);
        if self.hand.is_some() {
            self.click(from);
        }
    }
    fn partial(&self, i: usize) -> Option<(usize, u32)> {
        self.slots[i].filter(|&(k, c)| c < self.kinds[k].max)
    }
}

/// The indices of `slots` to left-click, in order, to turn them into
/// `target`, starting and ending with an empty hand. Merges partial stacks
/// first when `merge` (the stacking modes). Empty when already there; None
/// when the clicks can't reach it.
pub fn plan(slots: &[Slot], target: &[Slot], kinds: &[Kind], merge: bool) -> Option<Vec<usize>> {
    let n = slots.len();
    if target.len() != n {
        return None;
    }
    let mut sim = Sim {
        slots: slots.to_vec(),
        hand: None,
        kinds,
        clicks: Vec::new(),
    };
    if merge {
        // Each later partial stack of a kind tops up the first one; what
        // doesn't fit goes back.
        for i in 0..n {
            for j in i + 1..n {
                let Some((kind, _)) = sim.partial(i) else {
                    break;
                };
                if sim.partial(j).is_some_and(|(k, _)| k == kind) {
                    sim.bring(j, i);
                }
            }
        }
    }
    for i in 0..n {
        if sim.slots[i] == target[i] {
            continue;
        }
        let j = (i + 1..n).find(|&j| sim.slots[j] == target[i])?;
        let mut tried = sim.clone();
        tried.bring(j, i);
        if tried.slots[i] == target[i] && tried.hand.is_none() {
            sim = tried;
            continue;
        }
        // The same kind with another count is here: the hand would merge
        // them, not swap. Move it out of the way first.
        let here = sim.slots[i].map(|(k, _)| k);
        let spare = (i + 1..n)
            .filter(|&e| e != j)
            .find(|&e| sim.slots[e].is_none())
            .or_else(|| {
                (i + 1..n)
                    .filter(|&k| k != j)
                    .find(|&k| sim.slots[k].is_some_and(|(kind, _)| Some(kind) != here))
            })?;
        sim.click(i);
        sim.click(spare);
        if sim.hand.is_some() {
            sim.click(i);
        }
        sim.bring(j, i);
        if sim.slots[i] != target[i] || sim.hand.is_some() {
            return None;
        }
    }
    (sim.hand.is_none() && sim.slots == target).then_some(sim.clicks)
}

/// The settings (one file per key in the mod's folder), with the Java mod's
/// defaults.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub mode: SortMode,
    pub buttons: bool,
    pub messages: bool,
    pub sound: bool,
    pub partial_chest: bool,
    pub partial_inventory: bool,
    pub chests: bool,
    pub shulker_boxes: bool,
    pub barrels: bool,
    pub ender_chests: bool,
    pub inventory: bool,
    pub hotbar: bool,
    /// `HotbarPriorities`, in `HotbarGroup::ALL`'s order. Not on the
    /// settings page (as in the Java mod): edit the files.
    pub priorities: [i32; 6],
    /// Player inventory slots (0-8 hotbar, 9-35 main) that sorting never moves.
    pub locked: BTreeSet<usize>,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            mode: SortMode::Default,
            buttons: true,
            messages: true,
            sound: true,
            partial_chest: true,
            partial_inventory: true,
            chests: true,
            shulker_boxes: true,
            barrels: true,
            ender_chests: true,
            inventory: true,
            hotbar: true,
            priorities: HotbarGroup::ALL.map(HotbarGroup::default_priority),
            locked: BTreeSet::new(),
        }
    }
}

/// The settings page's toggles: their stored key and label.
const TOGGLES: [(&str, &str); 11] = [
    ("buttons", "Sort Buttons"),
    ("messages", "Messages"),
    ("sound", "Sound"),
    ("partial_chest", "Partial Stacks in Chests"),
    ("partial_inventory", "Partial Stacks in Inventory"),
    ("chests", "Chest Sorting"),
    ("shulker_boxes", "Shulker Box Sorting"),
    ("barrels", "Barrel Sorting"),
    ("ender_chests", "Ender Chest Sorting"),
    ("inventory", "Inventory Sorting"),
    ("hotbar", "Hotbar Sorting"),
];

fn on_off(on: bool) -> &'static str {
    if on { "ON" } else { "OFF" }
}

impl Settings {
    /// Reads them through `get` (a key's stored value).
    pub fn read(get: impl Fn(&str) -> Option<String>) -> Settings {
        let mut s = Settings::default();
        if let Some(mode) = get("mode").as_deref().and_then(SortMode::parse) {
            s.mode = mode;
        }
        for (i, (key, _)) in TOGGLES.iter().enumerate() {
            if let Some(v) = get(key) {
                *s.toggle_mut(i) = v.trim() != "false";
            }
        }
        for (i, group) in HotbarGroup::ALL.into_iter().enumerate() {
            if let Some(p) = get(group.setting()).and_then(|v| v.trim().parse().ok()) {
                s.priorities[i] = p;
            }
        }
        if let Some(v) = get("locked_slots") {
            s.locked = v
                .split(',')
                .filter_map(|n| n.trim().parse().ok())
                .filter(|n| (0..36).contains(n))
                .collect();
        }
        s
    }

    fn toggle_mut(&mut self, i: usize) -> &mut bool {
        match i {
            0 => &mut self.buttons,
            1 => &mut self.messages,
            2 => &mut self.sound,
            3 => &mut self.partial_chest,
            4 => &mut self.partial_inventory,
            5 => &mut self.chests,
            6 => &mut self.shulker_boxes,
            7 => &mut self.barrels,
            8 => &mut self.ender_chests,
            9 => &mut self.inventory,
            _ => &mut self.hotbar,
        }
    }

    /// What to store, by key.
    pub fn stored(&self) -> Vec<(&'static str, String)> {
        let mut out = vec![("mode", self.mode.key().to_owned())];
        let mut copy = self.clone();
        for (i, (key, _)) in TOGGLES.iter().enumerate() {
            out.push((key, copy.toggle_mut(i).to_string()));
        }
        for (i, group) in HotbarGroup::ALL.into_iter().enumerate() {
            out.push((group.setting(), self.priorities[i].to_string()));
        }
        let locked: Vec<String> = self.locked.iter().map(usize::to_string).collect();
        out.push(("locked_slots", locked.join(",")));
        out
    }

    /// The settings page's buttons: the mode, the toggles, then clearing the locks.
    pub fn labels(&self) -> Vec<String> {
        let mut copy = self.clone();
        let mut labels = vec![format!("Sort Mode: {}", self.mode.name())];
        for (i, (_, label)) in TOGGLES.iter().enumerate() {
            labels.push(format!("{label}: {}", on_off(*copy.toggle_mut(i))));
        }
        labels.push(format!("Unlock All Slots ({} locked)", self.locked.len()));
        labels
    }

    /// A click on button `button` of the page.
    pub fn click(&mut self, button: usize) {
        match button {
            0 => self.mode = self.mode.next(),
            b if b <= TOGGLES.len() => {
                let t = self.toggle_mut(b - 1);
                *t = !*t;
            }
            b if b == TOGGLES.len() + 1 => self.locked.clear(),
            _ => {}
        }
    }

    pub fn allow_partial(&self, region: Region) -> bool {
        match region {
            Region::Container => self.partial_chest,
            Region::Inventory => self.partial_inventory,
            Region::Hotbar => true,
        }
    }

    pub fn hotbar_priority(&self, group: HotbarGroup) -> i32 {
        let i = HotbarGroup::ALL.iter().position(|&g| g == group).unwrap_or(5);
        self.priorities[i]
    }

    pub fn container_enabled(&self, kind: ContainerKind) -> bool {
        match kind {
            ContainerKind::Chest => self.chests,
            ContainerKind::ShulkerBox => self.shulker_boxes,
            ContainerKind::Barrel => self.barrels,
            ContainerKind::EnderChest => self.ender_chests,
        }
    }

    /// Locks or unlocks a player slot: true when it's now locked.
    pub fn toggle_lock(&mut self, slot: usize) -> bool {
        if self.locked.remove(&slot) {
            false
        } else {
            self.locked.insert(slot);
            true
        }
    }

    /// The region's slots that sorting may move.
    pub fn unlocked(&self, range: std::ops::Range<usize>) -> Vec<usize> {
        range.filter(|s| !self.locked.contains(s)).collect()
    }
}

thread_local! {
    /// The settings, read once from their files, then kept.
    static SETTINGS: RefCell<Option<Settings>> = const { RefCell::new(None) };
    /// The player's own value of Horizon's sort buttons extra, while the
    /// mod's buttons replace it.
    static OWN_SORT_BUTTONS: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

/// The settings, read once through `get`.
pub fn settings(get: impl Fn(&str) -> Option<String>) -> Settings {
    SETTINGS.with(|s| s.borrow_mut().get_or_insert_with(|| Settings::read(get)).clone())
}

/// Replaces them (the settings page, a command or a lock changed them).
pub fn set_settings(settings: Settings) {
    SETTINGS.with(|s| *s.borrow_mut() = Some(settings));
}

/// Remembers the player's own sort buttons value the first time the mod
/// takes them over; gives it back (once) when `restore`.
pub fn own_sort_buttons(current: bool, restore: bool) -> Option<bool> {
    OWN_SORT_BUTTONS.with(|own| {
        if restore {
            own.take()
        } else {
            if own.get().is_none() {
                own.set(Some(current));
            }
            None
        }
    })
}

/// What a sort or a toggle says, and its sound's pitch offset.
#[derive(Clone, Debug, PartialEq)]
pub struct Feedback {
    pub text: String,
    pub pitch: f32,
}

impl Feedback {
    fn new(text: impl Into<String>, pitch: f32) -> Feedback {
        Feedback {
            text: text.into(),
            pitch,
        }
    }
    pub fn container_sorted(kind: ContainerKind, mode: SortMode) -> Feedback {
        Feedback::new(format!("✨ {} magically sorted! ({})", kind.name(), mode.name()), 0.0)
    }
    /// After a player sort: which parts were sorted.
    pub fn player_sorted(hotbar: bool, inventory: bool, mode: SortMode) -> Feedback {
        match (hotbar, inventory) {
            (true, true) => Feedback::new(
                format!("🧙✨ Complete inventory enchanted by the InventoryWizard! ({})", mode.name()),
                -0.2,
            ),
            (true, false) => {
                Feedback::new(format!("✨ Hotbar arranged by wizardry! ({})", mode.name()), 0.2)
            }
            _ => Feedback::new(
                format!("✨ Inventory organized with wizard magic! ({})", mode.name()),
                0.0,
            ),
        }
    }
    pub fn mode_changed(mode: SortMode) -> Feedback {
        Feedback::new(format!("🔄 Sorting mode changed to: {}", mode.name()), 0.4)
    }
    pub fn disabled() -> Feedback {
        Feedback::new("That kind of sorting is turned off in the InventoryWizard settings.", 0.0)
    }
    pub fn unsupported_container() -> Feedback {
        Feedback::new("InventoryWizard can't sort this container.", 0.0)
    }
    pub fn could_not_sort() -> Feedback {
        Feedback::new("InventoryWizard couldn't sort that with clicks; nothing was moved.", 0.0)
    }
    pub fn lock(locked: bool) -> Feedback {
        if locked {
            Feedback::new("🔒 Slot locked: sorting will leave it alone", 0.4)
        } else {
            Feedback::new("🔓 Slot unlocked", 0.2)
        }
    }
}

/// What `/iwiz` was asked to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Hotbar,
    Inventory,
    All,
    Mode,
}

/// `/iwiz [hotbar|inventory|all|mode]` (or `/inventorywizard`), without its
/// slash; None for other commands. `Err` holds the usage line.
pub fn parse_command(command: &str) -> Option<Result<Command, &'static str>> {
    let mut words = command.split_whitespace();
    if !matches!(words.next(), Some("iwiz" | "inventorywizard")) {
        return None;
    }
    let parsed = match (words.next(), words.next()) {
        (None, _) | (Some("inventory"), None) => Ok(Command::Inventory),
        (Some("hotbar"), None) => Ok(Command::Hotbar),
        (Some("all"), None) => Ok(Command::All),
        (Some("mode"), None) => Ok(Command::Mode),
        _ => Err("Usage: /iwiz [hotbar|inventory|all|mode]"),
    };
    Some(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_tags(_: &str) -> bool {
        false
    }
    fn facts<'a>(id: &'a str, tag: &'a dyn Fn(&str) -> bool) -> Facts<'a> {
        Facts {
            id,
            block: false,
            food: false,
            tool: false,
            glider: false,
            blocks_attacks: false,
            tag,
        }
    }
    fn kind(path: &str, max: u32, priority: i32) -> Kind {
        Kind {
            path: path.into(),
            namespace: "minecraft".into(),
            max,
            inventory_priority: priority,
            hotbar_group: HotbarGroup::Other,
        }
    }
    /// Plays `clicks` on `slots`, checking that every one starts and ends
    /// with an empty hand, as `plan` promises.
    fn replay(slots: &[Slot], kinds: &[Kind], clicks: &[usize]) -> Vec<Slot> {
        let mut sim = Sim {
            slots: slots.to_vec(),
            hand: None,
            kinds,
            clicks: Vec::new(),
        };
        for &c in clicks {
            sim.click(c);
        }
        assert!(sim.hand.is_none());
        sim.slots
    }

    #[test]
    fn categories_follow_the_java_mod() {
        let swords = |t: &str| t == "minecraft:swords";
        let mut sword = facts("minecraft:diamond_sword", &swords);
        sword.tool = true;
        assert_eq!(sword.hotbar_group(), HotbarGroup::Weapons);
        assert_eq!(sword.inventory_priority(), 700);

        let mut redstone_ore = facts("minecraft:deepslate_redstone_ore", &no_tags);
        redstone_ore.block = true;
        assert_eq!(redstone_ore.inventory_priority(), 400);

        let mut redstone_block = facts("minecraft:redstone_block", &no_tags);
        redstone_block.block = true;
        assert_eq!(redstone_block.inventory_priority(), 400);

        let mut cobble = facts("minecraft:cobblestone", &no_tags);
        cobble.block = true;
        assert_eq!(cobble.inventory_priority(), 100);
        assert_eq!(cobble.hotbar_group(), HotbarGroup::BuildingBlocks);

        let mut glowstone = facts("minecraft:glowstone", &no_tags);
        glowstone.block = true;
        assert_eq!(glowstone.inventory_priority(), 500);

        let mut bread = facts("minecraft:bread", &no_tags);
        bread.food = true;
        assert_eq!(bread.hotbar_group(), HotbarGroup::Food);
        assert_eq!(bread.inventory_priority(), 900);

        let mut elytra = facts("minecraft:elytra", &no_tags);
        elytra.glider = true;
        assert_eq!(elytra.inventory_priority(), 800);

        assert_eq!(facts("minecraft:water_bucket", &no_tags).hotbar_group(), HotbarGroup::UtilityItems);
        assert_eq!(facts("minecraft:compass", &no_tags).hotbar_group(), HotbarGroup::Tools);
        assert_eq!(facts("minecraft:oak_boat", &no_tags).inventory_priority(), 9999);
        let boats = |t: &str| t == "minecraft:boats";
        assert_eq!(facts("minecraft:oak_boat", &boats).inventory_priority(), 1100);
        assert_eq!(facts("modded:thing", &no_tags).namespace(), "modded");
    }

    #[test]
    fn default_mode_merges_and_orders_by_category_then_name() {
        // 0 dirt (200), 1 cobblestone (100), 2 apple (900)
        let kinds = [kind("dirt", 64, 200), kind("cobblestone", 64, 100), kind("apple", 64, 900)];
        let slots = [Some((2, 5)), Some((0, 40)), None, Some((1, 10)), Some((0, 40))];
        let s = Settings::default();
        let target = arrange(&slots, &kinds, SortMode::Default, Region::Inventory, &s).unwrap();
        assert_eq!(
            target,
            [Some((1, 10)), Some((0, 64)), Some((0, 16)), Some((2, 5)), None]
        );
        let clicks = plan(&slots, &target, &kinds, true).unwrap();
        assert_eq!(replay(&slots, &kinds, &clicks), target);
    }

    #[test]
    fn partial_stacks_go_last_when_not_allowed() {
        let kinds = [kind("a", 64, 1), kind("b", 64, 2)];
        let slots = [Some((1, 70)), Some((0, 3)), None, None];
        let s = Settings {
            partial_inventory: false,
            ..Settings::default()
        };
        // 70 is over b's maximum: arrange splits it, the clicks can't
        let target = arrange(&slots, &kinds, SortMode::Default, Region::Inventory, &s).unwrap();
        assert_eq!(target, [Some((1, 64)), Some((0, 3)), Some((1, 6)), None]);
        assert_eq!(plan(&slots, &target, &kinds, true), None);

        let slots = [Some((1, 64)), Some((0, 3)), Some((1, 6)), Some((0, 64))];
        let target = arrange(&slots, &kinds, SortMode::Default, Region::Inventory, &s).unwrap();
        assert_eq!(target, [Some((0, 64)), Some((1, 64)), Some((0, 3)), Some((1, 6))]);
        let clicks = plan(&slots, &target, &kinds, true).unwrap();
        assert_eq!(replay(&slots, &kinds, &clicks), target);
    }

    #[test]
    fn hotbar_uses_its_priorities() {
        let mut kinds = [kind("bread", 64, 900), kind("sword", 1, 700)];
        kinds[0].hotbar_group = HotbarGroup::Food;
        kinds[1].hotbar_group = HotbarGroup::Weapons;
        let slots = [Some((0, 5)), Some((1, 1))];
        let mut s = Settings::default();
        let target = arrange(&slots, &kinds, SortMode::Default, Region::Hotbar, &s).unwrap();
        assert_eq!(target, [Some((1, 1)), Some((0, 5))]);
        s.priorities[2] = 1;
        let target = arrange(&slots, &kinds, SortMode::Default, Region::Hotbar, &s).unwrap();
        assert_eq!(target, [Some((0, 5)), Some((1, 1))]);
    }

    #[test]
    fn stack_based_keeps_counts_largest_first() {
        let kinds = [kind("b", 64, 0), kind("a", 64, 0)];
        let slots = [Some((0, 3)), Some((1, 10)), Some((1, 30)), Some((0, 64)), None];
        let s = Settings::default();
        let target = arrange(&slots, &kinds, SortMode::StackBased, Region::Container, &s).unwrap();
        assert_eq!(
            target,
            [Some((1, 30)), Some((1, 10)), Some((0, 64)), Some((0, 3)), None]
        );
        let clicks = plan(&slots, &target, &kinds, false).unwrap();
        assert_eq!(replay(&slots, &kinds, &clicks), target);
    }

    #[test]
    fn same_kind_swaps_go_through_a_spare_slot() {
        let kinds = [kind("a", 64, 0), kind("b", 64, 0)];
        // No empty slot: the other kind is the spare
        let slots = [Some((0, 10)), Some((0, 30)), Some((1, 1))];
        let target = [Some((0, 30)), Some((0, 10)), Some((1, 1))];
        let clicks = plan(&slots, &target, &kinds, false).unwrap();
        assert_eq!(replay(&slots, &kinds, &clicks), target);
        // Nothing to swap through: the clicks can't do it
        let slots = [Some((0, 10)), Some((0, 30))];
        let target = [Some((0, 30)), Some((0, 10))];
        assert_eq!(plan(&slots, &target, &kinds, false), None);
    }

    #[test]
    fn random_slots_always_reach_their_order() {
        let kinds = [kind("a", 64, 3), kind("b", 16, 1), kind("c", 1, 2), kind("d", 64, 1)];
        let mut seed: u64 = 0x1f2e_3d4c;
        let mut next = |n: u64| {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (seed >> 33) % n
        };
        let s = Settings {
            partial_inventory: false,
            ..Settings::default()
        };
        for round in 0..2000 {
            let len = 1 + next(27) as usize;
            let slots: Vec<Slot> = (0..len)
                .map(|_| {
                    let k = next(5) as usize;
                    (k < kinds.len()).then(|| (k, 1 + next(u64::from(kinds[k].max)) as u32))
                })
                .collect();
            for mode in SortMode::ALL {
                let region = if round % 2 == 0 { Region::Inventory } else { Region::Container };
                let target = arrange(&slots, &kinds, mode, region, &s).unwrap();
                let merge = mode != SortMode::StackBased;
                match plan(&slots, &target, &kinds, merge) {
                    Some(clicks) => assert_eq!(replay(&slots, &kinds, &clicks), target),
                    None => assert!(!merge, "{mode:?} couldn't sort {slots:?}"),
                }
            }
        }
    }

    #[test]
    fn sorted_slots_need_no_clicks() {
        let kinds = [kind("a", 64, 0)];
        let slots = [Some((0, 64)), Some((0, 2)), None];
        let s = Settings::default();
        let target = arrange(&slots, &kinds, SortMode::Alphabetical, Region::Inventory, &s).unwrap();
        assert_eq!(plan(&slots, &target, &kinds, true), Some(Vec::new()));
    }

    #[test]
    fn settings_round_trip_and_page() {
        let mut s = Settings::default();
        s.click(0);
        s.click(2);
        assert!(s.toggle_lock(4));
        assert!(s.toggle_lock(12));
        assert!(!s.toggle_lock(4));
        let stored = s.stored();
        let read = Settings::read(|k| stored.iter().find(|(key, _)| *key == k).map(|(_, v)| v.clone()));
        assert_eq!(read, s);
        assert_eq!(read.mode, SortMode::Alphabetical);
        assert!(!read.messages);
        assert_eq!(read.unlocked(9..14), vec![9, 10, 11, 13]);
        let labels = s.labels();
        assert_eq!(labels[0], "Sort Mode: Alphabetical");
        assert_eq!(labels[2], "Messages: OFF");
        assert_eq!(labels.last().unwrap(), "Unlock All Slots (1 locked)");
        s.click(labels.len() - 1);
        assert!(s.locked.is_empty());
    }

    #[test]
    fn modes_cycle_and_commands_parse() {
        assert_eq!(SortMode::StackBased.next(), SortMode::Default);
        assert_eq!(parse_command("iwiz"), Some(Ok(Command::Inventory)));
        assert_eq!(parse_command("inventorywizard all"), Some(Ok(Command::All)));
        assert_eq!(parse_command("iwiz mode"), Some(Ok(Command::Mode)));
        assert!(matches!(parse_command("iwiz nope"), Some(Err(_))));
        assert_eq!(parse_command("where"), None);
        assert_eq!(ContainerKind::of_block("minecraft:trapped_chest"), Some(ContainerKind::Chest));
        assert_eq!(ContainerKind::of_block("minecraft:red_shulker_box"), Some(ContainerKind::ShulkerBox));
        assert_eq!(ContainerKind::of_block("minecraft:hopper"), None);
    }
}
