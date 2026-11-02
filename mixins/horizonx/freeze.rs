// Freecam: congela o personagem (solta as teclas de andar) enquanto a câmera voa.
fn horizonx_freeze(game: &mut crate::game::Game<'_>) {
    crate::game::horizonx_glue::freecam_freeze(game);
}
