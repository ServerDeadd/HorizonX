// Staff Viewer: quem está conectado mas fora da tab, quem está no espectador e, quando o corpo
// está carregado perto, a distância e se está olhando para você. Fica dentro de game::remote
// porque a lista de jogadores do servidor é privada desse módulo.
use super::Game;
use horizon_client::tab_list::SPECTATOR;

pub(crate) struct StaffRow {
    pub(crate) name: String,
    /// Conectado, mas o servidor tirou da tab.
    pub(crate) hidden: bool,
    pub(crate) spectator: bool,
    /// Distância em blocos e se está olhando para você, quando o corpo está carregado.
    pub(crate) near: Option<(f64, bool)>,
}

/// Olhando para você: a cabeça aponta para o seu olho com folga de 12 graus, até 64 blocos.
fn looking(yaw: f32, pitch: f32, eye: [f64; 3], target: [f64; 3]) -> bool {
    let (dx, dy, dz) = (target[0] - eye[0], target[1] - eye[1], target[2] - eye[2]);
    (dx * dx + dy * dy + dz * dz).sqrt() <= 64.0 && crate::game::horizonx::angle_to(eye, yaw, pitch, target) <= 12.0
}

pub(crate) fn rows(game: &Game<'_>) -> Vec<StaffRow> {
    let Some(r) = game.remote.as_ref() else { return Vec::new() };
    let account = horizon_client::auth::current();
    let own_uuid = account.as_ref().map(|a| a.uuid);
    let own_names: Vec<String> = account
        .as_ref()
        .map(|a| a.name.to_lowercase())
        .into_iter()
        .chain(game.options.player_name.as_ref().map(|n| n.to_lowercase()))
        .collect();
    let mine = |uuid: u128, name: &str| Some(uuid) == own_uuid || own_names.iter().any(|o| *o == name.to_lowercase());

    let mut rows: std::collections::BTreeMap<u128, StaffRow> = std::collections::BTreeMap::new();
    for (&uuid, t) in &r.tab_players {
        let hidden = !r.listed.contains(&uuid);
        let spectator = t.game_mode == SPECTATOR;
        if (hidden || spectator) && !mine(uuid, &t.name) {
            rows.insert(uuid, StaffRow { name: t.name.clone(), hidden, spectator, near: None });
        }
    }
    if let Some(p) = game.player.as_ref() {
        let my_eye = [p.position.x, p.position.y + 1.62, p.position.z];
        for (&id, &uuid) in &r.level_players {
            if id == r.entity_id {
                continue;
            }
            let listed = r.listed.contains(&uuid);
            let spectator = r.tab_players.get(&uuid).is_some_and(|t| t.game_mode == SPECTATOR);
            if listed && !spectator {
                continue;
            }
            let name = r
                .level_player_names
                .get(&id)
                .or_else(|| r.players.get(&uuid))
                .cloned()
                .unwrap_or_else(|| format!("{uuid:032x}").chars().take(8).collect());
            if mine(uuid, &name) {
                continue;
            }
            let Some(mob) = game.mobs.iter().find(|x| x.id as i32 == id && !x.removed) else { continue };
            let eye = [mob.position.x, mob.position.y + mob.eye_height(), mob.position.z];
            let (dx, dy, dz) = (eye[0] - my_eye[0], eye[1] - my_eye[1], eye[2] - my_eye[2]);
            let distance = (dx * dx + dy * dy + dz * dz).sqrt();
            let row = rows
                .entry(uuid)
                .or_insert(StaffRow { name, hidden: !listed, spectator, near: None });
            row.near = Some((distance, looking(mob.head_yaw, mob.head_pitch, eye, my_eye)));
        }
    }
    let mut out: Vec<StaffRow> = rows.into_values().collect();
    out.sort_by(|a, b| {
        let d = |r: &StaffRow| r.near.map_or(f64::MAX, |n| n.0);
        d(a).total_cmp(&d(b)).then_with(|| a.name.cmp(&b.name))
    });
    out
}
