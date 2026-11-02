// A linha do Meteor em Opções > Mods, com a tecla do menu. As teclas dos módulos ficam no
// ClickGUI (linha "Tecla") e em .bind.
fn horizonx_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use crate::game::horizonx as m;
    mods.push(crate::game::native_hooks::NativeMod {
        id: m::ID.into(),
        title: format!("{} {}", m::NAME, m::VERSION),
        description: m::DESCRIPTION.into(),
        keys: vec![[m::MENU_KEY.into(), "Abrir o menu do HorizonX".into(), m::MENU_DEFAULT.into()]],
    });
}
