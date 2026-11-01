// Hook helper: tweaks::Game::sort_slots (cancellable HEAD). Horizon's sort buttons sort the mod's way.
fn inventory_wizard_sort_slots(
    game: &mut crate::game::Game<'_>,
    slots: &[usize],
) -> core::ops::ControlFlow<()> {
    crate::game::mod_inventory_wizard_glue::on_sort_slots(game, slots)
}
