// Timer: o relógio do cliente deixa de ser preso a 20 ticks por segundo enquanto o Timer está
// ligado; só o relógio da janela muda (os do servidor nunca são de cliente).
fn horizonx_timer(step: &mut FixedStep) {
    let shared = &horizon_world::horizonx_shared::timer_clock;
    let me = step as *const FixedStep as usize;
    if horizon_world::horizonx_shared::timer() {
        if step.client {
            step.client = false;
            horizon_world::horizonx_shared::set_timer_clock(me);
        }
    } else if shared() == me {
        step.client = true;
        horizon_world::horizonx_shared::set_timer_clock(0);
    }
}
