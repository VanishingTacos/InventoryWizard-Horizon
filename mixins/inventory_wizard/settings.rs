// Hook helper: native_settings. The settings page's buttons.
fn inventory_wizard_settings(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    crate::game::mod_inventory_wizard_glue::on_settings(game, mod_id, buttons)
}
