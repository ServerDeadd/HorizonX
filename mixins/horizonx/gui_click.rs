// Cliques no ClickGUI.
fn horizonx_gui_click(
    game: &crate::game::Game<'_>,
    screen: &crate::game::native_hooks::NativeScreenView<'_>,
    button: i32,
    canceled: &mut bool,
) {
    crate::game::horizonx_glue::gui_click(game, screen.id, button, canceled);
}
