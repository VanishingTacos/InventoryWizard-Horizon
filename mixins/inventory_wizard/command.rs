// Hook helper: native_command. /iwiz [hotbar|inventory|all|mode].
fn inventory_wizard_command(game: &mut crate::game::Game<'_>, command: &str, ran: &mut bool) {
    crate::game::mod_inventory_wizard_glue::on_command(game, command, ran)
}
