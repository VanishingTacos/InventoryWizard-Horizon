// Hook helper: tweaks::Game::sort_key_press (cancellable HEAD). The sort and mode keys.
fn inventory_wizard_key(
    game: &mut crate::game::Game<'_>,
    key: winit::keyboard::KeyCode,
) -> core::ops::ControlFlow<bool> {
    crate::game::mod_inventory_wizard_glue::on_key(game, key)
}
