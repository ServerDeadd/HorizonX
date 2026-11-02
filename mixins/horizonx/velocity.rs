// Velocity: o empurrão de golpes e explosões chega multiplicado pelo ajuste.
fn horizonx_velocity(game: &mut crate::game::Game<'_>, packet: &C) -> core::ops::ControlFlow<Result<()>> {
    if crate::game::horizonx_glue::velocity_packet(game, packet) {
        core::ops::ControlFlow::Break(Ok(()))
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
