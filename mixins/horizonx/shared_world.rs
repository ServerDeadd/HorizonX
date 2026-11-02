// O que o Meteor precisa enxergar de dentro de outros crates do jogo. Fica no horizon-world
// porque o mesher (horizon-client-render), a regra de faces (horizon-world), o relógio e a
// conta (horizon-client e horizon-client-render) dependem dele.
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

static XRAY: AtomicBool = AtomicBool::new(false);
static TIMER: AtomicBool = AtomicBool::new(false);
static NICK: AtomicBool = AtomicBool::new(false);
/// Por id de estado de bloco: se o Xray mostra o bloco.
static KEEP: RwLock<Vec<bool>> = RwLock::new(Vec::new());

pub fn xray() -> bool {
    XRAY.load(Ordering::Relaxed)
}

pub fn set_xray(on: bool) {
    XRAY.store(on, Ordering::Relaxed);
}

/// Timer ligado: o relógio do cliente pode ir mais rápido que 20 ticks por segundo.
pub fn timer() -> bool {
    TIMER.load(Ordering::Relaxed)
}

pub fn set_timer(on: bool) {
    TIMER.store(on, Ordering::Relaxed);
}

/// O relógio que o Timer destravou (endereço), para travar de novo quando desligar.
static TIMER_CLOCK: AtomicUsize = AtomicUsize::new(0);

pub fn timer_clock() -> usize {
    TIMER_CLOCK.load(Ordering::Relaxed)
}

pub fn set_timer_clock(clock: usize) {
    TIMER_CLOCK.store(clock, Ordering::Relaxed);
}

/// .nick ligado: o jogo se comporta como sem conta e entra com o nick.
pub fn nick() -> bool {
    NICK.load(Ordering::Relaxed)
}

pub fn set_nick(on: bool) {
    NICK.store(on, Ordering::Relaxed);
}

pub fn set_keep(keep: Vec<bool>) {
    if let Ok(mut k) = KEEP.write() {
        *k = keep;
    }
}

/// Estado desconhecido aparece, para nunca sumir com algo por engano.
pub fn keep(id: u32) -> bool {
    KEEP.read().map_or(true, |k| k.get(id as usize).copied().unwrap_or(true))
}

/// Os blocos que o Xray mostra: minérios, detritos antigos, ametista e tudo que tem
/// bloco-entidade (baús, fornalhas, spawners).
pub fn is_kept(name: &str, block_entity: bool) -> bool {
    block_entity
        || name.ends_with("_ore")
        || matches!(
            name,
            "minecraft:ancient_debris" | "minecraft:budding_amethyst" | "minecraft:amethyst_cluster"
        )
}
