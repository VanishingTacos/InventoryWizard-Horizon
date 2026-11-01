// Hook helper: tweaks::Game::sort_press (cancellable HEAD). The mode button.
fn inventory_wizard_sort_press(game: &mut crate::game::Game<'_>) -> core::ops::ControlFlow<bool> {
    crate::game::mod_inventory_wizard_glue::on_sort_press(game)
}
