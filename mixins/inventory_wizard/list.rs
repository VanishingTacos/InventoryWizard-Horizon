// Hook helper: native_mods. The mod's row in Options > Mods.
fn inventory_wizard_list(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    crate::game::mod_inventory_wizard_glue::on_list(mods)
}
