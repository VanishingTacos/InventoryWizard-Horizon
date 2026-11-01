// Hook helper: tweaks::Game::sort_mouse_press (cancellable HEAD). Alt+Click locks, Shift+Right Click sorts.
fn inventory_wizard_mouse(
    game: &mut crate::game::Game<'_>,
    button: winit::event::MouseButton,
) -> core::ops::ControlFlow<bool> {
    crate::game::mod_inventory_wizard_glue::on_mouse(game, button)
}
