// NoRender: sem efeitos por cima da tela; a função do jogo nem roda.
fn horizonx_quads(game: &crate::game::Game<'_>) -> core::ops::ControlFlow<Vec<ScreenQuad>> {
    if crate::game::horizonx_glue::hide_overlays(game) {
        core::ops::ControlFlow::Break(Vec::new())
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
