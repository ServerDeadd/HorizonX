// Xray: a face de um minério encostada num bloco escondido aparece.
fn horizonx_xray_face(state: BlockStateId, neighbor: BlockStateId) -> core::ops::ControlFlow<io::Result<bool>> {
    let shared = &crate::horizonx_shared::keep;
    if crate::horizonx_shared::xray() && shared(state.0) && !shared(neighbor.0) {
        core::ops::ControlFlow::Break(Ok(true))
    } else {
        core::ops::ControlFlow::Continue(())
    }
}
