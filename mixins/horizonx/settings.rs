// A página do Meteor em Opções > Mods: categorias, módulos e os ajustes de cada um.
fn horizonx_page(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    use crate::game::horizonx as m;
    if mod_id != m::ID {
        return;
    }
    let game = &*game;
    m::load(|key| game.native_setting(m::ID, key));
    buttons.extend(m::labels());
}
