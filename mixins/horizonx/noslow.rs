// NoSlow: usar itens não deixa lento (agachar continua como o jogo faz).
fn horizonx_noslow(input: [f32; 2], item_use: Option<f32>, sneaking: Option<f32>) -> core::ops::ControlFlow<[f32; 2]> {
    if crate::horizonx_noslow::on() && item_use.is_some() {
        core::ops::ControlFlow::Break(modify_input(input, None, sneaking))
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
