// Freecam: o giro do mouse vai para a câmera solta e o personagem não vira.
fn horizonx_turn(game: &mut crate::game::Game<'_>, xo: f64, yo: f64) -> core::ops::ControlFlow<()> {
    if crate::game::horizonx_glue::freecam_turn(game, xo, yo) {
        core::ops::ControlFlow::Break(())
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
