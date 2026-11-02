// Cada quadro: Fullbright, Xray e o movimento do Freecam.
fn horizonx_frame(game: &mut crate::game::Game<'_>, elapsed: std::time::Duration) {
    crate::game::horizonx_glue::frame(game, elapsed);
}
