// Teclas na tela do ClickGUI: a tecla de um módulo esperando uma.
fn horizonx_gui_key(
    game: &crate::game::Game<'_>,
    screen: &crate::game::native_hooks::NativeScreenView<'_>,
    key: &str,
    canceled: &mut bool,
) {
    crate::game::horizonx_glue::gui_key(game, screen.id, key, canceled);
}
