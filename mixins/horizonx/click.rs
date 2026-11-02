// Um clique na página do Meteor: navega, liga um módulo ou muda um ajuste, e guarda.
fn horizonx_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    use crate::game::horizonx as m;
    if mod_id != m::ID {
        return;
    }
    {
        let game = &*game;
        m::load(|key| game.native_setting(m::ID, key));
    }
    for (key, value) in m::click(button) {
        game.native_set_setting(m::ID, &key, &value);
    }
}
