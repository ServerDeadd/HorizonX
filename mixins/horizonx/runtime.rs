// Meteor para o Horizon: os módulos do Meteor Client (meteorclient.com, GPL-3.0) refeitos em
// Rust como um mod nativo. Este arquivo é a lógica pura do mod: módulos, ajustes, menu,
// comandos, HUD e as decisões de cada módulo. Ele não toca no jogo; os arquivos de gancho
// (key.rs, tick.rs, hud.rs...) leem o jogo, chamam estas funções e aplicam o resultado.
// É ao mesmo tempo a biblioteca do Cargo (testes) e o módulo que o pacote põe no jogo.
use std::cell::RefCell;

pub const ID: &str = "horizonx";
pub const NAME: &str = "HorizonX";
pub const VERSION: &str = "0.3.1";
pub const DESCRIPTION: &str =
    "HorizonX: KillAura, Freecam, Flight, Xray, ESP e muito mais. Shift direito abre o menu, ponto para comandos. Baseado no Meteor Client.";
/// A tecla que abre o menu de módulos.
pub const MENU_KEY: &str = "menu";
pub const MENU_DEFAULT: &str = "key.keyboard.right.shift";
/// O roxo do Meteor (RGB).
pub const ACCENT: u32 = 0x91_3DE2;

// ----- Módulos e ajustes -------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Combat,
    Player,
    Movement,
    Render,
    World,
}

impl Category {
    pub const ALL: [Category; 5] = [
        Category::Combat,
        Category::Player,
        Category::Movement,
        Category::Render,
        Category::World,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Category::Combat => "Combate",
            Category::Player => "Jogador",
            Category::Movement => "Movimento",
            Category::Render => "Visual",
            Category::World => "Mundo",
        }
    }

    /// Cor da categoria na lista de módulos ativos (RGB).
    pub fn color(self) -> u32 {
        match self {
            Category::Combat => 0xFF_5555,
            Category::Player => 0x55_AAFF,
            Category::Movement => 0x55_FF99,
            Category::Render => 0xC8_7BFF,
            Category::World => 0xFF_C855,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Bool(bool),
    Num { default: f64, min: f64, max: f64, step: f64 },
    Pick { default: usize, options: &'static [&'static str] },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spec {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: Kind,
}

const fn flag(key: &'static str, label: &'static str, default: bool) -> Spec {
    Spec { key, label, kind: Kind::Bool(default) }
}

const fn num(key: &'static str, label: &'static str, default: f64, min: f64, max: f64, step: f64) -> Spec {
    Spec { key, label, kind: Kind::Num { default, min, max, step } }
}

const fn pick(key: &'static str, label: &'static str, default: usize, options: &'static [&'static str]) -> Spec {
    Spec { key, label, kind: Kind::Pick { default, options } }
}

#[derive(Clone, Copy, Debug)]
pub struct ModuleSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub category: Category,
    pub description: &'static str,
    pub settings: &'static [Spec],
}

/// Alvos de KillAura, ESP e Tracers.
pub const TARGETS: &[&str] = &["Jogadores", "Hostis", "Jogadores e hostis", "Todos os vivos"];

pub const MODULES: &[ModuleSpec] = &[
    // Combate
    ModuleSpec {
        id: "killaura",
        name: "KillAura",
        category: Category::Combat,
        description: "Ataca sozinho as entidades perto de você.",
        settings: &[
            pick("targets", "Alvos", 2, TARGETS),
            num("range", "Alcance", 4.5, 1.0, 6.0, 0.5),
            pick("priority", "Prioridade", 0, &["Mais perto", "Menos vida", "Mais no centro"]),
            flag("cooldown", "Esperar a recarga", true),
            flag("rotate", "Virar para o alvo", true),
            flag("only_click", "Só segurando o clique", false),
            flag("ignore_friends", "Ignorar nomes da lista de amigos", true),
        ],
    },
    ModuleSpec {
        id: "criticals",
        name: "Criticals",
        category: Category::Combat,
        description: "Faz cada golpe virar crítico com um pulinho antes do ataque.",
        settings: &[pick("mode", "Modo", 0, &["Pulinho", "Pacote"])],
    },
    ModuleSpec {
        id: "triggerbot",
        name: "TriggerBot",
        category: Category::Combat,
        description: "Ataca quem estiver na sua mira quando a recarga terminar.",
        settings: &[pick("targets", "Alvos", 3, TARGETS)],
    },
    ModuleSpec {
        id: "autoclicker",
        name: "AutoClicker",
        category: Category::Combat,
        description: "Clica sozinho enquanto você segura o botão.",
        settings: &[num("cps", "Cliques por segundo", 10.0, 1.0, 20.0, 1.0)],
    },
    // Jogador
    ModuleSpec {
        id: "autorespawn",
        name: "AutoRespawn",
        category: Category::Player,
        description: "Renasce sozinho quando você morre.",
        settings: &[],
    },
    ModuleSpec {
        id: "fastuse",
        name: "FastUse",
        category: Category::Player,
        description: "Tira a espera entre colocar blocos e usar itens.",
        settings: &[],
    },
    // Movimento
    ModuleSpec {
        id: "sprint",
        name: "Sprint",
        category: Category::Movement,
        description: "Corre sempre que estiver andando para frente.",
        settings: &[flag("omni", "Correr para qualquer lado", false)],
    },
    ModuleSpec {
        id: "flight",
        name: "Flight",
        category: Category::Movement,
        description: "Voa como no criativo.",
        settings: &[
            num("speed", "Velocidade", 1.0, 0.1, 5.0, 0.1),
            num("vertical", "Velocidade vertical", 0.5, 0.1, 3.0, 0.1),
            flag("antikick", "Anti-kick (desce um pouco)", true),
        ],
    },
    ModuleSpec {
        id: "speed",
        name: "Speed",
        category: Category::Movement,
        description: "Anda mais rápido no chão.",
        settings: &[
            num("speed", "Multiplicador", 1.6, 1.1, 4.0, 0.1),
            flag("autojump", "Pular sozinho (Bhop)", false),
        ],
    },
    ModuleSpec {
        id: "nofall",
        name: "NoFall",
        category: Category::Movement,
        description: "Não toma dano de queda.",
        settings: &[],
    },
    ModuleSpec {
        id: "spider",
        name: "Spider",
        category: Category::Movement,
        description: "Sobe paredes como aranha.",
        settings: &[num("speed", "Velocidade", 0.2, 0.05, 1.0, 0.05)],
    },
    ModuleSpec {
        id: "highjump",
        name: "HighJump",
        category: Category::Movement,
        description: "Pula mais alto.",
        settings: &[num("multiplier", "Multiplicador", 1.5, 1.1, 5.0, 0.1)],
    },
    ModuleSpec {
        id: "airjump",
        name: "AirJump",
        category: Category::Movement,
        description: "Pula de novo no ar.",
        settings: &[],
    },
    ModuleSpec {
        id: "fullbright",
        name: "Fullbright",
        category: Category::Render,
        description: "Tudo claro, até no escuro e nas cavernas.",
        settings: &[],
    },
    ModuleSpec {
        id: "esp",
        name: "ESP",
        category: Category::Render,
        description: "Mostra as entidades através das paredes.",
        settings: &[pick("targets", "Alvos", 3, TARGETS), pick("mode", "Modo", 0, &["Contorno", "Caixa"])],
    },
    ModuleSpec {
        id: "tracers",
        name: "Tracers",
        category: Category::Render,
        description: "Linhas do centro da tela até as entidades.",
        settings: &[pick("targets", "Alvos", 0, TARGETS)],
    },
    ModuleSpec {
        id: "norender",
        name: "NoRender",
        category: Category::Render,
        description: "Tira a neblina e os efeitos na tela: fogo, abóbora, água e portal.",
        settings: &[flag("fog", "Sem neblina", true), flag("overlays", "Sem efeitos na tela", true)],
    },
    // Movimento (novos)
    ModuleSpec {
        id: "velocity",
        name: "Velocity",
        category: Category::Movement,
        description: "Não é empurrado por golpes nem explosões.",
        settings: &[
            num("horizontal", "Empurrão horizontal (%)", 0.0, 0.0, 100.0, 10.0),
            num("vertical", "Empurrão vertical (%)", 0.0, 0.0, 100.0, 10.0),
        ],
    },
    ModuleSpec {
        id: "jesus",
        name: "Jesus",
        category: Category::Movement,
        description: "Anda em cima da água e da lava.",
        settings: &[],
    },
    ModuleSpec {
        id: "step",
        name: "Step",
        category: Category::Movement,
        description: "Sobe blocos inteiros sem pular.",
        settings: &[num("height", "Altura", 1.0, 1.0, 3.0, 0.5)],
    },
    ModuleSpec {
        id: "noslow",
        name: "NoSlow",
        category: Category::Movement,
        description: "Não fica lento comendo, usando arco ou escudo.",
        settings: &[],
    },
    ModuleSpec {
        id: "autowalk",
        name: "AutoWalk",
        category: Category::Movement,
        description: "Anda para frente sozinho.",
        settings: &[],
    },
    ModuleSpec {
        id: "sneak",
        name: "Sneak",
        category: Category::Movement,
        description: "Fica agachado o tempo todo.",
        settings: &[],
    },
    // Jogador (novos)
    ModuleSpec {
        id: "antihunger",
        name: "AntiHunger",
        category: Category::Player,
        description: "Gasta menos fome: o servidor acha que você não está correndo.",
        settings: &[],
    },
    ModuleSpec {
        id: "reach",
        name: "Reach",
        category: Category::Player,
        description: "Alcança entidades e blocos mais longe.",
        settings: &[num("range", "Alcance", 4.5, 3.0, 6.0, 0.5)],
    },
    ModuleSpec {
        id: "fastbreak",
        name: "FastBreak",
        category: Category::Player,
        description: "Sem espera entre quebrar um bloco e o próximo.",
        settings: &[],
    },
    ModuleSpec {
        id: "autotool",
        name: "AutoTool",
        category: Category::Player,
        description: "Troca para a melhor ferramenta antes de quebrar o bloco.",
        settings: &[flag("switchback", "Voltar ao slot de antes", true)],
    },
    ModuleSpec {
        id: "autototem",
        name: "AutoTotem",
        category: Category::Player,
        description: "Mantém um totem da imortalidade na mão secundária.",
        settings: &[],
    },
    ModuleSpec {
        id: "autoeat",
        name: "AutoEat",
        category: Category::Player,
        description: "Come sozinho quando a fome fica baixa.",
        settings: &[num("hunger", "Comer com fome até", 17.0, 1.0, 19.0, 1.0)],
    },
    ModuleSpec {
        id: "antiafk",
        name: "AntiAFK",
        category: Category::Player,
        description: "Pula e gira de vez em quando para não ser expulso por ficar parado.",
        settings: &[num("interval", "Intervalo (segundos)", 30.0, 5.0, 120.0, 5.0)],
    },
    // Visual (novos)
    ModuleSpec {
        id: "xray",
        name: "Xray",
        category: Category::Render,
        description: "Mostra só os minérios, baús e líquidos.",
        settings: &[],
    },
    ModuleSpec {
        id: "storageesp",
        name: "StorageESP",
        category: Category::Render,
        description: "Baús, barris, shulkers e fornalhas através das paredes.",
        settings: &[pick("which", "Mostrar", 0, &["Tudo", "Só baús e shulkers"])],
    },
    ModuleSpec {
        id: "hud",
        name: "HUD",
        category: Category::Render,
        description: "Informações na tela: módulos ligados, coordenadas, FPS e velocidade.",
        settings: &[
            flag("list", "Lista de módulos", true),
            flag("coords", "Coordenadas", true),
            flag("fps", "FPS", true),
            flag("speed", "Velocidade", false),
            flag("facing", "Direção", false),
        ],
    },
    ModuleSpec {
        id: "freecam",
        name: "Freecam",
        category: Category::Render,
        description: "Solta a câmera do corpo; você voa com ela e o personagem fica parado.",
        settings: &[num("speed", "Velocidade", 1.0, 0.2, 5.0, 0.2)],
    },
    ModuleSpec {
        id: "nametags",
        name: "Nametags",
        category: Category::Render,
        description: "Nome e vida dos jogadores flutuando acima da cabeça, através das paredes.",
        settings: &[flag("health", "Mostrar vida", true)],
    },
    ModuleSpec {
        id: "trajectories",
        name: "Trajectories",
        category: Category::Render,
        description: "Mostra onde a flecha, bola de neve, pérola ou tridente vai cair.",
        settings: &[],
    },
    // Mundo
    ModuleSpec {
        id: "timer",
        name: "Timer",
        category: Category::World,
        description: "Acelera o tempo do jogo para você.",
        settings: &[num("speed", "Velocidade", 1.5, 0.1, 5.0, 0.1)],
    },
    ModuleSpec {
        id: "nuker",
        name: "Nuker",
        category: Category::World,
        description: "Quebra os blocos ao redor (instantâneo no criativo). A velocidade é quantos blocos por tick: aumente para um speednuker.",
        settings: &[
            num("range", "Alcance", 4.5, 1.0, 6.0, 0.5),
            num("speed", "Blocos por tick", 128.0, 16.0, 1024.0, 16.0),
            flag("down", "Só para baixo", false),
        ],
    },
    ModuleSpec {
        id: "scaffold",
        name: "Scaffold",
        category: Category::World,
        description: "Coloca bloco embaixo de você para não cair.",
        settings: &[],
    },
    ModuleSpec {
        id: "staffviewer",
        name: "StaffViewer",
        category: Category::World,
        description: "Tabela de quem está conectado fora da tab, no espectador e perto olhando para você.",
        settings: &[flag("alert", "Avisar no chat", true)],
    },
];

pub fn index_of(id: &str) -> Option<usize> {
    MODULES.iter().position(|m| m.id == id)
}

// ----- Estado ------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    Bool(bool),
    Num(f64),
    Pick(usize),
}

impl Value {
    fn default_of(kind: Kind) -> Value {
        match kind {
            Kind::Bool(b) => Value::Bool(b),
            Kind::Num { default, .. } => Value::Num(default),
            Kind::Pick { default, .. } => Value::Pick(default),
        }
    }

    fn parse(kind: Kind, text: &str) -> Option<Value> {
        let text = text.trim();
        match kind {
            Kind::Bool(_) => match text {
                "true" | "ligado" | "on" | "sim" => Some(Value::Bool(true)),
                "false" | "desligado" | "off" | "nao" | "não" => Some(Value::Bool(false)),
                _ => None,
            },
            Kind::Num { min, max, .. } => text
                .replace(',', ".")
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .map(|v| Value::Num(v.clamp(min, max))),
            Kind::Pick { options, .. } => text
                .parse::<usize>()
                .ok()
                .filter(|&i| i < options.len())
                .or_else(|| {
                    let t = text.to_lowercase();
                    options.iter().position(|o| o.to_lowercase() == t)
                })
                .map(Value::Pick),
        }
    }

    fn stored(self) -> String {
        match self {
            Value::Bool(b) => b.to_string(),
            Value::Num(v) => format!("{v}"),
            Value::Pick(i) => i.to_string(),
        }
    }

    fn label(self, kind: Kind) -> String {
        match (self, kind) {
            (Value::Bool(b), _) => if b { "Ligado" } else { "Desligado" }.to_owned(),
            (Value::Num(v), Kind::Num { step, .. }) if step >= 1.0 => format!("{v:.0}"),
            (Value::Num(v), _) => format!("{v:.2}").trim_end_matches('0').trim_end_matches('.').replace('.', ","),
            (Value::Pick(i), Kind::Pick { options, .. }) => options.get(i).copied().unwrap_or("?").to_owned(),
            (Value::Pick(i), _) => i.to_string(),
        }
    }
}

/// Onde o menu está (Opções > Mods > Meteor).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Home,
    Category(usize),
    Module(usize),
}

#[derive(Default)]
struct State {
    loaded: bool,
    on: Vec<bool>,
    values: Vec<Vec<Value>>,
    friends: Vec<String>,
    /// A tecla de cada módulo (`key.keyboard.k`), vazia quando não tem.
    binds: Vec<String>,
    /// O ClickGUI esperando a tecla deste módulo.
    binding: Option<usize>,
    page: Page,
    /// Módulos com os ajustes abertos no ClickGUI.
    expanded: Vec<bool>,
    /// O que os módulos lembram entre um tick e outro.
    run: Run,
}

/// Memória dos módulos entre ticks.
#[derive(Default, Clone, Debug)]
pub struct Run {
    pub ticks: u64,
    /// Queda acumulada no último tick (NoFall).
    pub fall: f64,
    pub was_on_ground: bool,
    pub jump_was_down: bool,
    /// Teclas que o Sprint, o Speed e o Criticals seguram: soltas quando o módulo desliga.
    pub holding_sprint: bool,
    pub holding_jump: bool,
    /// Ticks até o próximo clique do AutoClicker.
    pub click_wait: f64,
    pub holding_forward: bool,
    pub holding_sneak: bool,
    /// Ticks desde o último movimento do AntiAFK.
    pub afk_ticks: u64,
    /// O Xray em uso e a tabela de malha que ele montou (para refazer se o jogo trocar).
    pub xray_applied: bool,
    pub xray_tables: usize,
    pub step_applied: bool,
    pub timer_applied: bool,
    /// O nome de antes do .nick, para voltar.
    pub name_before_nick: Option<Option<String>>,
    /// Quem o StaffViewer já avisou no chat.
    pub staff_seen: Vec<String>,
    /// Estado do Freecam: ligado, e a posição e o olhar da câmera solta.
    pub freecam_on: bool,
    pub freecam_pos: [f64; 3],
    pub freecam_yaw: f32,
    pub freecam_pitch: f32,
    /// AutoEat comendo agora e o slot de antes, para voltar.
    pub eating: bool,
    pub eat_prev: usize,
    /// AutoTool: slot de antes, para voltar quando terminar de quebrar.
    pub tool_prev: Option<usize>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn store_key(module: &str, key: &str) -> String {
    format!("{module}.{key}")
}

/// Lê os ajustes salvos uma vez (`get` lê um arquivo de ajuste do mod).
pub fn load(get: impl Fn(&str) -> Option<String>) {
    STATE.with_borrow_mut(|s| {
        if s.loaded {
            return;
        }
        s.loaded = true;
        // O HUD começa ligado; os outros, desligados.
        s.on = MODULES
            .iter()
            .map(|m| get(&store_key(m.id, "on")).map_or(DEFAULT_ON.contains(&m.id), |v| v.trim() == "true"))
            .collect();
        s.binds = MODULES
            .iter()
            .map(|m| get(&store_key(m.id, "bind")).unwrap_or_default().trim().to_owned())
            .collect();
        s.values = MODULES
            .iter()
            .map(|m| {
                m.settings
                    .iter()
                    .map(|spec| {
                        get(&store_key(m.id, spec.key))
                            .and_then(|t| Value::parse(spec.kind, &t))
                            .unwrap_or(Value::default_of(spec.kind))
                    })
                    .collect()
            })
            .collect();
        s.friends = get("friends")
            .map(|v| v.split(',').map(|n| n.trim().to_lowercase()).filter(|n| !n.is_empty()).collect())
            .unwrap_or_default();
    });
}

/// Para os testes: volta ao estado inicial.
#[allow(dead_code)]
pub fn reset() {
    STATE.with_borrow_mut(|s| *s = State::default());
}

thread_local! {
    static MOD_ON: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}

/// O mod ligado em Opções > Mods (a ponte atualiza a cada tick).
pub fn set_mod_on(on: bool) {
    MOD_ON.with(|m| m.set(on));
}

/// O módulo está ligado e o mod também.
pub fn active(id: &str) -> bool {
    MOD_ON.with(|m| m.get()) && is_on(id)
}

pub fn is_on(id: &str) -> bool {
    let Some(i) = index_of(id) else { return false };
    STATE.with_borrow(|s| s.on.get(i).copied().unwrap_or(false))
}

fn value(id: &str, key: &str) -> Option<Value> {
    let i = index_of(id)?;
    let j = MODULES[i].settings.iter().position(|s| s.key == key)?;
    STATE.with_borrow(|s| s.values.get(i).and_then(|v| v.get(j)).copied())
        .or_else(|| Some(Value::default_of(MODULES[i].settings[j].kind)))
}

pub fn get_flag(id: &str, key: &str) -> bool {
    matches!(value(id, key), Some(Value::Bool(true)))
}

pub fn get_num(id: &str, key: &str) -> f64 {
    match value(id, key) {
        Some(Value::Num(v)) => v,
        _ => 0.0,
    }
}

pub fn get_pick(id: &str, key: &str) -> usize {
    match value(id, key) {
        Some(Value::Pick(i)) => i,
        _ => 0,
    }
}

/// Módulos que já vêm ligados.
pub const DEFAULT_ON: &[&str] = &["hud"];

pub fn hud_shown() -> bool {
    is_on("hud") && get_flag("hud", "list")
}

pub fn is_friend(name: &str) -> bool {
    let name = name.to_lowercase();
    STATE.with_borrow(|s| s.friends.iter().any(|f| *f == name))
}

pub fn run<R>(f: impl FnOnce(&mut Run) -> R) -> R {
    STATE.with_borrow_mut(|s| f(&mut s.run))
}

/// Liga ou desliga; devolve o que guardar e a mensagem para o chat.
pub fn toggle(i: usize) -> (Vec<(String, String)>, String) {
    let on = STATE.with_borrow_mut(|s| {
        if s.on.len() != MODULES.len() {
            s.on = vec![false; MODULES.len()];
        }
        s.on[i] = !s.on[i];
        s.on[i]
    });
    let m = &MODULES[i];
    (
        vec![(store_key(m.id, "on"), on.to_string())],
        format!("{} {}", m.name, if on { "ligado" } else { "desligado" }),
    )
}

fn set_value(i: usize, j: usize, v: Value) -> (String, String) {
    STATE.with_borrow_mut(|s| {
        if s.values.len() != MODULES.len() {
            s.values = MODULES
                .iter()
                .map(|m| m.settings.iter().map(|spec| Value::default_of(spec.kind)).collect())
                .collect();
        }
        s.values[i][j] = v;
    });
    (store_key(MODULES[i].id, MODULES[i].settings[j].key), v.stored())
}

/// Próximo valor de um ajuste num clique do menu: liga/desliga, sobe um passo (volta ao mínimo
/// depois do máximo) ou passa para a próxima opção.
fn next(kind: Kind, v: Value) -> Value {
    match (kind, v) {
        (Kind::Bool(_), Value::Bool(b)) => Value::Bool(!b),
        (Kind::Num { min, max, step, .. }, Value::Num(x)) => {
            let n = ((x + step) / step).round() * step;
            Value::Num(if n > max + 1e-9 { min } else { n })
        }
        (Kind::Pick { options, .. }, Value::Pick(p)) => Value::Pick((p + 1) % options.len()),
        (kind, _) => Value::default_of(kind),
    }
}

// ----- Menu (Opções > Mods > Meteor) -------------------------------------------------------

pub fn active_count(c: Category) -> (usize, usize) {
    STATE.with_borrow(|s| {
        let all: Vec<usize> = MODULES.iter().enumerate().filter(|(_, m)| m.category == c).map(|(i, _)| i).collect();
        let on = all.iter().filter(|&&i| s.on.get(i).copied().unwrap_or(false)).count();
        (on, all.len())
    })
}

fn modules_in(c: Category) -> Vec<usize> {
    MODULES.iter().enumerate().filter(|(_, m)| m.category == c).map(|(i, _)| i).collect()
}

/// Os botões da página atual.
pub fn labels() -> Vec<String> {
    let page = STATE.with_borrow(|s| s.page);
    let on = |i: usize| if is_on(MODULES[i].id) { "Ligado" } else { "Desligado" };
    match page {
        Page::Home => {
            let out: Vec<String> = Category::ALL
                .iter()
                .map(|&c| {
                    let (a, n) = active_count(c);
                    format!("{}: {a} de {n} ligados", c.name())
                })
                .collect();
            out
        }
        Page::Category(c) => {
            let mut out = vec!["Voltar".to_owned()];
            for i in modules_in(Category::ALL[c]) {
                out.push(format!("{}: {}", MODULES[i].name, on(i)));
                if !MODULES[i].settings.is_empty() {
                    out.push(format!("Ajustes do {}", MODULES[i].name));
                }
            }
            out
        }
        Page::Module(i) => {
            let m = &MODULES[i];
            let mut out = vec!["Voltar".to_owned(), format!("{}: {}", m.name, on(i))];
            for spec in m.settings {
                let v = value(m.id, spec.key).unwrap_or(Value::default_of(spec.kind));
                out.push(format!("{}: {}", spec.label, v.label(spec.kind)));
            }
            out
        }
    }
}

/// Um clique no botão `b` da página atual; devolve o que guardar.
pub fn click(b: usize) -> Vec<(String, String)> {
    let page = STATE.with_borrow(|s| s.page);
    let go = |p: Page| STATE.with_borrow_mut(|s| s.page = p);
    match page {
        Page::Home => {
            if b < Category::ALL.len() {
                go(Page::Category(b));
            }
            vec![]
        }
        Page::Category(c) => {
            if b == 0 {
                go(Page::Home);
                return vec![];
            }
            // Os botões depois de "Voltar": o módulo e, se tiver ajustes, a página dele.
            let mut n = 1;
            for i in modules_in(Category::ALL[c]) {
                if b == n {
                    return toggle(i).0;
                }
                n += 1;
                if !MODULES[i].settings.is_empty() {
                    if b == n {
                        go(Page::Module(i));
                        return vec![];
                    }
                    n += 1;
                }
            }
            vec![]
        }
        Page::Module(i) => match b {
            0 => {
                go(Page::Category(Category::ALL.iter().position(|&c| c == MODULES[i].category).unwrap_or(0)));
                vec![]
            }
            1 => toggle(i).0,
            _ => {
                let j = b - 2;
                let Some(spec) = MODULES[i].settings.get(j) else { return vec![] };
                let current = value(MODULES[i].id, spec.key).unwrap_or(Value::default_of(spec.kind));
                vec![set_value(i, j, next(spec.kind, current))]
            }
        },
    }
}

#[allow(dead_code)]
pub fn page() -> Page {
    STATE.with_borrow(|s| s.page)
}

#[allow(dead_code)]
pub fn open_home() {
    STATE.with_borrow_mut(|s| s.page = Page::Home);
}

// ----- Comandos (.) ------------------------------------------------------------------------

/// O que só a ponte com o jogo sabe fazer (precisa do mundo ou da conexão).
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Action {
    /// `.locate <coisa>`
    Locate(String),
    /// `.seed <número>`; None mostra a seed atual.
    Seed(Option<String>),
    /// `.nick <nome>`; None volta ao nome da conta.
    Nick(Option<String>),
}

/// O resultado de um comando: o que guardar, as linhas do chat e o que a ponte faz.
#[derive(Default, Debug, PartialEq)]
pub struct Reply {
    pub store: Vec<(String, String)>,
    pub chat: Vec<String>,
    pub action: Option<Action>,
}

/// O prefixo dos comandos no chat, como no Meteor.
pub const PREFIX: char = '.';

fn find_module(name: &str) -> Option<usize> {
    let n = name.to_lowercase();
    MODULES.iter().position(|m| m.id == n || m.name.to_lowercase() == n)
}

/// `.lista`, `.<módulo>`, `.<módulo> <ajuste> <valor>`, `.bind`, `.amigo`, `.seed`, `.locate`,
/// `.nick`. `args` vem sem o ponto.
pub fn command(args: &str) -> Reply {
    let mut r = Reply::default();
    let parts: Vec<&str> = args.split_whitespace().collect();
    match parts.as_slice() {
        [] | ["ajuda"] => {
            r.chat.push(format!("{NAME} {VERSION}. Comandos:"));
            r.chat.push(".lista  .<módulo>  .<módulo> ajustes  .<módulo> <ajuste> <valor>".into());
            r.chat.push(".bind <módulo> <tecla>  .amigo <nome>  .nick <nome>".into());
            r.chat.push(".seed <número>  .locate <vila, fortaleza, cidade antiga, ...>".into());
            r.chat.push("O menu abre no Shift direito.".into());
        }
        ["lista"] => {
            for c in Category::ALL {
                let names: Vec<String> = modules_in(c)
                    .into_iter()
                    .map(|i| {
                        let m = &MODULES[i];
                        if is_on(m.id) { format!("{} (ligado)", m.name) } else { m.name.to_owned() }
                    })
                    .collect();
                r.chat.push(format!("{}: {}", c.name(), names.join(", ")));
            }
        }
        ["amigo", name] => {
            let name = name.to_lowercase();
            let list = STATE.with_borrow_mut(|s| {
                if let Some(p) = s.friends.iter().position(|f| *f == name) {
                    s.friends.remove(p);
                    r.chat.push(format!("{name} saiu da lista de amigos"));
                } else {
                    s.friends.push(name.clone());
                    r.chat.push(format!("{name} entrou na lista de amigos"));
                }
                s.friends.join(",")
            });
            r.store.push(("friends".into(), list));
        }
        ["bind", name, rest @ ..] => match find_module(name) {
            Some(i) => {
                let key = if rest.is_empty() { String::new() } else { parse_key(&rest.join(" ")) };
                r.store.push(set_bind(i, &key));
                r.chat.push(if key.is_empty() {
                    format!("{} ficou sem tecla", MODULES[i].name)
                } else {
                    format!("{} agora liga na tecla {}", MODULES[i].name, key_label(&key))
                });
            }
            None => r.chat.push(format!("Não existe módulo {name}")),
        },
        ["seed"] => r.action = Some(Action::Seed(None)),
        ["seed", n] => r.action = Some(Action::Seed(Some((*n).to_owned()))),
        ["locate"] => r.chat.push("Use .locate <coisa>: vila, fortaleza, cidade antiga, mansão, monumento, ...".into()),
        ["locate", rest @ ..] => r.action = Some(Action::Locate(rest.join(" "))),
        ["nick"] => r.action = Some(Action::Nick(None)),
        ["nick", name] => r.action = Some(Action::Nick(Some((*name).to_owned()))),
        [name] => match find_module(name) {
            Some(i) => {
                let (store, line) = toggle(i);
                r.store = store;
                r.chat.push(line);
            }
            None => r.chat.push(format!("Não existe módulo {name}. Veja .lista")),
        },
        [name, _] => match find_module(name) {
            Some(i) => {
                let m = &MODULES[i];
                for spec in m.settings {
                    let v = value(m.id, spec.key).unwrap_or(Value::default_of(spec.kind));
                    r.chat.push(format!("{}: {} ({})", spec.label, v.label(spec.kind), spec.key));
                }
                if m.settings.is_empty() {
                    r.chat.push(format!("{} não tem ajustes", m.name));
                }
            }
            None => r.chat.push(format!("Não existe módulo {name}")),
        },
        [name, key, value_text, ..] => {
            let Some(i) = find_module(name) else {
                r.chat.push(format!("Não existe módulo {name}"));
                return r;
            };
            let m = &MODULES[i];
            let Some(j) = m.settings.iter().position(|s| s.key == *key) else {
                r.chat.push(format!("{} não tem o ajuste {key}. Use .{} ajustes", m.name, m.id));
                return r;
            };
            match Value::parse(m.settings[j].kind, value_text) {
                Some(v) => {
                    r.store.push(set_value(i, j, v));
                    r.chat.push(format!("{}: {} agora é {}", m.name, m.settings[j].label, v.label(m.settings[j].kind)));
                }
                None => r.chat.push(format!("Valor inválido para {}", m.settings[j].label)),
            }
        }
    }
    r
}

// ----- Teclas ------------------------------------------------------------------------------

/// A tecla de cada módulo, no formato do jogo (`key.keyboard.k`); vazio é sem tecla.
pub fn bind_of(i: usize) -> String {
    STATE.with_borrow(|s| s.binds.get(i).cloned().unwrap_or_default())
}

pub fn set_bind(i: usize, key: &str) -> (String, String) {
    STATE.with_borrow_mut(|s| {
        if s.binds.len() != MODULES.len() {
            s.binds = vec![String::new(); MODULES.len()];
        }
        s.binds[i] = key.to_owned();
        s.binding = None;
    });
    (store_key(MODULES[i].id, "bind"), key.to_owned())
}

/// O ClickGUI está esperando uma tecla para este módulo.
pub fn binding() -> Option<usize> {
    STATE.with_borrow(|s| s.binding)
}

pub fn start_binding(i: usize) {
    STATE.with_borrow_mut(|s| s.binding = Some(i));
}

pub fn stop_binding() {
    STATE.with_borrow_mut(|s| s.binding = None);
}

/// Os módulos presos à tecla `key`.
pub fn modules_for_key(key: &str) -> Vec<usize> {
    STATE.with_borrow(|s| {
        s.binds.iter().enumerate().filter(|(_, b)| !b.is_empty() && *b == key).map(|(i, _)| i).collect()
    })
}

/// Teclas que nunca viram atalho: soltar a tecla do binding ou cancelar.
pub fn is_cancel_key(key: &str) -> bool {
    key == "key.keyboard.escape"
}

pub fn is_clear_key(key: &str) -> bool {
    matches!(key, "key.keyboard.backspace" | "key.keyboard.delete")
}

/// "key.keyboard.right.shift" -> "Shift direito", "key.keyboard.k" -> "K".
pub fn key_label(key: &str) -> String {
    let short = key
        .strip_prefix("key.keyboard.")
        .or_else(|| key.strip_prefix("key.mouse."))
        .unwrap_or(key);
    let named = match short {
        "left.shift" => "Shift esquerdo",
        "right.shift" => "Shift direito",
        "left.control" => "Ctrl esquerdo",
        "right.control" => "Ctrl direito",
        "left.alt" => "Alt esquerdo",
        "right.alt" => "Alt direito",
        "space" => "Espaço",
        "tab" => "Tab",
        "caps.lock" => "Caps Lock",
        "enter" => "Enter",
        "left" => "Botão esquerdo",
        "right" => "Botão direito",
        "middle" => "Botão do meio",
        "unknown" | "" => "Nenhuma",
        _ => "",
    };
    if !named.is_empty() {
        return named.to_owned();
    }
    if let Some(n) = short.strip_prefix("keypad.") {
        return format!("Num {}", n.to_uppercase());
    }
    short.replace('.', " ").to_uppercase()
}

/// O que a pessoa digitou em `.bind` -> nome da tecla no jogo.
pub fn parse_key(text: &str) -> String {
    let t = text.trim().to_lowercase();
    let name = match t.as_str() {
        "shift" | "shift esquerdo" => "left.shift",
        "shift direito" | "rshift" => "right.shift",
        "ctrl" | "control" => "left.control",
        "rctrl" => "right.control",
        "alt" => "left.alt",
        "espaco" | "espaço" | "space" => "space",
        "capslock" | "caps" => "caps.lock",
        "nenhuma" | "nada" | "none" => return String::new(),
        other => other,
    };
    if name.starts_with("key.") {
        name.to_owned()
    } else if name.starts_with("mouse") {
        format!("key.mouse.{}", name.trim_start_matches("mouse").trim())
    } else {
        format!("key.keyboard.{}", name.replace(' ', "."))
    }
}

// ----- HUD ---------------------------------------------------------------------------------

/// Os módulos ligados, do nome mais largo para o mais estreito (como o Meteor desenha), com a
/// cor da categoria.
pub fn active_lines(width: impl Fn(&str) -> f32) -> Vec<(String, u32)> {
    let mut lines: Vec<(String, u32)> = MODULES
        .iter()
        .filter(|m| is_on(m.id))
        .map(|m| (m.name.to_owned(), m.category.color()))
        .collect();
    lines.sort_by(|a, b| width(&b.0).total_cmp(&width(&a.0)).then(a.0.cmp(&b.0)));
    lines
}

/// As linhas de informação do HUD (canto de cima à esquerda), conforme os ajustes.
pub fn info_lines(pos: [f64; 3], nether: bool, fps: u32, speed_bps: f64, yaw: f32) -> Vec<String> {
    let mut out = Vec::new();
    if !is_on("hud") {
        return out;
    }
    if get_flag("hud", "coords") {
        let b = pos.map(|v| v.floor() as i64);
        out.push(format!("XYZ {} {} {}", b[0], b[1], b[2]));
        // No Nether cada bloco vale 8 no mundo normal, e o contrário.
        let (f, label) = if nether { (8.0, "Mundo normal") } else { (0.125, "Nether") };
        out.push(format!("{label} {} {}", (pos[0] * f).floor() as i64, (pos[2] * f).floor() as i64));
    }
    if get_flag("hud", "fps") {
        out.push(format!("FPS {fps}"));
    }
    if get_flag("hud", "speed") {
        out.push(format!("Velocidade {:.1} blocos/s", speed_bps).replace('.', ","));
    }
    if get_flag("hud", "facing") {
        out.push(format!("Direção {}", facing(yaw)));
    }
    out
}

/// Para onde o yaw aponta (0 é sul, 90 oeste, como no Minecraft).
pub fn facing(yaw: f32) -> &'static str {
    match ((wrap_degrees(yaw) + 360.0 + 45.0) % 360.0 / 90.0) as u32 {
        0 => "Sul",
        1 => "Oeste",
        2 => "Norte",
        _ => "Leste",
    }
}

// ----- Seed e .locate ----------------------------------------------------------------------

/// O que procurar: estruturas por id, uma tag de estruturas ou biomas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Query {
    Structures(Vec<String>),
    Tag(String),
    Biomes(Vec<String>),
}

fn plain(text: &str) -> String {
    text.trim()
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ã' | 'â' => 'a',
            'é' | 'ê' => 'e',
            'í' => 'i',
            'ó' | 'õ' | 'ô' => 'o',
            'ú' => 'u',
            'ç' => 'c',
            '_' | '-' => ' ',
            c => c,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn ids(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| format!("minecraft:{s}")).collect()
}

/// Nomes em português (e em inglês) das estruturas que o `.locate` acha.
const STRUCTURES: &[(&[&str], &str, &[&str])] = &[
    (&["vila", "vilarejo", "village"], "Vila", &["#village"]),
    (&["fortaleza", "stronghold", "portal do end"], "Fortaleza", &["stronghold"]),
    (&["fortaleza do nether", "fortaleza nether", "fortress", "nether fortress"], "Fortaleza do Nether", &["fortress"]),
    (&["bastiao", "bastion"], "Bastião", &["bastion_remnant"]),
    (&["cidade antiga", "ancient city"], "Cidade antiga", &["ancient_city"]),
    (&["cidade do end", "end city"], "Cidade do End", &["end_city"]),
    (&["mansao", "mansion"], "Mansão", &["mansion"]),
    (&["monumento", "monument"], "Monumento oceânico", &["monument"]),
    (&["templo do deserto", "piramide", "desert pyramid"], "Templo do deserto", &["desert_pyramid"]),
    (&["templo da selva", "jungle temple"], "Templo da selva", &["jungle_pyramid"]),
    (&["iglu", "igloo"], "Iglu", &["igloo"]),
    (&["cabana da bruxa", "bruxa", "swamp hut"], "Cabana da bruxa", &["swamp_hut"]),
    (&["posto avancado", "outpost", "saqueadores"], "Posto avançado", &["pillager_outpost"]),
    (&["naufragio", "navio", "shipwreck"], "Naufrágio", &["shipwreck", "shipwreck_beached"]),
    (&["ruina oceanica", "ruinas oceanicas", "ocean ruin"], "Ruína oceânica", &["ocean_ruin_cold", "ocean_ruin_warm"]),
    (&["portal em ruinas", "portal", "ruined portal"], "Portal em ruínas", &["#ruined_portal"]),
    (&["camara de teste", "camaras de teste", "trial", "trial chambers"], "Câmaras de teste", &["trial_chambers"]),
    (&["ruinas da trilha", "trilha", "trail ruins"], "Ruínas da trilha", &["trail_ruins"]),
    (&["mina", "mina abandonada", "mineshaft"], "Mina abandonada", &["#mineshaft"]),
    (&["tesouro", "tesouro enterrado", "buried treasure"], "Tesouro enterrado", &["buried_treasure"]),
];

/// Biomas pelos nomes em português; o resto vai pelo id do jogo (`.locate bioma jungle`).
const BIOMES: &[(&[&str], &str, &[&str])] = &[
    (&["cogumelo", "ilha de cogumelo", "mushroom"], "Campos de cogumelo", &["mushroom_fields"]),
    (&["cerejeira", "bosque de cerejeiras", "cherry"], "Bosque de cerejeiras", &["cherry_grove"]),
    (&["deserto", "desert"], "Deserto", &["desert"]),
    (&["selva", "jungle"], "Selva", &["jungle", "bamboo_jungle", "sparse_jungle"]),
    (&["mesa", "badlands"], "Badlands", &["badlands", "eroded_badlands", "wooded_badlands"]),
    (&["espinhos de gelo", "gelo", "ice spikes"], "Espinhos de gelo", &["ice_spikes"]),
    (&["pantano", "swamp"], "Pântano", &["swamp"]),
    (&["mangue", "mangrove"], "Manguezal", &["mangrove_swamp"]),
    (&["jardim palido", "pale garden"], "Jardim pálido", &["pale_garden"]),
    (&["planicie", "plains"], "Planície", &["plains", "sunflower_plains"]),
    (&["floresta", "forest"], "Floresta", &["forest"]),
    (&["savana", "savanna"], "Savana", &["savanna", "savanna_plateau"]),
    (&["taiga"], "Taiga", &["taiga", "old_growth_pine_taiga", "old_growth_spruce_taiga"]),
    (&["oceano", "ocean"], "Oceano", &["ocean", "deep_ocean"]),
    (&["caverna exuberante", "lush caves"], "Caverna exuberante", &["lush_caves"]),
    (&["escuridao profunda", "deep dark"], "Escuridão profunda", &["deep_dark"]),
];

fn expand(list: &[&str]) -> Query {
    match list {
        [tag] if tag.starts_with('#') => Query::Tag(format!("minecraft:{}", &tag[1..])),
        _ => Query::Structures(ids(list)),
    }
}

/// `.locate <texto>` -> o que procurar e o nome que aparece no chat.
pub fn locate_query(text: &str) -> (Query, String) {
    let t = plain(text);
    if let Some(rest) = t.strip_prefix("bioma ").or_else(|| t.strip_prefix("biome ")) {
        if let Some((_, label, list)) = BIOMES.iter().find(|(names, _, _)| names.contains(&rest)) {
            return (Query::Biomes(ids(list)), (*label).to_owned());
        }
        let id = rest.replace(' ', "_");
        let id = if id.contains(':') { id } else { format!("minecraft:{id}") };
        return (Query::Biomes(vec![id.clone()]), id);
    }
    if let Some((_, label, list)) = STRUCTURES.iter().find(|(names, _, _)| names.contains(&t.as_str())) {
        return (expand(list), (*label).to_owned());
    }
    if let Some((_, label, list)) = BIOMES.iter().find(|(names, _, _)| names.contains(&t.as_str())) {
        return (Query::Biomes(ids(list)), (*label).to_owned());
    }
    let id = t.replace(' ', "_");
    let id = if id.contains(':') { id } else { format!("minecraft:{id}") };
    (Query::Structures(vec![id.clone()]), id)
}

/// As coordenadas já visitadas de uma estrutura, no formato "x,z;x,z".
pub fn visited_parse(text: &str) -> Vec<[i32; 2]> {
    text.split(';')
        .filter_map(|p| {
            let (x, z) = p.split_once(',')?;
            Some([x.trim().parse().ok()?, z.trim().parse().ok()?])
        })
        .collect()
}

pub fn visited_text(list: &[[i32; 2]]) -> String {
    list.iter().map(|c| format!("{},{}", c[0], c[1])).collect::<Vec<_>>().join(";")
}

/// Já passei perto dessa (a menos de `tol` blocos de uma guardada)?
pub fn is_visited(list: &[[i32; 2]], at: [i32; 3], tol: i32) -> bool {
    list.iter().any(|v| (v[0] - at[0]).abs() <= tol && (v[1] - at[2]).abs() <= tol)
}

/// Distância horizontal entre um ponto do mundo e uma coordenada [x,z].
pub fn flat_dist(from: [i32; 3], at: [i32; 3]) -> f64 {
    let (dx, dz) = (f64::from(at[0] - from[0]), f64::from(at[2] - from[2]));
    (dx * dx + dz * dz).sqrt()
}

/// Para procurar a PRÓXIMA estrutura: empurra a origem da busca para além de `at`, no
/// sentido que se afasta de `from`.
pub fn next_origin(from: [i32; 3], at: [i32; 3], push: f64) -> [i32; 3] {
    let (dx, dz) = (f64::from(at[0] - from[0]), f64::from(at[2] - from[2]));
    let len = (dx * dx + dz * dz).sqrt();
    let (ux, uz) = if len < 1.0 { (1.0, 0.0) } else { (dx / len, dz / len) };
    [at[0] + (ux * push) as i32, from[1], at[2] + (uz * push) as i32]
}

/// A seed como o Minecraft lê: número, ou `String.hashCode` do Java para texto.
pub fn parse_seed(text: &str) -> i64 {
    let t = text.trim();
    t.parse::<i64>().unwrap_or_else(|_| {
        let mut h: i32 = 0;
        for c in t.encode_utf16() {
            h = h.wrapping_mul(31).wrapping_add(i32::from(c));
        }
        i64::from(h)
    })
}

/// A seed digitada em `.seed` (para servidores), se houver.
pub fn stored_seed(get: impl Fn(&str) -> Option<String>) -> Option<i64> {
    get("seed").and_then(|s| s.trim().parse().ok())
}

/// "Vila mais perto: X 120 Z -340, a 362 blocos"
pub fn found_line(label: &str, at: [i32; 3], from: [i32; 3]) -> String {
    let (dx, dz) = (f64::from(at[0] - from[0]), f64::from(at[2] - from[2]));
    let d = (dx * dx + dz * dz).sqrt().floor() as i64;
    format!("{label} mais perto: X {} Z {}, a {d} blocos", at[0], at[2])
}

/// Uma linha do StaffViewer: nome, situação, distância e cor (vermelho olhando para você,
/// amarelo perto, cinza longe).
pub fn staff_line(name: &str, hidden: bool, spectator: bool, near: Option<(f64, bool)>) -> (String, String, String, u32) {
    let state = match (hidden, spectator) {
        (true, true) => "fora da tab e espectador",
        (true, false) => "fora da tab",
        (false, true) => "espectador",
        _ => "",
    };
    let (distance, color) = match near {
        Some((d, true)) => (format!("{d:.0} blocos, olhando para você"), 0xFF_5555),
        Some((d, false)) => (format!("{d:.0} blocos"), 0xFF_D24A),
        None => ("longe".to_owned(), 0xB8_B8C4),
    };
    (name.to_owned(), state.to_owned(), distance, color)
}

/// Os nomes novos desde o último aviso (e guarda a lista atual).
pub fn staff_new(names: &[String]) -> Vec<String> {
    STATE.with_borrow_mut(|s| {
        let fresh: Vec<String> = names.iter().filter(|n| !s.run.staff_seen.contains(n)).cloned().collect();
        s.run.staff_seen = names.to_vec();
        fresh
    })
}

// ----- Alvos, mira e movimento -------------------------------------------------------------

/// Uma entidade viva como os módulos de combate e visual a enxergam.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Target {
    pub id: u32,
    /// Pés (centro da base da caixa).
    pub pos: [f64; 3],
    pub width: f64,
    pub height: f64,
    pub health: f32,
    pub player: bool,
    pub hostile: bool,
    pub friend: bool,
}

impl Target {
    pub fn min(&self) -> [f64; 3] {
        [self.pos[0] - self.width / 2.0, self.pos[1], self.pos[2] - self.width / 2.0]
    }
    pub fn max(&self) -> [f64; 3] {
        [self.pos[0] + self.width / 2.0, self.pos[1] + self.height, self.pos[2] + self.width / 2.0]
    }
    pub fn center(&self) -> [f64; 3] {
        [self.pos[0], self.pos[1] + self.height / 2.0, self.pos[2]]
    }
    /// Distância do olho até o ponto mais perto da caixa, como o alcance do ataque mede.
    pub fn reach_from(&self, eye: [f64; 3]) -> f64 {
        let (lo, hi) = (self.min(), self.max());
        let d: Vec<f64> = (0..3).map(|i| eye[i].clamp(lo[i], hi[i]) - eye[i]).collect();
        (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
    }
}

/// O filtro "Alvos": 0 jogadores, 1 hostis, 2 jogadores e hostis, 3 todos os vivos.
pub fn wanted(targets: usize, t: &Target) -> bool {
    match targets {
        0 => t.player,
        1 => t.hostile,
        2 => t.player || t.hostile,
        _ => true,
    }
}

/// Cor do ESP e dos Tracers (RGB): amigos em ciano, jogadores em branco, hostis em vermelho e
/// o resto em verde, como no Meteor.
pub fn esp_color(t: &Target) -> u32 {
    if t.player && t.friend {
        0x55_FFFF
    } else if t.player {
        0xFF_FFFF
    } else if t.hostile {
        0xFF_5555
    } else {
        0x55_FF55
    }
}

pub fn wrap_degrees(a: f32) -> f32 {
    let mut a = a % 360.0;
    if a >= 180.0 {
        a -= 360.0;
    }
    if a < -180.0 {
        a += 360.0;
    }
    a
}

/// Yaw e pitch (graus, como o Minecraft: yaw 0 olha para o sul) do olho até `point`.
pub fn look_at(eye: [f64; 3], point: [f64; 3]) -> (f32, f32) {
    let (dx, dy, dz) = (point[0] - eye[0], point[1] - eye[1], point[2] - eye[2]);
    let yaw = dz.atan2(dx).to_degrees() as f32 - 90.0;
    let pitch = -(dy.atan2((dx * dx + dz * dz).sqrt()).to_degrees() as f32);
    (wrap_degrees(yaw), pitch.clamp(-90.0, 90.0))
}

/// Para onde o olho aponta.
pub fn look_dir(yaw: f32, pitch: f32) -> [f64; 3] {
    let (y, p) = (f64::from(yaw).to_radians(), f64::from(pitch).to_radians());
    [-y.sin() * p.cos(), -p.sin(), y.cos() * p.cos()]
}

/// Quanto o olhar precisa girar para chegar no alvo.
pub fn angle_to(eye: [f64; 3], yaw: f32, pitch: f32, point: [f64; 3]) -> f32 {
    let (ty, tp) = look_at(eye, point);
    let dy = wrap_degrees(ty - yaw);
    let dp = tp - pitch;
    (dy * dy + dp * dp).sqrt()
}

/// Raio contra caixa (método das placas): a distância até entrar, se acertar.
pub fn ray_box(origin: [f64; 3], dir: [f64; 3], min: [f64; 3], max: [f64; 3]) -> Option<f64> {
    let (mut near, mut far) = (f64::NEG_INFINITY, f64::INFINITY);
    for i in 0..3 {
        if dir[i].abs() < 1e-12 {
            if origin[i] < min[i] || origin[i] > max[i] {
                return None;
            }
            continue;
        }
        let a = (min[i] - origin[i]) / dir[i];
        let b = (max[i] - origin[i]) / dir[i];
        near = near.max(a.min(b));
        far = far.min(a.max(b));
    }
    (near <= far && far >= 0.0).then_some(near.max(0.0))
}

/// A entidade na mira, até `reach` blocos.
pub fn under_crosshair(targets: &[Target], eye: [f64; 3], yaw: f32, pitch: f32, reach: f64) -> Option<Target> {
    let dir = look_dir(yaw, pitch);
    targets
        .iter()
        .filter_map(|t| ray_box(eye, dir, t.min(), t.max()).filter(|&d| d <= reach).map(|d| (d, *t)))
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, t)| t)
}

/// O alvo do KillAura: dentro do alcance, pela prioridade (0 mais perto, 1 menos vida, 2 mais
/// no centro da tela).
pub fn pick_target(
    targets: &[Target],
    eye: [f64; 3],
    yaw: f32,
    pitch: f32,
    range: f64,
    priority: usize,
) -> Option<Target> {
    let near: Vec<&Target> = targets.iter().filter(|t| t.reach_from(eye) <= range).collect();
    let key = |t: &&Target| -> f64 {
        match priority {
            1 => f64::from(t.health),
            2 => f64::from(angle_to(eye, yaw, pitch, t.center())),
            _ => t.reach_from(eye),
        }
    };
    near.into_iter().min_by(|a, b| key(a).total_cmp(&key(b))).copied()
}

/// Direção do movimento no chão (x, z) pelas teclas e pelo yaw; tamanho 1 ou 0.
pub fn move_dir(yaw: f32, forward: bool, back: bool, left: bool, right: bool) -> [f64; 2] {
    let f = f64::from(i8::from(forward) - i8::from(back));
    let s = f64::from(i8::from(left) - i8::from(right));
    if f == 0.0 && s == 0.0 {
        return [0.0, 0.0];
    }
    let r = f64::from(yaw).to_radians();
    let (x, z) = (s * r.cos() - f * r.sin(), f * r.cos() + s * r.sin());
    let len = (x * x + z * z).sqrt();
    [x / len, z / len]
}

/// Velocidade do Flight neste tick.
pub fn flight_velocity(dir: [f64; 2], up: bool, down: bool, ticks: u64) -> [f64; 3] {
    let speed = get_num("flight", "speed");
    let vertical = get_num("flight", "vertical");
    let mut vy = f64::from(i8::from(up) - i8::from(down)) * vertical;
    // Anti-kick: a cada 2 segundos desce um pouco, para o servidor não achar que flutua.
    if get_flag("flight", "antikick") && !up && ticks % 40 == 0 {
        vy = -0.04;
    }
    [dir[0] * speed, vy, dir[1] * speed]
}

/// Velocidade horizontal do Speed: a corrida normal (0,2873 por tick) vezes o multiplicador.
pub fn speed_horizontal(dir: [f64; 2]) -> [f64; 2] {
    let v = 0.2873 * get_num("speed", "speed");
    [dir[0] * v, dir[1] * v]
}

/// Para o NoFall: dizer ao servidor que está no chão quando a queda já passaria de 2 blocos.
pub fn nofall_spoof(on_ground: bool) -> bool {
    on_ground || (is_on("nofall") && STATE.with_borrow(|s| s.run.fall) > 2.0)
}

/// Normal de uma face -> o número que o pacote usa (baixo 0, cima 1, N 2, S 3, O 4, L 5).
#[allow(dead_code)]
pub fn face_dir(face: [i32; 3]) -> u8 {
    match face {
        [0, -1, 0] => 0,
        [0, 1, 0] => 1,
        [0, 0, -1] => 2,
        [0, 0, 1] => 3,
        [-1, 0, 0] => 4,
        [1, 0, 0] => 5,
        _ => 1,
    }
}

/// O índice da maior nota (a ferramenta mais rápida); None se a lista estiver vazia.
pub fn best_index(scores: &[f32]) -> Option<usize> {
    scores
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map(|(i, _)| i)
}

// ----- Freecam e trajetórias ---------------------------------------------------------------

/// Quanto a câmera do Freecam anda neste quadro: WASD relativo ao yaw, Espaço/Shift na vertical.
pub fn freecam_delta(
    yaw: f32,
    fwd: bool,
    back: bool,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    speed: f64,
) -> [f64; 3] {
    let h = move_dir(yaw, fwd, back, left, right);
    let vy = f64::from(i8::from(up) - i8::from(down));
    [h[0] * speed, vy * speed, h[1] * speed]
}

/// Velocidade inicial, gravidade e atrito de cada item que dá para arremessar.
pub fn projectile_params(item: &str) -> Option<(f64, f64, f64)> {
    let id = item.strip_prefix("minecraft:").unwrap_or(item);
    match id {
        "ender_pearl" | "snowball" | "egg" => Some((1.5, 0.03, 0.99)),
        "splash_potion" | "lingering_potion" => Some((0.5, 0.05, 0.99)),
        "bow" => Some((3.0, 0.05, 0.99)),
        "crossbow" => Some((3.15, 0.05, 0.99)),
        "trident" => Some((2.5, 0.05, 0.99)),
        "fishing_rod" => Some((0.6, 0.04, 0.92)),
        _ => None,
    }
}

/// A arco da trajetória: solta o projétil de `start` com `vel` e simula `steps` ticks.
pub fn arc(start: [f64; 3], mut vel: [f64; 3], gravity: f64, drag: f64, steps: usize) -> Vec<[f64; 3]> {
    let mut p = start;
    let mut out = Vec::with_capacity(steps + 1);
    out.push(start);
    for _ in 0..steps {
        vel[1] -= gravity;
        for i in 0..3 {
            vel[i] *= drag;
            p[i] += vel[i];
        }
        out.push(p);
    }
    out
}

// ----- ClickGUI ----------------------------------------------------------------------------

pub const GUI_HEADER: f32 = 18.0;
pub const GUI_ROW: f32 = 14.0;
pub const GUI_GAP: f32 = 8.0;
pub const GUI_TOP: f32 = 24.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuiItem {
    Header(usize),
    Module(usize),
    /// A linha "Tecla" de um módulo aberto.
    Bind(usize),
    Setting(usize, usize),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GuiRect {
    pub item: GuiItem,
    pub rect: [f32; 4],
}

thread_local! {
    static GUI_WIDTH: std::cell::Cell<f32> = const { std::cell::Cell::new(640.0) };
}

/// A largura da tela no último desenho do ClickGUI (o clique não recebe a tela).
pub fn set_gui_width(w: f32) {
    GUI_WIDTH.with(|g| g.set(w));
}

pub fn gui_width() -> f32 {
    GUI_WIDTH.with(|g| g.get())
}

pub fn expanded(i: usize) -> bool {
    STATE.with_borrow(|s| s.expanded.get(i).copied().unwrap_or(false))
}

/// Uma coluna por categoria, como o ClickGUI do Meteor; os ajustes abertos ficam embaixo do
/// módulo.
pub fn gui_layout(screen_w: f32) -> Vec<GuiRect> {
    let n = Category::ALL.len() as f32;
    let col = ((screen_w - 16.0 - GUI_GAP * (n - 1.0)) / n).min(124.0);
    let left = ((screen_w - (col * n + GUI_GAP * (n - 1.0))) / 2.0).max(4.0);
    let mut out = Vec::new();
    for (c, cat) in Category::ALL.iter().enumerate() {
        let x = left + c as f32 * (col + GUI_GAP);
        let mut y = GUI_TOP;
        out.push(GuiRect { item: GuiItem::Header(c), rect: [x, y, col, GUI_HEADER] });
        y += GUI_HEADER;
        for i in modules_in(*cat) {
            out.push(GuiRect { item: GuiItem::Module(i), rect: [x, y, col, GUI_ROW] });
            y += GUI_ROW;
            if expanded(i) {
                out.push(GuiRect { item: GuiItem::Bind(i), rect: [x, y, col, GUI_ROW] });
                y += GUI_ROW;
                for j in 0..MODULES[i].settings.len() {
                    out.push(GuiRect { item: GuiItem::Setting(i, j), rect: [x, y, col, GUI_ROW] });
                    y += GUI_ROW;
                }
            }
        }
    }
    out
}

pub fn gui_hit(layout: &[GuiRect], x: f32, y: f32) -> Option<GuiItem> {
    layout
        .iter()
        .find(|r| x >= r.rect[0] && x < r.rect[0] + r.rect[2] && y >= r.rect[1] && y < r.rect[1] + r.rect[3])
        .map(|r| r.item)
}

fn prev(kind: Kind, v: Value) -> Value {
    match (kind, v) {
        (Kind::Bool(_), Value::Bool(b)) => Value::Bool(!b),
        (Kind::Num { min, max, step, .. }, Value::Num(x)) => {
            let n = ((x - step) / step).round() * step;
            Value::Num(if n < min - 1e-9 { max } else { n })
        }
        (Kind::Pick { options, .. }, Value::Pick(p)) => Value::Pick((p + options.len() - 1) % options.len()),
        (kind, _) => Value::default_of(kind),
    }
}

/// Clique no ClickGUI: botão 0 é o esquerdo, 1 o direito, 2 o do meio. Devolve o que guardar.
pub fn gui_press(item: GuiItem, button: i32) -> Vec<(String, String)> {
    match item {
        GuiItem::Header(_) => vec![],
        GuiItem::Module(i) if button == 2 => {
            start_binding(i);
            vec![]
        }
        GuiItem::Module(i) if button == 1 => {
            STATE.with_borrow_mut(|s| {
                if s.expanded.len() != MODULES.len() {
                    s.expanded = vec![false; MODULES.len()];
                }
                s.expanded[i] = !s.expanded[i];
            });
            vec![]
        }
        GuiItem::Bind(i) if button == 1 => vec![set_bind(i, "")],
        GuiItem::Bind(i) => {
            start_binding(i);
            vec![]
        }
        GuiItem::Module(i) => toggle(i).0,
        GuiItem::Setting(i, j) => {
            let spec = MODULES[i].settings[j];
            let current = value(MODULES[i].id, spec.key).unwrap_or(Value::default_of(spec.kind));
            let v = if button == 1 { prev(spec.kind, current) } else { next(spec.kind, current) };
            vec![set_value(i, j, v)]
        }
    }
}

/// A linha "Tecla": a tecla do módulo, ou o aviso enquanto espera uma.
pub fn bind_text(i: usize) -> String {
    if binding() == Some(i) {
        "Aperte uma tecla".to_owned()
    } else {
        key_label(&bind_of(i))
    }
}

/// Rótulo e valor de uma linha de ajuste.
pub fn setting_text(i: usize, j: usize) -> (&'static str, String) {
    let spec = MODULES[i].settings[j];
    let v = value(MODULES[i].id, spec.key).unwrap_or(Value::default_of(spec.kind));
    let text = match (v, spec.kind) {
        (Value::Bool(b), _) => if b { "Sim" } else { "Não" }.to_owned(),
        _ => v.label(spec.kind),
    };
    (spec.label, text)
}

/// Corta `text` até caber em `max` pixels (sem reticências).
pub fn fit(text: &str, max: f32, width: impl Fn(&str) -> f32) -> String {
    let mut t = text.to_owned();
    while !t.is_empty() && width(&t) > max {
        t.pop();
    }
    t.trim_end().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn loads_defaults_and_saved_values() {
        reset();
        load(|k| match k {
            "killaura.on" => Some("true".into()),
            "killaura.range" => Some("3.5".into()),
            "flight.speed" => Some("99".into()),
            _ => None,
        });
        assert!(is_on("killaura"));
        assert!(!is_on("flight"));
        assert_eq!(get_num("killaura", "range"), 3.5);
        assert_eq!(get_num("flight", "speed"), 5.0, "cortado no máximo");
        assert!(get_flag("killaura", "cooldown"));
    }

    #[test]
    fn menu_navigates_and_toggles() {
        reset();
        load(none);
        open_home();
        assert_eq!(labels().len(), Category::ALL.len());
        click(0); // Combate
        assert_eq!(page(), Page::Category(0));
        let l = labels();
        assert_eq!(l[0], "Voltar");
        assert_eq!(l[1], "KillAura: Desligado");
        let stored = click(1);
        assert_eq!(stored, vec![("killaura.on".to_owned(), "true".to_owned())]);
        assert!(is_on("killaura"));
        click(2); // Ajustes do KillAura
        assert_eq!(page(), Page::Module(index_of("killaura").unwrap()));
        let l = labels();
        assert_eq!(l[2], "Alvos: Jogadores e hostis");
        assert_eq!(l[3], "Alcance: 4,5");
        click(3);
        assert_eq!(get_num("killaura", "range"), 5.0);
        click(3);
        click(3);
        assert_eq!(get_num("killaura", "range"), 6.0);
        click(3);
        assert_eq!(get_num("killaura", "range"), 1.0, "volta ao mínimo");
        click(0);
        assert_eq!(page(), Page::Category(0));
    }

    #[test]
    fn commands_toggle_and_set() {
        reset();
        load(none);
        assert!(is_on("hud"), "o HUD começa ligado");
        let r = command("flight");
        assert!(is_on("flight"));
        assert_eq!(r.chat, vec!["Flight ligado".to_owned()]);
        let r = command("flight speed 2,5");
        assert_eq!(get_num("flight", "speed"), 2.5);
        assert_eq!(r.store, vec![("flight.speed".to_owned(), "2.5".to_owned())]);
        let r = command("esp mode caixa");
        assert_eq!(get_pick("esp", "mode"), 1);
        assert!(r.chat[0].contains("Caixa"));
        assert!(command("nada").chat[0].contains("Não existe"));
        command("amigo Steve");
        assert!(is_friend("steve"));
        command("amigo steve");
        assert!(!is_friend("Steve"));
    }

    #[test]
    fn aim_math_matches_minecraft() {
        let (yaw, pitch) = look_at([0.0, 0.0, 0.0], [0.0, 0.0, 5.0]);
        assert!(yaw.abs() < 1e-3, "sul é yaw 0, deu {yaw}");
        assert!(pitch.abs() < 1e-3);
        let (yaw, _) = look_at([0.0, 0.0, 0.0], [-5.0, 0.0, 0.0]);
        assert!((yaw - 90.0).abs() < 1e-3, "oeste é yaw 90, deu {yaw}");
        let (_, pitch) = look_at([0.0, 0.0, 0.0], [0.0, 5.0, 0.0001]);
        assert!(pitch < -89.0, "olhar para cima é pitch negativo");
        let d = look_dir(0.0, 0.0);
        assert!((d[2] - 1.0).abs() < 1e-9);
        let m = move_dir(0.0, true, false, false, false);
        assert!((m[1] - 1.0).abs() < 1e-9 && m[0].abs() < 1e-9);
        let m = move_dir(90.0, true, false, false, false);
        assert!((m[0] + 1.0).abs() < 1e-9, "yaw 90 anda para oeste");
    }

    fn mob(id: u32, pos: [f64; 3], health: f32, player: bool) -> Target {
        Target { id, pos, width: 0.6, height: 1.8, health, player, hostile: !player, friend: false }
    }

    #[test]
    fn picks_targets_and_crosshair() {
        let eye = [0.0, 1.62, 0.0];
        let list = [
            mob(1, [0.0, 0.0, 3.0], 20.0, false),
            mob(2, [2.0, 0.0, 2.0], 5.0, true),
            mob(3, [0.0, 0.0, 9.0], 1.0, false),
        ];
        assert_eq!(pick_target(&list, eye, 0.0, 0.0, 4.5, 0).map(|t| t.id), Some(2));
        assert_eq!(pick_target(&list, eye, 0.0, 0.0, 4.5, 1).map(|t| t.id), Some(2));
        assert_eq!(pick_target(&list, eye, 0.0, 0.0, 4.5, 2).map(|t| t.id), Some(1), "o que está no meio da tela");
        assert_eq!(pick_target(&list, eye, 0.0, 0.0, 1.0, 0), None);
        assert_eq!(under_crosshair(&list, eye, 0.0, 0.0, 3.0).map(|t| t.id), Some(1));
        assert_eq!(under_crosshair(&list, eye, 180.0, 0.0, 3.0), None);
        assert!(wanted(0, &list[1]) && !wanted(0, &list[0]) && wanted(3, &list[0]));
    }

    #[test]
    fn gui_layout_toggles_and_expands() {
        reset();
        load(none);
        let layout = gui_layout(640.0);
        let ka = index_of("killaura").unwrap();
        let rect = layout.iter().find(|r| r.item == GuiItem::Module(ka)).unwrap().rect;
        let hit = gui_hit(&layout, rect[0] + 2.0, rect[1] + 2.0);
        assert_eq!(hit, Some(GuiItem::Module(ka)));
        gui_press(GuiItem::Module(ka), 0);
        assert!(is_on("killaura"));
        gui_press(GuiItem::Module(ka), 1);
        assert!(expanded(ka));
        let layout = gui_layout(640.0);
        assert!(layout.iter().any(|r| r.item == GuiItem::Setting(ka, 1)));
        gui_press(GuiItem::Setting(ka, 1), 1);
        assert_eq!(get_num("killaura", "range"), 4.0);
        assert_eq!(setting_text(ka, 1), ("Alcance", "4".to_owned()));
        assert!(layout.iter().all(|r| r.rect[0] >= 4.0 && r.rect[0] + r.rect[2] <= 640.0));
    }

    #[test]
    fn nofall_only_spoofs_real_falls() {
        reset();
        load(none);
        command("nofall");
        assert!(!nofall_spoof(false));
        run(|r| r.fall = 3.0);
        assert!(nofall_spoof(false));
        command("nofall");
        assert!(!nofall_spoof(false));
    }

    #[test]
    fn binds_and_actions() {
        reset();
        load(|k| (k == "killaura.bind").then(|| "key.keyboard.r".to_owned()));
        let ka = index_of("killaura").unwrap();
        assert_eq!(modules_for_key("key.keyboard.r"), vec![ka]);
        assert_eq!(key_label("key.keyboard.right.shift"), "Shift direito");
        assert_eq!(key_label("key.keyboard.k"), "K");
        assert_eq!(key_label(""), "Nenhuma");
        assert_eq!(parse_key("f"), "key.keyboard.f");
        assert_eq!(parse_key("shift direito"), "key.keyboard.right.shift");
        let r = command("bind flight g");
        assert_eq!(r.store, vec![("flight.bind".to_owned(), "key.keyboard.g".to_owned())]);
        assert_eq!(modules_for_key("key.keyboard.g"), vec![index_of("flight").unwrap()]);
        command("bind flight");
        assert!(modules_for_key("key.keyboard.g").is_empty());
        gui_press(GuiItem::Module(ka), 2);
        assert_eq!(binding(), Some(ka));
        assert_eq!(bind_text(ka), "Aperte uma tecla");
        set_bind(ka, "key.keyboard.x");
        assert_eq!(binding(), None);
        assert_eq!(bind_text(ka), "X");
        assert_eq!(command("locate vila").action, Some(Action::Locate("vila".into())));
        assert_eq!(command("seed 123").action, Some(Action::Seed(Some("123".into()))));
        assert_eq!(command("nick Steve").action, Some(Action::Nick(Some("Steve".into()))));
    }

    #[test]
    fn hud_info_and_facing() {
        reset();
        load(none);
        assert_eq!(facing(0.0), "Sul");
        assert_eq!(facing(90.0), "Oeste");
        assert_eq!(facing(180.0), "Norte");
        assert_eq!(facing(-90.0), "Leste");
        let l = info_lines([100.5, 64.0, -16.2], false, 144, 4.3, 0.0);
        assert_eq!(l, vec!["XYZ 100 64 -17".to_owned(), "Nether 12 -3".to_owned(), "FPS 144".to_owned()]);
        command("hud speed true");
        assert!(info_lines([0.0; 3], true, 60, 4.3, 0.0).contains(&"Velocidade 4,3 blocos/s".to_owned()));
    }

    #[test]
    fn visited_and_skip() {
        let mut v = visited_parse("10,20;30,40");
        assert_eq!(v, vec![[10, 20], [30, 40]]);
        assert!(is_visited(&v, [12, 64, 22], 8));
        assert!(!is_visited(&v, [100, 64, 100], 8));
        v.push([50, 60]);
        assert_eq!(visited_parse(&visited_text(&v)), v);
        let o = next_origin([0, 64, 0], [300, 64, 0], 600.0);
        assert!(o[0] > 300 && o[1] == 64, "empurra para longe na direção da estrutura");
        assert!((flat_dist([0, 70, 0], [30, 0, 40]) - 50.0).abs() < 1e-6);
    }

    #[test]
    fn locate_names_and_seeds() {
        assert_eq!(locate_query("vila"), (Query::Tag("minecraft:village".into()), "Vila".into()));
        assert_eq!(locate_query("Cidade Antiga").0, Query::Structures(vec!["minecraft:ancient_city".into()]));
        assert_eq!(locate_query("mansão").1, "Mansão");
        assert_eq!(locate_query("bioma cogumelo").0, Query::Biomes(vec!["minecraft:mushroom_fields".into()]));
        assert_eq!(locate_query("bioma dark_forest").0, Query::Biomes(vec!["minecraft:dark_forest".into()]));
        assert_eq!(locate_query("trial_chambers").0, Query::Structures(vec!["minecraft:trial_chambers".into()]));
        assert_eq!(parse_seed("-4172144997902289642"), -4172144997902289642);
        assert_eq!(parse_seed("abc"), 96354);
        assert_eq!(parse_seed("Hello"), 69609650);
        assert_eq!(found_line("Vila", [120, 64, -340], [0, 64, 0]), "Vila mais perto: X 120 Z -340, a 360 blocos");
    }

    #[test]
    fn staff_lines() {
        reset();
        let (_, state, dist, color) = staff_line("Mod", true, true, Some((12.4, true)));
        assert_eq!(state, "fora da tab e espectador");
        assert_eq!(dist, "12 blocos, olhando para você");
        assert_eq!(color, 0xFF_5555);
        assert_eq!(staff_line("A", false, true, None).2, "longe");
        let names = vec!["Mod".to_owned()];
        assert_eq!(staff_new(&names), names);
        assert!(staff_new(&names).is_empty(), "não avisa de novo");
        assert_eq!(staff_new(&["Mod".to_owned(), "Adm".to_owned()]), vec!["Adm".to_owned()]);
    }

    #[test]
    fn face_and_best() {
        assert_eq!(face_dir([0, 1, 0]), 1);
        assert_eq!(face_dir([0, 0, -1]), 2);
        assert_eq!(face_dir([-1, 0, 0]), 4);
        assert_eq!(best_index(&[1.0, 5.0, 2.0]), Some(1));
        assert_eq!(best_index(&[]), None);
    }

    #[test]
    fn freecam_and_arc() {
        let d = freecam_delta(0.0, true, false, false, false, false, false, 1.0);
        assert!((d[2] - 1.0).abs() < 1e-9 && d[1] == 0.0, "yaw 0 anda para o sul");
        let d = freecam_delta(0.0, false, false, false, false, true, false, 2.0);
        assert_eq!(d, [0.0, 2.0, 0.0], "Espaço sobe");
        assert_eq!(projectile_params("minecraft:ender_pearl"), Some((1.5, 0.03, 0.99)));
        assert_eq!(projectile_params("minecraft:dirt"), None);
        let pts = arc([0.0, 70.0, 0.0], [0.0, 0.0, 1.5], 0.03, 0.99, 20);
        assert_eq!(pts.len(), 21);
        assert!(pts[20][2] > 20.0 && pts[20][1] < 70.0, "avança e cai");
    }

    #[test]
    fn active_list_is_widest_first() {
        reset();
        load(none);
        command("esp");
        command("killaura");
        command("nofall");
        let lines = active_lines(|t| t.len() as f32);
        let names: Vec<&str> = lines.iter().map(|l| l.0.as_str()).collect();
        assert_eq!(names, ["KillAura", "NoFall", "ESP", "HUD"]);
    }
}
