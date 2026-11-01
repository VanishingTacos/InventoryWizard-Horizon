// Hook helper: native_frame. Keeps Horizon's sort buttons in step with the mod's setting.
fn inventory_wizard_frame(game: &mut crate::game::Game<'_>) {
    crate::game::mod_inventory_wizard_glue::on_frame(game)
}
