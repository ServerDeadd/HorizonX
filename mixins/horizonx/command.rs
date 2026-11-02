// /m continua valendo (o prefixo principal agora é o ponto).
fn horizonx_command(game: &mut crate::game::Game<'_>, command: &str, ran: &mut bool) {
    crate::game::horizonx_glue::slash_command(game, command, ran);
}
