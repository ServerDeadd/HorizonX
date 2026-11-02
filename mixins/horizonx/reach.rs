// Reach: o alcance de entidades e blocos vira o do ajuste.
fn horizonx_reach(game: &crate::game::Game<'_>) -> core::ops::ControlFlow<f64> {
    match crate::game::horizonx_glue::reach(game) {
        Some(range) => core::ops::ControlFlow::Break(range),
        None => core::ops::ControlFlow::Continue(()),
    }
}
