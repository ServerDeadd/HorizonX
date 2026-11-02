// A lista de módulos e as informações na tela.
fn horizonx_hud(
    game: &mut crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    corners: &mut horizon_client::extras_hud::Corners,
) {
    crate::game::horizonx_glue::hud(game, batch, corners);
}
