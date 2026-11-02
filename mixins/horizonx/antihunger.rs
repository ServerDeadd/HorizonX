// AntiHunger: o servidor nunca recebe que você começou a correr.
fn horizonx_antihunger(value: bool) -> bool {
    value && !crate::game::horizonx::active("antihunger")
}
