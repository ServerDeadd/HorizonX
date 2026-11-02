// Cada tick do jogo: os módulos de movimento e de combate.
fn horizonx_tick(game: &mut crate::game::Game<'_>) {
    crate::game::horizonx_glue::tick(game);
}
