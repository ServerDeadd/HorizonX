// NoFall: o servidor recebe "no chão" quando a queda já daria dano.
fn horizonx_nofall(value: bool) -> bool {
    crate::game::horizonx::nofall_spoof(value)
}
