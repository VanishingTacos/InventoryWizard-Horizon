// Hook helper: native_settings_click. A settings button was clicked.
fn inventory_wizard_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    crate::game::mod_inventory_wizard_glue::on_click(game, mod_id, button)
}
