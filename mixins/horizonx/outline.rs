// O contorno brilhante do ESP.
fn horizonx_outline(
    game: &crate::game::Game<'_>,
    mob: &horizon_client::mobs::Mob,
    value: Option<u32>,
) -> Option<u32> {
    crate::game::horizonx_glue::outline(game, mob, value)
}
