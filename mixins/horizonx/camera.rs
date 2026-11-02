// Freecam: com a câmera solta, a câmera do jogo vai para a posição livre em vez de seguir o
// personagem.
fn horizonx_camera(
    game: &mut crate::game::Game<'_>,
    _partial: f32,
) -> core::ops::ControlFlow<Option<(glam::DVec3, f32, f32)>> {
    match crate::game::horizonx_glue::apply_freecam_camera(game) {
        Some(v) => core::ops::ControlFlow::Break(Some(v)),
        None => core::ops::ControlFlow::Continue(()),
    }
}
