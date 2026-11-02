// NoRender: a neblina.
fn horizonx_fog(game: &crate::game::Game<'_>, fog: &mut horizon_client::fog::FogData) {
    crate::game::horizonx_glue::fog(game, fog);
}
