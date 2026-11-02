// O NoSlow do Meteor: este crate (horizon-client-input) não depende de nenhum outro, então o
// interruptor mora aqui mesmo.
use std::sync::atomic::{AtomicBool, Ordering};

static ON: AtomicBool = AtomicBool::new(false);

pub fn on() -> bool {
    ON.load(Ordering::Relaxed)
}

pub fn set(on: bool) {
    ON.store(on, Ordering::Relaxed);
}
