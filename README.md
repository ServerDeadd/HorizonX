# HorizonX 0.3

Client de utilidades para o [Horizon](https://github.com/horizon-rust/horizon) (o Minecraft 26.3
reescrito em Rust), como **mod nativo** em Rust. Baseado no [Meteor Client](https://meteorclient.com).

Não é oficial do Horizon nem do Meteor. Como usa código do Meteor (GPL-3.0), este mod também é
GPL-3.0 (veja `LICENSE`).

## Instalar ou atualizar

No PowerShell, com o launcher do Horizon na pasta Downloads:

```powershell
& "C:\Users\kaleb\Downloads\horizon-launcher.exe" native-build "C:\Users\kaleb\OneDrive\Desktop\Claude\MeteorHorizon\mixins\horizonx" --trust-native-code
```

Testado no Nightly `c9d474dd` (01/11/2026). Usa funções internas do jogo, então um Nightly bem mais
novo pode pedir ajuste.

## Usar

- **Shift direito** abre o menu (ClickGUI): uma coluna por categoria.
  - clique esquerdo liga e desliga;
  - clique direito abre os ajustes (no ajuste, esquerdo sobe e direito desce);
  - clique do meio no módulo, ou na linha "Tecla", e aperte a tecla. Esc cancela, Backspace tira.
- **Comandos no chat com ponto** (não vão para o servidor):

| Comando | O que faz |
| --- | --- |
| `.lista` | mostra os módulos |
| `.freecam` | liga ou desliga um módulo |
| `.killaura range 4` | muda um ajuste |
| `.bind freecam c` | põe a tecla C no Freecam (`.bind freecam` tira) |
| `.amigo Nome` | lista de amigos (o KillAura ignora, o ESP pinta de ciano) |
| `.seed 123456` | guarda a seed de um servidor (no singleplayer não precisa) |
| `.locate end_city` | acha a estrutura mais perto; se você já estiver nela, aponta a próxima |
| `.locate bioma cogumelo` | acha o bioma mais perto |
| `.nick Nome` | entra no próximo servidor com esse nick (`.nick` volta à conta) |

### `.locate` que pula o que você já visitou

Quando você está a menos de 100 blocos da estrutura que o `.locate` apontou, ele a guarda e aponta
a **próxima**, e assim por diante. Ele mostra quantas você já visitou. Então `.locate end_city`
repetido vai te levando de uma cidade do End para a seguinte, sem ficar preso na mesma.

## Módulos (41)

| Categoria | Módulos |
| --- | --- |
| Combate | KillAura, Criticals, TriggerBot, AutoClicker |
| Jogador | AutoRespawn, FastUse, AntiHunger, Reach, FastBreak, AntiAFK, **AutoTool**, **AutoTotem**, **AutoEat** |
| Movimento | Sprint, Flight, Speed, NoFall, Spider, HighJump, AirJump, Velocity, Jesus, Step, NoSlow, AutoWalk, Sneak |
| Visual | Fullbright, ESP, Tracers, NoRender, Xray, StorageESP, HUD, **Freecam**, **Nametags**, **Trajectories** |
| Mundo | Timer, StaffViewer, **Nuker**, **Scaffold** |

Novos nesta versão (em negrito):

- **Freecam:** solta a câmera do corpo. WASD/Espaço/Shift voam com ela e o mouse gira; o personagem
  fica parado e não manda pacotes de movimento. A distância de carregamento continua presa ao
  personagem, então muito longe você vê o mundo não carregado.
- **Nametags:** nome e vida dos jogadores flutuando acima da cabeça, através das paredes.
- **Trajectories:** mostra onde a flecha, bola de neve, pérola, poção ou tridente vai cair.
- **AutoTool:** troca para a melhor ferramenta antes de quebrar, e volta ao slot de antes.
- **AutoTotem:** mantém um totem na mão secundária.
- **AutoEat:** come sozinho quando a fome fica baixa.
- **Nuker:** quebra os blocos ao redor (instantâneo no criativo; ajuste de alcance e "só para baixo").
- **Scaffold:** coloca bloco embaixo de você para não cair.

## Aviso

Isto é um client de hack. Em servidor normal o anticheat pega Fly, Speed, KillAura, Nuker, Reach e
cia, e você leva ban. Use no seu mundo, em LAN ou em servidor anárquico.

## Não tem (ainda)

- Seed cracker (descobrir a seed de um servidor só pelas estruturas, tipo SeedCrackerX): projeto à
  parte, com matemática de reticulado.
- Vários outros módulos do Meteor (AutoAnvil, Freecam de morte, Baritone, etc.).

## Para quem for mexer

- `mixins/horizonx/runtime.rs`: módulos, ajustes, menu, comandos, teclas, ClickGUI, `.locate` e as
  contas puras (mira, movimento, trajetória, pular estruturas). Tem os testes (`cargo test`).
- `mixins/horizonx/glue.rs`: a ponte com o jogo (jogador, entidades, câmera do Freecam, desenho,
  inventário, pacotes).
- `mixins/horizonx/staff.rs` (StaffViewer) e `nextseq.rs` ficam dentro de `game::remote`;
  `shared_world.rs` e `noslow_input.rs` são interruptores vistos por outros crates.
- Os outros `.rs` são os ganchos, listados em `horizon-mixin.toml`.
