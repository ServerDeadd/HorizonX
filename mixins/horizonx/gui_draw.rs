// O ClickGUI por cima do mundo.
fn horizonx_gui_draw(
    game: &crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    screen: &crate::game::native_hooks::NativeScreenView<'_>,
) {
    crate::game::horizonx_glue::gui_draw(game, batch, screen.id);
}
