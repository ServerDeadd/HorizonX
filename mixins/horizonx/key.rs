// A tecla do menu e as teclas dos módulos.
fn horizonx_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    crate::game::horizonx_glue::key(game, key, pressed);
}
