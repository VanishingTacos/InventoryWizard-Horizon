// Hook helper: tweaks::Game::draw_sort_buttons. Locks on locked slots and the mode button.
fn inventory_wizard_draw(
    game: &crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    ox: f32,
    oy: f32,
) {
    crate::game::mod_inventory_wizard_glue::on_draw_buttons(game, batch, ox, oy)
}
