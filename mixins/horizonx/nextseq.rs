// Uma sequência de pacote nova para o Nuker e o Scaffold. `next_sequence` é privado do módulo
// game::remote, então esta ponte (um submódulo dele) o expõe para o resto do mod.
impl crate::game::Game<'_> {
    pub(crate) fn horizonx_next_sequence(&mut self) -> i32 {
        self.next_sequence()
    }
}
