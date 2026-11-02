// .nick: com o nick ligado o jogo se vê sem conta e entra com o nome escolhido.
fn horizonx_auth(value: Option<Arc<AuthSession>>) -> Option<Arc<AuthSession>> {
    if horizon_world::horizonx_shared::nick() { None } else { value }
}
