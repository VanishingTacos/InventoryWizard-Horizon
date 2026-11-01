// Hook helper: native_commands. The chat box completes /iwiz and /inventorywizard.
fn inventory_wizard_commands(commands: &mut Vec<&'static str>) {
    crate::game::mod_inventory_wizard_glue::on_commands(commands)
}
