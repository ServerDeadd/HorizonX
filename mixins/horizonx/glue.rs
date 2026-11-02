// A ponte entre a lógica do Meteor (crate::game::horizonx) e o jogo: lê o jogador e as
// entidades, ataca, move, desenha, roda os comandos e o .locate. Fica dentro de `game` como
// crate::game::horizonx_glue, então enxerga os campos do Game; as decisões ficam em runtime.rs,
// que tem os testes.
use super::horizonx as m;
use super::{Game, Screen};
use horizon_client::gizmos::{Gizmos, Line};
use horizon_client::hud::HudBatch;
use horizon_client::mobs::Mob;
use horizon_client::vanillagen::VanillaTerrain;
use horizon_core::geometry::Vec3;
use horizon_protocol::packets::{
    Empty,
    play::{self, Clientbound, Serverbound as S},
};
use horizon_world::horizonx_shared as shared;
use glam::Mat4;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use winit::keyboard::KeyCode;

/// Altura do olho em pé e agachado.
const EYE: f64 = 1.62;
const EYE_SNEAK: f64 = 1.27;

fn loaded(game: &Game<'_>) {
    m::load(|key| game.native_setting(m::ID, key));
}

fn save(game: &Game<'_>, pairs: Vec<(String, String)>) {
    for (key, value) in pairs {
        game.native_set_setting(m::ID, &key, &value);
    }
}

fn me(game: &Game<'_>) -> Option<u32> {
    game.remote.as_ref().map(|r| r.entity_id as u32)
}

/// Uma entidade viva vista pelos módulos, na posição `pos`.
fn target_at(game: &Game<'_>, x: &Mob, pos: [f64; 3]) -> m::Target {
    let player = x.kind.id == "minecraft:player";
    let friend = player && game.remote_player_name(x.id).is_some_and(m::is_friend);
    m::Target {
        id: x.id,
        pos,
        width: f64::from(x.width()),
        height: f64::from(x.height()),
        health: x.health,
        player,
        hostile: x.hostile(),
        friend,
    }
}

fn living<'a>(game: &'a Game<'_>) -> impl Iterator<Item = &'a Mob> {
    let me = me(game);
    game.mobs.iter().filter(move |x| !x.removed && x.alive() && Some(x.id) != me)
}

fn targets(game: &Game<'_>) -> Vec<m::Target> {
    living(game)
        .map(|x| target_at(game, x, [x.position.x, x.position.y, x.position.z]))
        .collect()
}

/// Ataca a entidade `id` como o clique faz: o pacote, a recarga, o braço e o balanço.
fn attack(game: &mut Game<'_>, id: u32) {
    game.send(S::Attack(play::Attack { id: id as i32 }));
    game.reset_attack_strength();
    game.swing_arm();
    game.send(S::Punch(Empty));
}

/// Segura ou solta uma tecla de movimento que o módulo aperta por conta própria.
fn hold(game: &mut Game<'_>, key: KeyCode, down: bool, holding: &mut bool) {
    if down {
        game.keys.insert(key);
        *holding = true;
    } else if *holding {
        game.keys.remove(&key);
        *holding = false;
    }
}

// ----- Tick --------------------------------------------------------------------------------

/// Uma vez por tick, depois do movimento do tick (gancho `native_tick`).
pub(super) fn tick(game: &mut Game<'_>) {
    if !game.native_window() {
        return;
    }
    let on = game.native_on(m::ID);
    m::set_mod_on(on);
    flush_outbox(game);
    if !on || game.remote.is_none() {
        return;
    }
    loaded(game);
    horizon_client_input::horizonx_noslow::set(m::active("noslow"));
    let ticks = m::run(|r| {
        r.ticks += 1;
        r.ticks
    });

    // Timer: o relógio anda mais rápido; ao desligar volta aos 20 ticks por segundo.
    let timer = m::is_on("timer");
    shared::set_timer(timer);
    let was_timer = m::run(|r| std::mem::replace(&mut r.timer_applied, timer));
    if timer {
        game.clock.set_rate(20.0 * m::get_num("timer", "speed") as f32);
    } else if was_timer {
        game.clock.set_rate(20.0);
    }

    if m::is_on("autorespawn") && game.screen == Screen::Dead {
        game.remote_respawn();
        return;
    }
    let Some(p) = game.player.as_ref() else { return };
    let playing = game.native_playing();
    let on_ground = p.ground.on_ground;
    let pos = [p.position.x, p.position.y, p.position.z];
    let vy = p.velocity.y;
    let collided = p.collision.horizontal;
    let in_fluid = p.fluids.was_touching_water || p.is_in_lava();
    m::run(|r| r.fall = p.fall_distance);

    let mut held = m::run(|r| (r.holding_sprint, r.holding_jump, r.holding_forward, r.holding_sneak));
    let down = |game: &Game<'_>, k: KeyCode| game.keys.contains(&k);
    let (fw, bk, lf, rt) = (
        down(game, KeyCode::KeyW),
        down(game, KeyCode::KeyS),
        down(game, KeyCode::KeyA),
        down(game, KeyCode::KeyD),
    );
    // O que a pessoa apertou, sem contar as teclas que o próprio mod segura.
    let jump = down(game, KeyCode::Space) && !held.1;
    let sneak = down(game, KeyCode::ShiftLeft) && !held.3;
    let moving = fw || bk || lf || rt;
    let dir = m::move_dir(game.yaw, fw, bk, lf, rt);
    let mut want_jump = false;

    if m::is_on("fastuse") {
        game.use_delay = 0;
    }

    // Sprint, AutoWalk e Sneak seguram teclas.
    let sprint = m::is_on("sprint") && playing && !sneak && (fw || (m::get_flag("sprint", "omni") && moving));
    hold(game, KeyCode::ControlLeft, sprint, &mut held.0);
    hold(game, KeyCode::KeyW, playing && m::is_on("autowalk"), &mut held.2);
    hold(game, KeyCode::ShiftLeft, playing && m::is_on("sneak"), &mut held.3);

    // AntiAFK: de tempos em tempos pula e vira um pouco.
    if m::is_on("antiafk") {
        let every = (m::get_num("antiafk", "interval") * 20.0) as u64;
        let now = m::run(|r| {
            r.afk_ticks += 1;
            if r.afk_ticks >= every {
                r.afk_ticks = 0;
                true
            } else {
                false
            }
        });
        if now && playing {
            want_jump = true;
            game.yaw = m::wrap_degrees(game.yaw + 45.0);
        }
    }

    // Movimento
    let (was_on_ground, jump_was_down) = m::run(|r| (r.was_on_ground, r.jump_was_down));
    if playing && let Some(p) = game.player.as_mut() {
        if m::is_on("flight") {
            let v = m::flight_velocity(dir, jump, sneak, ticks);
            p.velocity.x = v[0];
            p.velocity.y = v[1];
            p.velocity.z = v[2];
        } else {
            if m::is_on("speed") && on_ground && moving {
                let h = m::speed_horizontal(dir);
                p.velocity.x = h[0];
                p.velocity.z = h[1];
                want_jump |= m::get_flag("speed", "autojump");
            }
            if m::is_on("spider") && collided && moving {
                p.velocity.y = m::get_num("spider", "speed");
            }
            if m::is_on("highjump") && was_on_ground && !on_ground && vy > 0.3 {
                p.velocity.y = vy * m::get_num("highjump", "multiplier");
            }
            if m::is_on("airjump") && jump && !jump_was_down && !on_ground {
                p.velocity.y = 0.42;
            }
            if m::is_on("jesus") && in_fluid && !sneak {
                p.velocity.y = p.velocity.y.max(0.11);
            }
        }
    }

    // Step: o degrau que o jogador sobe sem pular (o normal é 0,6 bloco).
    let step = m::is_on("step");
    let was_step = m::run(|r| std::mem::replace(&mut r.step_applied, step));
    if let Some(p) = game.player.as_mut()
        && (step || was_step)
    {
        let extra = (m::get_num("step", "height") - 0.6).max(0.0);
        let _ = p.set_modifier("minecraft:step_height", "horizonx:step", extra, "add_value", step);
    }

    // Combate
    let attack_down = game.native_mapping("key.attack").is_some_and(|k| k.0);
    let ready = game.hand_strength.scale(0.5, game.main_hand_attack_speed()) >= 0.95;
    let eye = [pos[0], pos[1] + if sneak { EYE_SNEAK } else { EYE }, pos[2]];
    if playing && m::is_on("killaura") && (!m::get_flag("killaura", "only_click") || attack_down) {
        let filter = m::get_pick("killaura", "targets");
        let skip_friends = m::get_flag("killaura", "ignore_friends");
        let list: Vec<m::Target> = targets(game)
            .into_iter()
            .filter(|t| m::wanted(filter, t) && !(skip_friends && t.friend))
            .collect();
        let range = m::get_num("killaura", "range");
        let priority = m::get_pick("killaura", "priority");
        if let Some(t) = m::pick_target(&list, eye, game.yaw, game.pitch, range, priority) {
            if m::get_flag("killaura", "rotate") {
                let (yaw, pitch) = m::look_at(eye, t.center());
                game.yaw = yaw;
                game.pitch = pitch;
            }
            let crits = m::is_on("criticals") && m::get_pick("criticals", "mode") == 0;
            if crits && on_ground {
                // Pulinho: ataca na descida, quando o golpe vira crítico.
                want_jump = true;
            } else if (ready || !m::get_flag("killaura", "cooldown")) && (!crits || vy < 0.0) {
                attack(game, t.id);
            }
        }
    }
    if playing && m::is_on("triggerbot") && ready {
        let filter = m::get_pick("triggerbot", "targets");
        let list: Vec<m::Target> = targets(game).into_iter().filter(|t| m::wanted(filter, t)).collect();
        if let Some(t) = m::under_crosshair(&list, eye, game.yaw, game.pitch, game.entity_reach) {
            attack(game, t.id);
        }
    }
    if playing && m::is_on("autoclicker") && attack_down {
        let cps = m::get_num("autoclicker", "cps").max(1.0);
        let fire = m::run(|r| {
            r.click_wait -= 1.0;
            let fire = r.click_wait <= 0.0;
            if fire {
                r.click_wait += 20.0 / cps;
            }
            fire
        });
        if fire {
            match m::under_crosshair(&targets(game), eye, game.yaw, game.pitch, game.entity_reach) {
                Some(t) => attack(game, t.id),
                None => {
                    game.swing_arm();
                    game.send(S::Punch(Empty));
                }
            }
        }
    }

    // StaffViewer: avisa no chat quem aparece fora da tab ou no espectador.
    if m::is_on("staffviewer") && m::get_flag("staffviewer", "alert") && ticks % 20 == 0 {
        let rows = super::remote::horizonx_staff::rows(game);
        let names: Vec<String> = rows.iter().map(|r| r.name.clone()).collect();
        for name in m::staff_new(&names) {
            if let Some(r) = rows.iter().find(|r| r.name == name) {
                let (_, state, distance, _) = m::staff_line(&r.name, r.hidden, r.spectator, r.near);
                game.native_chat(&format!("StaffViewer: {name} está {state}, {distance}"));
            }
        }
    }

    interact(game, playing);

    hold(game, KeyCode::Space, playing && want_jump, &mut held.1);
    m::run(|r| {
        (r.holding_sprint, r.holding_jump, r.holding_forward, r.holding_sneak) = held;
        r.was_on_ground = on_ground;
        r.jump_was_down = jump;
    });
}

// ----- Quadro: Fullbright e Xray -----------------------------------------------------------

/// Uma vez por quadro: o modo claro do renderizador, o Xray e o Freecam.
pub(super) fn frame(game: &mut Game<'_>, elapsed: std::time::Duration) {
    if !game.native_window() {
        return;
    }
    loaded(game);
    let on = game.native_on(m::ID);
    xray_sync(game, on && m::is_on("xray"));
    freecam_frame(game, on && m::is_on("freecam"), elapsed);
    let forced = on && (m::is_on("fullbright") || m::is_on("xray"));
    let chosen = game.settings.extras.fullbright;
    if let Some(r) = game.renderer.as_mut() {
        r.fullbright = forced || chosen;
    }
}

/// Liga ou desliga o Xray: monta a tabela de malha com o que não é minério invisível e manda
/// remontar os chunks (como o próprio jogo faz ao trocar as cores dos biomas).
fn xray_sync(game: &mut Game<'_>, want: bool) {
    let (applied, ours) = m::run(|r| (r.xray_applied, r.xray_tables));
    let current = Arc::as_ptr(&game.pack.context.tables) as usize;
    if want == applied && (!want || current == ours) {
        return;
    }
    let facts = game.res.facts();
    let keep: Vec<bool> = facts.states.iter().map(|s| shared::is_kept(&s.name, s.block_entity)).collect();
    let states = facts
        .states
        .iter()
        .zip(&keep)
        .map(|(s, &k)| {
            let mut s = s.clone();
            if want && !k {
                s.solid_render = false;
                s.full_collision = false;
                s.render_shape = horizon_items::blocks::RenderShape::Invisible;
            }
            s
        })
        .collect();
    let facts = horizon_items::blocks::BlockFacts { states, air: facts.air };
    shared::set_keep(keep);
    let old = game.pack.context.clone();
    let Ok(tables) =
        horizon_client::mesher::MeshTables::new(&old.assets, &old.render, &facts, old.tables.visibility.clone())
    else {
        return;
    };
    let tables = Arc::new(tables);
    let ptr = Arc::as_ptr(&tables) as usize;
    let context = Arc::new(horizon_client::streaming::WorkerContext {
        assets: old.assets.clone(),
        render: old.render.clone(),
        tables,
        biomes: old.biomes.clone(),
        light: old.light.clone(),
    });
    game.pack.context = context.clone();
    game.streaming.reload_resources(&mut game.world, context);
    shared::set_xray(want);
    m::run(|r| {
        r.xray_applied = want;
        r.xray_tables = if want { ptr } else { 0 };
    });
}

// ----- Desenho no mundo --------------------------------------------------------------------

/// ESP em caixa, Tracers e StorageESP, por cima das paredes (as linhas do F3+B por cima).
pub(super) fn gizmos(game: &Game<'_>, camera: [f64; 3], g: &mut Gizmos) {
    if !game.native_on(m::ID) {
        return;
    }
    loaded(game);
    let boxes = m::is_on("esp") && m::get_pick("esp", "mode") == 1;
    let tracers = m::is_on("tracers");
    if boxes || tracers {
        let (esp_filter, tracer_filter) = (m::get_pick("esp", "targets"), m::get_pick("tracers", "targets"));
        let dir = m::look_dir(game.yaw, game.pitch);
        let start = [camera[0] + dir[0], camera[1] + dir[1], camera[2] + dir[2]];
        // A fração do tick deste quadro: as caixas andam junto com os modelos.
        let a = f64::from(game.clock.fraction());
        for x in living(game) {
            let lerp = |o: f64, n: f64| o + a * (n - o);
            let pos = [
                lerp(x.previous.x, x.position.x),
                lerp(x.previous.y, x.position.y),
                lerp(x.previous.z, x.position.z),
            ];
            let t = target_at(game, x, pos);
            let color = 0xFF00_0000 | m::esp_color(&t);
            if boxes && m::wanted(esp_filter, &t) {
                g.cuboid_on_top(t.min(), t.max(), color, 2.0);
            }
            if tracers && m::wanted(tracer_filter, &t) {
                g.on_top.push(Line {
                    start,
                    end: t.center(),
                    color,
                    width: 1.5,
                });
            }
        }
    }
    if m::is_on("storageesp") {
        let only_chests = m::get_pick("storageesp", "which") == 1;
        let facts = game.res.facts();
        for (&(x, y, z), _) in game.world.block_entities.iter() {
            let (dx, dy, dz) = (f64::from(x) - camera[0], f64::from(y) - camera[1], f64::from(z) - camera[2]);
            if dx * dx + dy * dy + dz * dz > 96.0 * 96.0 {
                continue;
            }
            let id = game.world.block(x, y, z);
            let Some(state) = facts.states.get(id.0 as usize) else { continue };
            let Some(rgb) = storage_color(&state.name, only_chests) else { continue };
            let min = [f64::from(x) + 0.06, f64::from(y), f64::from(z) + 0.06];
            let max = [f64::from(x) + 0.94, f64::from(y) + 0.88, f64::from(z) + 0.94];
            g.cuboid_on_top(min, max, 0xFF00_0000 | rgb, 2.0);
        }
    }
    if m::is_on("trajectories") {
        trajectory(game, g);
    }
}

/// A trajetória do que você segura (flecha, bola de neve, pérola, tridente).
fn trajectory(game: &Game<'_>, g: &mut Gizmos) {
    let Some(p) = game.player.as_ref() else { return };
    let Some(stack) = game.held() else { return };
    let Some((speed, gravity, drag)) = m::projectile_params(&stack.item) else { return };
    let dir = m::look_dir(game.yaw, game.pitch);
    let start = [p.position.x, p.position.y + 1.42, p.position.z];
    let vel = [dir[0] * speed, dir[1] * speed, dir[2] * speed];
    let points = m::arc(start, vel, gravity, drag, 160);
    for pair in points.windows(2) {
        g.on_top.push(Line {
            start: pair[0],
            end: pair[1],
            color: 0xFF55_FF99,
            width: 1.5,
        });
    }
    if let Some(last) = points.last() {
        let d = 0.15;
        g.cuboid_on_top([last[0] - d, last[1] - d, last[2] - d], [last[0] + d, last[1] + d, last[2] + d], 0xFFFF_5555, 2.0);
    }
}

/// Cor do StorageESP por bloco (RGB); None para o que não é armazenamento.
fn storage_color(name: &str, only_chests: bool) -> Option<u32> {
    let id = name.strip_prefix("minecraft:").unwrap_or(name);
    let rgb = if id.contains("shulker_box") {
        0xC0_60FF
    } else if id.contains("chest") {
        0xFF_A040
    } else if id == "barrel" {
        0xB0_7840
    } else if only_chests {
        return None;
    } else if matches!(id, "furnace" | "blast_furnace" | "smoker") {
        0x9A_9AA6
    } else if matches!(id, "hopper" | "dispenser" | "dropper" | "crafter") {
        0x6E_6E80
    } else {
        return None;
    };
    Some(rgb)
}

/// O contorno brilhante do ESP: a entidade brilha na cor do Meteor.
pub(super) fn outline(game: &Game<'_>, x: &Mob, value: Option<u32>) -> Option<u32> {
    if value.is_some() || !game.native_on(m::ID) {
        return value;
    }
    loaded(game);
    if !m::is_on("esp") || m::get_pick("esp", "mode") != 0 || x.removed || !x.alive() || Some(x.id) == me(game) {
        return value;
    }
    let t = target_at(game, x, [x.position.x, x.position.y, x.position.z]);
    m::wanted(m::get_pick("esp", "targets"), &t).then(|| m::esp_color(&t)).or(value)
}

/// NoRender: neblina lá longe.
pub(super) fn fog(game: &Game<'_>, fog: &mut horizon_client::fog::FogData) {
    if !game.native_on(m::ID) {
        return;
    }
    loaded(game);
    if (m::is_on("norender") && m::get_flag("norender", "fog")) || m::is_on("xray") {
        const FAR: f32 = 1.0e6;
        fog.environmental_start = FAR;
        fog.environmental_end = FAR + 1.0;
        fog.render_distance_start = FAR;
        fog.render_distance_end = FAR + 1.0;
    }
}

/// NoRender: sem efeitos por cima da tela (fogo, abóbora, água, portal, náusea).
pub(super) fn hide_overlays(game: &Game<'_>) -> bool {
    game.native_on(m::ID) && m::is_on("norender") && m::get_flag("norender", "overlays")
}

// ----- Pacotes, alcance --------------------------------------------------------------------

/// Velocity: troca o empurrão que o servidor manda (golpes, explosões) pelo do ajuste.
/// Devolve true quando já tratou o pacote.
pub(super) fn velocity_packet(game: &mut Game<'_>, packet: &Clientbound) -> bool {
    if !m::active("velocity") {
        return false;
    }
    let h = m::get_num("velocity", "horizontal") / 100.0;
    let v = m::get_num("velocity", "vertical") / 100.0;
    match packet {
        Clientbound::SetEntityMotion(p) if game.remote.as_ref().is_some_and(|r| r.entity_id == p.id) => {
            if let Some(pl) = game.player.as_mut() {
                pl.velocity = Vec3::new(p.velocity[0] * h, p.velocity[1] * v, p.velocity[2] * h);
            }
            true
        }
        Clientbound::Explode(p) => {
            let mut q = p.clone();
            if let Some(k) = q.knockback.as_mut() {
                k[0] *= h;
                k[1] *= v;
                k[2] *= h;
            }
            game.remote_explode(&q);
            true
        }
        _ => false,
    }
}

/// Reach: o alcance de entidades e blocos.
pub(super) fn reach(_game: &Game<'_>) -> Option<f64> {
    m::active("reach").then(|| m::get_num("reach", "range"))
}

// ----- HUD ---------------------------------------------------------------------------------

/// A lista de módulos ligados (canto de cima à direita) e as informações (à esquerda).
pub(super) fn hud(game: &mut Game<'_>, batch: &mut HudBatch, corners: &mut horizon_client::extras_hud::Corners) {
    use horizon_client::extras::Corner;
    if !game.native_on(m::ID) {
        return;
    }
    loaded(game);
    let game = &*game;
    if m::is_on("nametags") {
        nametags(game, batch);
    }
    let line_h = 11.0;
    if m::hud_shown() && game.native_corner_shown(Corner::TopRight) {
        let title = format!("{} {}", m::NAME, m::VERSION);
        let lines = m::active_lines(|t| game.native_text_width(t));
        let widest = lines
            .iter()
            .map(|(t, _)| game.native_text_width(t))
            .fold(game.native_text_width(&title), f32::max);
        let (x, y) = corners.place(Corner::TopRight, widest + 8.0, line_h * (lines.len() + 1) as f32);
        let right = x + widest + 8.0;
        let tw = game.native_text_width(&title);
        game.native_text(batch, &title, [right - tw - 3.0, y + 1.5], m::ACCENT, true);
        for (n, (text, color)) in lines.iter().enumerate() {
            let tw = game.native_text_width(text);
            let ly = y + line_h * (n + 1) as f32;
            game.native_fill(batch, [right - tw - 5.0, ly, tw + 5.0, line_h], 0x8800_0000);
            game.native_fill(batch, [right - 1.0, ly, 1.0, line_h], 0xFF00_0000 | color);
            game.native_text(batch, text, [right - tw - 3.0, ly + 1.5], *color, true);
        }
    }
    if !game.native_corner_shown(Corner::TopLeft) {
        return;
    }
    staff_table(game, batch, corners);
    let Some(p) = game.player.as_ref() else { return };
    let speed = (p.velocity.x * p.velocity.x + p.velocity.z * p.velocity.z).sqrt() * 20.0;
    let lines = m::info_lines(
        [p.position.x, p.position.y, p.position.z],
        game.dim.id() == "minecraft:the_nether",
        game.native_frame_rate().0,
        speed,
        game.yaw,
    );
    if lines.is_empty() {
        return;
    }
    let widest = lines.iter().map(|t| game.native_text_width(t)).fold(0.0, f32::max);
    let (x, y) = corners.place(Corner::TopLeft, widest + 8.0, line_h * lines.len() as f32);
    for (n, text) in lines.iter().enumerate() {
        game.native_text(batch, text, [x + 3.0, y + line_h * n as f32 + 1.5], 0xE6_E6EE, true);
    }
}

/// A tabelinha do StaffViewer no canto de cima à esquerda: nome, situação e distância.
fn staff_table(game: &Game<'_>, batch: &mut HudBatch, corners: &mut horizon_client::extras_hud::Corners) {
    use horizon_client::extras::Corner;
    if !m::is_on("staffviewer") {
        return;
    }
    let rows: Vec<(String, String, String, u32)> = super::remote::horizonx_staff::rows(game)
        .iter()
        .map(|r| m::staff_line(&r.name, r.hidden, r.spectator, r.near))
        .collect();
    let line_h = 11.0;
    let gap = 8.0;
    let w = |t: &str| game.native_text_width(t);
    let title = "StaffViewer";
    let empty = "Ninguém escondido";
    let (c0, c1, c2) = rows.iter().fold((w(title), 0.0f32, 0.0f32), |(a, b, c), r| {
        (a.max(w(&r.0)), b.max(w(&r.1)), c.max(w(&r.2)))
    });
    let width = if rows.is_empty() { w(title).max(w(empty)) + 8.0 } else { c0 + c1 + c2 + gap * 2.0 + 8.0 };
    let height = line_h * (rows.len().max(1) + 1) as f32 + 4.0;
    let (x, y) = corners.place(Corner::TopLeft, width, height);
    game.native_fill(batch, [x, y, width, height], 0x9000_0000);
    game.native_fill(batch, [x, y, width, 1.0], 0xFF00_0000 | m::ACCENT);
    game.native_text(batch, title, [x + 4.0, y + 3.0], m::ACCENT, true);
    if rows.is_empty() {
        game.native_text(batch, empty, [x + 4.0, y + 3.0 + line_h], 0x9A_9AA6, false);
        return;
    }
    for (n, (name, state, distance, color)) in rows.iter().enumerate() {
        let ly = y + 3.0 + line_h * (n + 1) as f32;
        game.native_text(batch, name, [x + 4.0, ly], *color, true);
        game.native_text(batch, state, [x + 4.0 + c0 + gap, ly], 0xB8_B8C4, false);
        game.native_text(batch, distance, [x + 4.0 + c0 + c1 + gap * 2.0, ly], *color, false);
    }
}

// ----- Teclas ------------------------------------------------------------------------------

/// Teclas: a do menu abre e fecha o ClickGUI; as dos módulos ligam e desligam.
pub(super) fn key(game: &mut Game<'_>, key: &str, pressed: bool) {
    if !pressed || !game.native_on(m::ID) || !game.native_window() || m::binding().is_some() {
        return;
    }
    loaded(game);
    let menu = game.native_mod_key(m::ID, m::MENU_KEY, m::MENU_DEFAULT);
    if key == menu {
        if game.native_mod_screen() == Some(m::ID) {
            game.native_mod_screen_close(m::ID);
        } else if game.native_playing() {
            game.native_mod_screen_open(m::ID);
        }
        return;
    }
    if !game.native_playing() {
        return;
    }
    for i in m::modules_for_key(key) {
        let (store, line) = m::toggle(i);
        save(game, store);
        game.native_chat(&line);
    }
}

/// Na tela do ClickGUI: a tecla que o módulo esperava (Esc cancela, Backspace tira).
pub(super) fn gui_key(game: &Game<'_>, screen_id: &str, key: &str, canceled: &mut bool) {
    if !ours(game, screen_id) {
        return;
    }
    let Some(i) = m::binding() else { return };
    *canceled = true;
    if m::is_cancel_key(key) {
        m::stop_binding();
    } else if m::is_clear_key(key) {
        save(game, vec![m::set_bind(i, "")]);
    } else {
        save(game, vec![m::set_bind(i, key)]);
    }
}

// ----- Comandos ----------------------------------------------------------------------------

/// Linhas do chat que começam com ponto: comandos do Meteor, que não vão para o servidor.
pub(super) fn chat(game: &mut Game<'_>, message: &str) -> bool {
    let line = horizon_client::chat::normalize_chat_message(message);
    let Some(args) = line.strip_prefix(m::PREFIX) else { return false };
    if args.starts_with(m::PREFIX) || !game.native_on(m::ID) || !game.native_window() {
        return false;
    }
    game.ui.chat.add_recent(&line);
    run_command(game, args);
    true
}

/// /m continua valendo, para quem preferir.
pub(super) fn slash_command(game: &mut Game<'_>, command: &str, ran: &mut bool) {
    let args = match command.split_once(' ') {
        Some(("m", rest)) => rest,
        None if command == "m" => "",
        _ => return,
    };
    if !game.native_on(m::ID) {
        return;
    }
    *ran = true;
    run_command(game, args);
}

fn run_command(game: &mut Game<'_>, args: &str) {
    loaded(game);
    let reply = m::command(args);
    save(game, reply.store);
    for line in reply.chat {
        game.native_chat(&line);
    }
    match reply.action {
        Some(m::Action::Locate(what)) => locate(game, &what),
        Some(m::Action::Seed(text)) => seed(game, text),
        Some(m::Action::Nick(name)) => nick(game, name),
        None => {}
    }
}

/// Respostas que chegam de outras threads (o .locate) e entram no chat no próximo tick.
static OUTBOX: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// Ajustes (as estruturas já visitadas) que a thread do .locate manda guardar.
static SAVE: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());
static LOCATING: AtomicBool = AtomicBool::new(false);
/// O gerador montado a partir de uma seed digitada, para não montar de novo.
static TERRAIN: Mutex<Option<(i64, &'static str, Arc<VanillaTerrain>)>> = Mutex::new(None);

fn flush_outbox(game: &mut Game<'_>) {
    let lines: Vec<String> = OUTBOX.lock().map(|mut o| std::mem::take(&mut *o)).unwrap_or_default();
    for line in lines {
        game.native_chat(&line);
    }
    let saves: Vec<(String, String)> = SAVE.lock().map(|mut o| std::mem::take(&mut *o)).unwrap_or_default();
    for (key, value) in saves {
        game.native_set_setting(m::ID, &key, &value);
    }
}

/// A seed do mundo aberto no singleplayer, se houver.
fn local_seed(game: &Game<'_>) -> Option<i64> {
    game.integrated.as_ref().and_then(|s| s.call(|g| g.level.seed))
}

fn seed(game: &mut Game<'_>, text: Option<String>) {
    match text {
        Some(t) => {
            let n = m::parse_seed(&t);
            game.native_set_setting(m::ID, "seed", &n.to_string());
            game.native_chat(&format!("Seed guardada: {n}. Agora é só usar .locate"));
        }
        None => {
            let line = match local_seed(game) {
                Some(n) => format!("Seed deste mundo: {n}"),
                None => match m::stored_seed(|k| game.native_setting(m::ID, k)) {
                    Some(n) => format!("Seed guardada: {n} (o servidor não manda a seed de verdade)"),
                    None => "Sem seed. Num servidor, use .seed <número>".to_owned(),
                },
            };
            game.native_chat(&line);
        }
    }
}

fn locate(game: &mut Game<'_>, text: &str) {
    let Some(p) = game.player.as_ref() else {
        game.native_chat("Entre num mundo para usar o .locate");
        return;
    };
    let from = [p.position.x.floor() as i32, p.position.y.floor() as i32, p.position.z.floor() as i32];
    let dim: &'static str = game.dim.id();
    let (query, label) = m::locate_query(text);
    let names = match &query {
        m::Query::Tag(tag) => {
            let mut list = game.structure_tag(tag);
            if list.is_empty() {
                list = game.structure_tag(tag.trim_start_matches("minecraft:"));
            }
            list
        }
        m::Query::Structures(list) | m::Query::Biomes(list) => list.clone(),
    };
    let integrated = game.integrated.as_ref().and_then(|s| s.terrain(dim));
    let seed = m::stored_seed(|k| game.native_setting(m::ID, k));
    if integrated.is_none() && seed.is_none() {
        game.native_chat("Sem seed para procurar. Num servidor, use .seed <número> primeiro");
        return;
    }
    if names.is_empty() {
        game.native_chat(&format!("Não sei o que é {label}"));
        return;
    }
    if LOCATING.swap(true, Ordering::SeqCst) {
        game.native_chat("Ainda procurando a busca anterior, espere um pouco");
        return;
    }
    game.native_chat(&format!("Procurando {label}"));
    let dir = game.options.resources.join("worldgen");
    let registry = game.pack.assets.registry;
    let biomes = game.res.biomes().clone();
    let (min_y, max_y) = match dim {
        "minecraft:overworld" => (-64, 319),
        _ => (0, 255),
    };
    // As estruturas já visitadas deste tipo, para apontar a próxima.
    let biome_search = matches!(query, m::Query::Biomes(_));
    let seen_key = format!("seen_{}", text.trim().to_lowercase().replace(' ', "_"));
    let visited_in = if biome_search { String::new() } else { game.native_setting(m::ID, &seen_key).unwrap_or_default() };

    std::thread::spawn(move || {
        let terrain = integrated.or_else(|| {
            let seed = seed?;
            let mut cache = TERRAIN.lock().ok()?;
            if let Some((s, d, t)) = cache.as_ref()
                && *s == seed
                && *d == dim
            {
                return Some(t.clone());
            }
            let t = Arc::new(
                VanillaTerrain::for_world(&dir, registry, &biomes, seed, "minecraft:normal", dim, true).ok()?,
            );
            *cache = Some((seed, dim, t.clone()));
            Some(t)
        });
        let mut save = None;
        let line = match terrain {
            None => "Não deu para montar o gerador desse mundo com essa seed".to_owned(),
            Some(t) if biome_search => {
                let allowed = |n: &str| names.iter().any(|x| x == n);
                let search = horizon_world::worldgen::generator::BiomeSearch {
                    allowed: &allowed,
                    origin: from,
                    radius: 6400,
                    horizontal: 32,
                    vertical: 64,
                    min_y,
                    max_y,
                };
                match t.find_closest_biome(&search) {
                    Some((_, at)) => m::found_line(&label, at, from),
                    None => format!("Não achei {label} por perto"),
                }
            }
            Some(t) => {
                // Pula a que você já está em cima (a menos de 100 blocos) e as já visitadas,
                // guardando-as, e aponta a próxima.
                let mut visited = m::visited_parse(&visited_in);
                let mut origin = from;
                let mut result = None;
                for _ in 0..8 {
                    let Some((_, at)) = t.locate(&names, origin, 100) else { break };
                    let near = m::flat_dist(from, at) <= 100.0;
                    let seen = m::is_visited(&visited, at, 8);
                    if near || seen {
                        if !seen {
                            visited.push([at[0], at[2]]);
                        }
                        origin = m::next_origin(from, at, 700.0);
                        continue;
                    }
                    result = Some(at);
                    break;
                }
                save = Some((seen_key, m::visited_text(&visited)));
                match result {
                    Some(at) => format!("{} ({} já visitadas)", m::found_line(&label, at, from), visited.len()),
                    None => format!("Já passei por todas as {label} que achei por perto"),
                }
            }
        };
        if let Ok(mut out) = OUTBOX.lock() {
            out.push(line);
        }
        if let Some(kv) = save {
            if let Ok(mut s) = SAVE.lock() {
                s.push(kv);
            }
        }
        LOCATING.store(false, Ordering::SeqCst);
    });
}

/// .nick: o jogo se vê sem conta e entra no próximo servidor com esse nome.
fn nick(game: &mut Game<'_>, name: Option<String>) {
    match name {
        Some(name) => {
            let valid = (3..=16).contains(&name.len()) && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
            if !valid {
                game.native_chat("O nick precisa ter de 3 a 16 letras, números ou _");
                return;
            }
            let before = game.options.player_name.clone();
            m::run(|r| {
                r.name_before_nick.get_or_insert(before);
            });
            game.options.player_name = Some(name.clone());
            shared::set_nick(true);
            game.native_chat(&format!(
                "Nick {name} ligado. Vale no próximo servidor que você entrar, e só servidores com modo offline aceitam"
            ));
        }
        None => {
            if let Some(before) = m::run(|r| r.name_before_nick.take()) {
                game.options.player_name = before;
            }
            shared::set_nick(false);
            game.native_chat("Voltou ao nome da sua conta no próximo servidor que entrar");
        }
    }
}

// ----- ClickGUI ----------------------------------------------------------------------------

const PANEL: u32 = 0xE811_1115;
const PANEL_ON: u32 = 0xE82A_1642;
const PANEL_HOVER: u32 = 0xE81D_1D24;
const SETTING: u32 = 0xF00B_0B0F;
const SETTING_HOVER: u32 = 0xF016_1620;

fn ours(game: &Game<'_>, screen_id: &str) -> bool {
    screen_id == "mod" && game.native_mod_screen() == Some(m::ID)
}

/// Desenha o ClickGUI por cima do mundo (gancho `native_screen_render_post`).
pub(super) fn gui_draw(game: &Game<'_>, batch: &mut HudBatch, screen_id: &str) {
    if !ours(game, screen_id) {
        return;
    }
    loaded(game);
    m::set_gui_width(batch.width);
    let layout = m::gui_layout(batch.width);
    let [cx, cy] = game.native_cursor();
    let hover = m::gui_hit(&layout, cx, cy);
    game.native_fill(batch, [0.0, 0.0, batch.width, batch.height], 0x5500_0000);
    let width = |t: &str| game.native_text_width(t);
    for r in &layout {
        let [x, y, w, h] = r.rect;
        let hovered = hover == Some(r.item);
        match r.item {
            m::GuiItem::Header(c) => {
                let cat = m::Category::ALL[c];
                game.native_fill(batch, [x, y, w, h], 0xFF00_0000 | m::ACCENT);
                let tw = width(cat.name());
                game.native_text(batch, cat.name(), [x + (w - tw) / 2.0, y + 5.0], 0xFF_FFFF, true);
            }
            m::GuiItem::Module(i) => {
                let module = &m::MODULES[i];
                let on = m::is_on(module.id);
                let bg = if on { PANEL_ON } else if hovered { PANEL_HOVER } else { PANEL };
                game.native_fill(batch, [x, y, w, h], bg);
                if on {
                    game.native_fill(batch, [x, y, 2.0, h], 0xFF00_0000 | m::ACCENT);
                }
                let ink = if on { 0xFF_FFFF } else { 0xB8_B8C4 };
                game.native_text(batch, module.name, [x + 6.0, y + 3.0], ink, true);
                let bind = m::bind_of(i);
                if !bind.is_empty() {
                    let label = m::key_label(&bind);
                    let lw = width(&label);
                    game.native_text(batch, &label, [x + w - lw - 5.0, y + 3.0], 0x6E_6E7A, false);
                }
            }
            m::GuiItem::Bind(i) => {
                game.native_fill(batch, [x, y, w, h], if hovered { SETTING_HOVER } else { SETTING });
                game.native_fill(batch, [x, y, 2.0, h], 0x6600_0000 | m::ACCENT);
                let value = m::bind_text(i);
                let vw = width(&value);
                let waiting = m::binding() == Some(i);
                game.native_text(batch, "Tecla", [x + 8.0, y + 3.0], 0x9A_9AA6, false);
                let ink = if waiting { 0xFF_D24A } else { 0xC9_A0FF };
                game.native_text(batch, &value, [x + w - vw - 4.0, y + 3.0], ink, false);
            }
            m::GuiItem::Setting(i, j) => {
                game.native_fill(batch, [x, y, w, h], if hovered { SETTING_HOVER } else { SETTING });
                game.native_fill(batch, [x, y, 2.0, h], 0x6600_0000 | m::ACCENT);
                let (label, value) = m::setting_text(i, j);
                let vw = width(&value);
                let label = m::fit(label, w - vw - 16.0, width);
                game.native_text(batch, &label, [x + 8.0, y + 3.0], 0x9A_9AA6, false);
                game.native_text(batch, &value, [x + w - vw - 4.0, y + 3.0], 0xC9_A0FF, false);
            }
        }
    }
    let hint = if m::binding().is_some() {
        "Aperte a tecla do módulo. Esc cancela, Backspace tira a tecla."
    } else {
        "Esquerdo liga e desliga. Direito abre os ajustes. Meio escolhe a tecla. Esc fecha."
    };
    let hw = width(hint);
    game.native_text(batch, hint, [(batch.width - hw) / 2.0, batch.height - 14.0], 0x8A_8A96, true);
    if let Some(m::GuiItem::Module(i)) = hover {
        let tip = m::MODULES[i].description;
        let tw = width(tip);
        let tx = (cx + 10.0).min(batch.width - tw - 6.0).max(2.0);
        let ty = (cy + 12.0).min(batch.height - 30.0);
        game.native_fill(batch, [tx - 3.0, ty - 3.0, tw + 6.0, 14.0], 0xF00A_0A0D);
        game.native_text(batch, tip, [tx, ty], 0xE6_E6EE, false);
    }
}

/// Clique no ClickGUI (gancho `native_screen_mouse_button_pressed_pre`).
pub(super) fn gui_click(game: &Game<'_>, screen_id: &str, button: i32, canceled: &mut bool) {
    if !ours(game, screen_id) {
        return;
    }
    *canceled = true;
    loaded(game);
    let layout = m::gui_layout(m::gui_width());
    let [x, y] = game.native_cursor();
    if let Some(item) = m::gui_hit(&layout, x, y) {
        save(game, m::gui_press(item, button));
    }
}

// ----- Nametags ----------------------------------------------------------------------------

/// Projeta um ponto do mundo na tela (pixels da HUD); None se estiver atrás ou fora.
fn project(cam: [f64; 3], yaw: f32, pitch: f32, fov: f32, w: f32, h: f32, world: [f64; 3]) -> Option<[f32; 2]> {
    let view = horizon_client::camera::view_matrix(yaw, pitch);
    let proj = Mat4::from_cols_array(&horizon_client::camera::perspective(fov, w, h, 0.05, 1000.0, true));
    let rel = glam::Vec3::new((world[0] - cam[0]) as f32, (world[1] - cam[1]) as f32, (world[2] - cam[2]) as f32);
    let clip = proj * (view * rel.extend(1.0));
    if clip.w <= 0.01 {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    if ndc.x < -1.2 || ndc.x > 1.2 || ndc.y < -1.2 || ndc.y > 1.2 {
        return None;
    }
    Some([(ndc.x * 0.5 + 0.5) * w, (0.5 - ndc.y * 0.5) * h])
}

/// Nome e vida dos jogadores acima da cabeça, através das paredes.
fn nametags(game: &Game<'_>, batch: &mut HudBatch) {
    let Some((cam, yaw, pitch, fov)) = game.locator_camera else { return };
    let show_health = m::get_flag("nametags", "health");
    let me = me(game);
    for x in living(game) {
        if x.kind.id != "minecraft:player" || Some(x.id) == me {
            continue;
        }
        let head = [x.position.x, x.position.y + f64::from(x.height()) + 0.5, x.position.z];
        let Some([sx, sy]) = project(cam, yaw, pitch, fov, batch.width, batch.height, head) else { continue };
        let name = game.remote_player_name(x.id).map(str::to_owned).unwrap_or_else(|| "?".into());
        let text = if show_health { format!("{name}  {:.0}", x.health) } else { name };
        let tw = game.native_text_width(&text);
        let x0 = sx - tw / 2.0;
        game.native_fill(batch, [x0 - 2.0, sy - 10.0, tw + 4.0, 11.0], 0xA000_0000);
        let color = if x.health <= 6.0 { 0xFF_5555 } else { 0xFF_FFFF };
        game.native_text(batch, &text, [x0, sy - 8.5], color, true);
    }
}

// ----- Freecam -----------------------------------------------------------------------------

/// O gancho da câmera pergunta aqui: com o Freecam ligado, a câmera vai para a posição solta.
pub(super) fn apply_freecam_camera(game: &mut Game<'_>) -> Option<(glam::DVec3, f32, f32)> {
    loaded(game);
    if !game.native_on(m::ID) || !m::is_on("freecam") {
        return None;
    }
    let (pos, yaw, pitch) = m::run(|r| (r.freecam_pos, r.freecam_yaw, r.freecam_pitch));
    game.view.eye = pos;
    game.view.eye_rotation = [yaw, pitch];
    game.view.detached = true;
    Some((glam::DVec3::from_array(pos), yaw, pitch))
}

/// O gancho do mouse manda o giro para a câmera solta e deixa o personagem parado.
pub(super) fn freecam_turn(game: &Game<'_>, xo: f64, yo: f64) -> bool {
    if !game.native_on(m::ID) || !m::is_on("freecam") {
        return false;
    }
    m::run(|r| {
        r.freecam_yaw = m::wrap_degrees(r.freecam_yaw + xo as f32 * 0.15);
        r.freecam_pitch = (r.freecam_pitch + yo as f32 * 0.15).clamp(-90.0, 90.0);
    });
    true
}

/// O gancho do tick do movimento: com o Freecam ligado, solta as teclas de andar para o
/// personagem ficar parado e não mandar pacotes de movimento.
pub(super) fn freecam_freeze(game: &mut Game<'_>) {
    if !game.native_window() || !game.native_on(m::ID) || !m::is_on("freecam") {
        return;
    }
    for k in [
        KeyCode::KeyW,
        KeyCode::KeyS,
        KeyCode::KeyA,
        KeyCode::KeyD,
        KeyCode::Space,
        KeyCode::ShiftLeft,
        KeyCode::ControlLeft,
    ] {
        game.keys.remove(&k);
    }
}

/// A cada quadro move a câmera solta pelas teclas; liga e desliga comparando com o quadro
/// anterior (ao ligar, começa de onde o jogador está).
fn freecam_frame(game: &mut Game<'_>, on: bool, elapsed: std::time::Duration) {
    let was = m::run(|r| r.freecam_on);
    if on && !was {
        if let Some(p) = game.player.as_ref() {
            let pos = [p.position.x, p.position.y + 1.62, p.position.z];
            let (yaw, pitch) = (game.yaw, game.pitch);
            m::run(|r| {
                r.freecam_pos = pos;
                r.freecam_yaw = yaw;
                r.freecam_pitch = pitch;
                r.freecam_on = true;
            });
        }
    } else if !on && was {
        m::run(|r| r.freecam_on = false);
    }
    if !on || !game.native_playing() {
        return;
    }
    let dt = elapsed.as_secs_f64().clamp(0.0, 0.1);
    let speed = m::get_num("freecam", "speed") * 10.0 * dt;
    let k = |c| game.physical_keys.contains(&c);
    let yaw = m::run(|r| r.freecam_yaw);
    let d = m::freecam_delta(
        yaw,
        k(KeyCode::KeyW),
        k(KeyCode::KeyS),
        k(KeyCode::KeyA),
        k(KeyCode::KeyD),
        k(KeyCode::Space),
        k(KeyCode::ShiftLeft),
        speed,
    );
    m::run(|r| {
        for i in 0..3 {
            r.freecam_pos[i] += d[i];
        }
    });
}

// ----- Interações: AutoEat, AutoTotem, AutoTool, Nuker, Scaffold ---------------------------

fn hotbar_slot(game: &Game<'_>, id: &str) -> Option<usize> {
    (0..9).find(|&s| game.stack_in(s).is_some_and(|st| &*st.item == id))
}

fn food_slot(game: &Game<'_>) -> Option<usize> {
    (0..9).find(|&s| {
        game.stack_in(s)
            .is_some_and(|st| game.items().get(&st.item).is_some_and(|i| i.food.is_some()))
    })
}

fn interact(game: &mut Game<'_>, playing: bool) {
    if !playing {
        return;
    }
    if m::is_on("autoeat") {
        autoeat(game);
    }
    if m::is_on("autototem") {
        autototem(game);
    }
    if m::is_on("autotool") {
        autotool(game);
    }
    if m::is_on("nuker") {
        nuker(game);
    }
    if m::is_on("scaffold") {
        scaffold(game);
    }
}

fn autoeat(game: &mut Game<'_>) {
    let hunger = match game.player.as_ref() {
        Some(p) => p.health.food.food,
        None => return,
    };
    let threshold = m::get_num("autoeat", "hunger") as i32;
    let eating = m::run(|r| r.eating);
    if hunger < 20 && (eating || hunger <= threshold) {
        if let Some(slot) = food_slot(game) {
            if !eating {
                let prev = game.selected;
                m::run(|r| {
                    r.eating = true;
                    r.eat_prev = prev;
                });
            }
            game.selected = slot;
            game.right = true;
            return;
        }
    }
    if eating {
        game.right = false;
        let prev = m::run(|r| {
            r.eating = false;
            r.eat_prev
        });
        game.selected = prev;
    }
}

fn autototem(game: &mut Game<'_>) {
    let ready = game.player.as_ref().is_none_or(|p| p.health.totem_ready);
    if ready {
        return;
    }
    if game.offhand.as_ref().is_some_and(|s| &*s.item == "minecraft:totem_of_undying") {
        return;
    }
    if let Some(slot) = hotbar_slot(game, "minecraft:totem_of_undying") {
        let prev = game.selected;
        game.selected = slot;
        game.remote_swap_hands();
        game.selected = prev;
    }
}

fn autotool(game: &mut Game<'_>) {
    if !game.left {
        if let Some(prev) = m::run(|r| r.tool_prev.take()) {
            if m::get_flag("autotool", "switchback") {
                game.selected = prev;
            }
        }
        return;
    }
    let Some(hit) = game.pick() else { return };
    let state = game.world.block(hit.pos[0], hit.pos[1], hit.pos[2]);
    let name = game.res.facts().get(state).name.clone();
    let scores: Vec<f32> = (0..9).map(|s| game.items().mining_speed(game.stack_in(s), &name)).collect();
    if let Some(best) = m::best_index(&scores) {
        let cur = game.selected.min(8);
        if scores[best] > scores[cur] + 0.01 {
            let sel = game.selected;
            m::run(|r| {
                if r.tool_prev.is_none() {
                    r.tool_prev = Some(sel);
                }
            });
            game.selected = best;
        }
    }
}

fn nuker(game: &mut Game<'_>) {
    if !game.creative {
        return;
    }
    let Some(p) = game.player.as_ref() else { return };
    let eye = [p.position.x, p.position.y + 1.62, p.position.z];
    let feet_y = p.position.y.floor() as i32;
    let range = m::get_num("nuker", "range");
    let only_down = m::get_flag("nuker", "down");
    let speed = m::get_num("nuker", "speed").max(1.0) as usize;
    let center = [eye[0].floor() as i32, eye[1].floor() as i32, eye[2].floor() as i32];
    let r = range.ceil() as i32;
    let air = game.world.air;
    let mut targets: Vec<([i32; 3], f64)> = Vec::new();
    for dx in -r..=r {
        for dy in -r..=r {
            for dz in -r..=r {
                let pos = [center[0] + dx, center[1] + dy, center[2] + dz];
                if only_down && pos[1] >= feet_y {
                    continue;
                }
                let c = [f64::from(pos[0]) + 0.5, f64::from(pos[1]) + 0.5, f64::from(pos[2]) + 0.5];
                let d = ((c[0] - eye[0]).powi(2) + (c[1] - eye[1]).powi(2) + (c[2] - eye[2]).powi(2)).sqrt();
                if d > range {
                    continue;
                }
                let state = game.world.block(pos[0], pos[1], pos[2]);
                if game.res.facts().is_air(state) || game.res.facts().get(state).hardness < 0.0 {
                    continue;
                }
                targets.push((pos, d));
            }
        }
    }
    targets.sort_by(|a, b| a.1.total_cmp(&b.1));
    for (pos, _) in targets.into_iter().take(speed) {
        let seq = game.horizonx_next_sequence();
        game.send(S::PlayerAction(play::PlayerAction { action: 0, pos, direction: 1, sequence: seq }));
        let _ = game.world.set(pos[0], pos[1], pos[2], air);
    }
}

fn scaffold(game: &mut Game<'_>) {
    let Some(p) = game.player.as_ref() else { return };
    let (px, py, pz) = (p.position.x, p.position.y, p.position.z);
    let placeable = game
        .held()
        .and_then(|s| game.items().get(&s.item))
        .is_some_and(|i| i.block.is_some());
    if !placeable {
        return;
    }
    let feet = [px.floor() as i32, py.floor() as i32 - 1, pz.floor() as i32];
    if !game.res.facts().is_air(game.world.block(feet[0], feet[1], feet[2])) {
        return;
    }
    let support = [feet[0], feet[1] - 1, feet[2]];
    if game.res.facts().is_air(game.world.block(support[0], support[1], support[2])) {
        return;
    }
    let seq = game.horizonx_next_sequence();
    game.send(S::UseItemOn(play::UseItemOn {
        hand: 0,
        pos: support,
        face: 1,
        cursor: [0.5, 1.0, 0.5],
        inside: false,
        world_border: false,
        sequence: seq,
    }));
    game.swing_arm();
}
