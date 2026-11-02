// As caixas do ESP e as linhas dos Tracers entram nas linhas desenhadas por cima de tudo.
fn horizonx_gizmos(game: &crate::game::Game<'_>, camera: [f64; 3], g: &mut horizon_client::gizmos::Gizmos) {
    crate::game::horizonx_glue::gizmos(game, camera, g);
}
