// FastBreak: sem a espera de 5 ticks depois de quebrar um bloco.
fn horizonx_fastbreak(game: &mut crate::game::Game<'_>) {
    if crate::game::horizonx::active("fastbreak")
        && let Some(r) = game.remote.as_mut()
    {
        r.destroy_delay = 0;
    }
}
