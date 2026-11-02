// Comandos com ponto (.lista, .locate...): ficam no cliente e nunca vão para o servidor.
fn horizonx_chat(game: &mut crate::game::Game<'_>, message: &str) -> core::ops::ControlFlow<()> {
    if crate::game::horizonx_glue::chat(game, message) {
        core::ops::ControlFlow::Break(())
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
