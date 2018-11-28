use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::path::Path;

use bytegrid::ByteGrid;
use encoding::Encoding;
use vecmath::*;

const GRID_MAX: u8 = 0xff;
const PLAYER_VAL: u8 = b'@';
const PLAYER_OFFSET: usize = 6;
const DEFAULT_PAGE: u8 = 0x42;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum PlayerPos {
    Pos(V2),
    Register(RegisterId),
}

#[derive(Serialize, Deserialize, Copy, Clone)]
enum TriggerKind {
    SetPC(u16),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Trigger {
    pos: V2,
    effect: TriggerKind,
    #[serde(default = "trigger_default_one_time")]
    one_time: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "trigger_skip_triggered")]
    triggered: bool,
}

fn trigger_skip_triggered(v: &bool) -> bool {
    *v == false
}
fn trigger_default_one_time() -> bool {
    true
}

impl Trigger {
    pub fn is_active(&self) -> bool {
        !self.triggered || !self.one_time
    }
}

pub struct PageState {
    pub memory: ByteGrid,
    pub triggers: HashMap<u16, Trigger>,
}

fn joinu8(x: u8, y: u8) -> u16 {
    ((x as u16) << 8) + y as u16
}

pub fn joinu16(p: V2) -> u16 {
    joinu8(p.x as u8, p.y as u8)
}

fn splitu16(p: u16) -> V2 {
    V2 {
        x: (p >> 8) as i32,
        y: (p & 0xff) as i32,
    }
}

impl PageState {
    pub fn new() -> PageState {
        PageState {
            memory: ByteGrid::new(),
            triggers: HashMap::new(),
        }
    }
    pub fn from_grid_raw(grid: ByteGrid) -> PageState {
        PageState {
            memory: grid,
            triggers: HashMap::new(),
        }
    }
    pub fn from_grid(grid: ByteGrid) -> PageState {
        let mut ans = PageState::from_grid_raw(grid);
        let PageState { memory, triggers } = &mut ans;
        let mut trigger_offset = 0x24;
        while trigger_offset < 0x100 {
            let px = memory[(0, trigger_offset as u8)];
            let py = memory[(1, trigger_offset as u8)];

            let tx = memory[(2, trigger_offset as u8)];
            let ty = memory[(3, trigger_offset as u8)];
            let triger_pos = joinu8(px, py);
            if px != 0 && py != 0 {
                triggers.insert(
                    triger_pos,
                    Trigger {
                        pos: V2 {
                            x: px as i32,
                            y: py as i32,
                        },
                        effect: TriggerKind::SetPC(joinu8(tx, ty)),
                        one_time: true,
                        triggered: false,
                    },
                );
            } else {
                break;
            }
            trigger_offset += 1;
        }
        ans
    }
}

#[derive(Serialize, Deserialize, Default)]
struct GameRules {
    #[serde(default)]
    wrap_mode: WrapingMode,
}

impl GameRules {
    fn new() -> GameRules {
        GameRules {
            wrap_mode: WrapingMode::Block,
        }
    }
}

pub struct GamePlayState {
    pub player: PlayerPos,
    pub player_page: u8,
    pub player_offset: u8,
    pub pages: HashMap<u8, PageState>,
    pub cpu: Vec<CPU>,
    game_rules: GameRules,
    null_page: PageState,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum MoveDir {
    Up,
    Left,
    Down,
    Right,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Serialize, Deserialize)]
enum WrapingMode {
    Block,
    WrapLine,
    WrapGrid,
}

impl Default for WrapingMode {
    fn default() -> WrapingMode {
        WrapingMode::Block
    }
}

fn step(p0: V2, d: MoveDir, mode: WrapingMode) -> V2 {
    if mode == WrapingMode::WrapGrid {
        let joined = joinu8(p0.x as u8, p0.y as u8);
        let add = match d {
            MoveDir::Up => 0xffffu16,
            MoveDir::Left => 0xff00u16,
            MoveDir::Down => 0x0001u16,
            MoveDir::Right => 0x0100u16,
        };

        return splitu16(joined.wrapping_add(add));
    }
    let dp = match d {
        MoveDir::Up => V2 { x: 0, y: -1 },
        MoveDir::Left => V2 { x: -1, y: 0 },
        MoveDir::Down => V2 { x: 0, y: 1 },
        MoveDir::Right => V2 { x: 1, y: 0 },
    };
    let tp = p0 + dp;
    if tp.x >= 0 && tp.x <= GRID_MAX as i32 && tp.y >= 0 && tp.y <= GRID_MAX as i32 {
        tp
    } else {
        match mode {
            WrapingMode::Block => p0,
            WrapingMode::WrapLine => V2 {
                x: (tp.x + 256) & 0xff,
                y: (tp.y + 256) & 0xff,
            },
            WrapingMode::WrapGrid => {
                unreachable!();
            }
        }
    }
}

enum LevelFormat {
    SingleGrid,
    Folder,
    RBStorage,
}

impl GamePlayState {
    pub fn new() -> GamePlayState {
        GamePlayState {
            player: PlayerPos::Pos(V2::new()),
            player_page: 0,
            player_offset: PLAYER_OFFSET as u8,
            pages: HashMap::new(),
            cpu: vec![CPU::new()],
            game_rules: GameRules::new(),
            null_page: PageState::new(),
        }
    }

    pub fn player_mask(&self) -> u8 {
        1 << self.player_offset
    }

    fn get_start(grid: &ByteGrid) -> V2 {
        let mut result = V2::new();
        for y in 0u8..=GRID_MAX {
            for x in 0u8..=GRID_MAX {
                if grid[(x, y)] == PLAYER_VAL {
                    result = V2 {
                        x: x as i32,
                        y: y as i32,
                    };
                    return result;
                }
            }
        }
        result
    }

    fn set_initial_page(&mut self, page: u8) {
        self.player_page = page;
        self.cpu[0].set_register(RegisterId::Page, page);
    }

    pub fn from_grid(grid: ByteGrid) -> GamePlayState {
        let mut state = GamePlayState::new();
        state.player = PlayerPos::Pos(GamePlayState::get_start(&grid));
        state.pages.insert(DEFAULT_PAGE, PageState::from_grid(grid));
        if let Some(page) = &mut state.pages.get_mut(&DEFAULT_PAGE) {
            if let PlayerPos::Pos(p) = &state.player {
                page.memory[*p] = 0;
            }
        }
        state.set_initial_page(DEFAULT_PAGE);
        state
    }

    pub fn load_tmp() -> std::io::Result<GamePlayState> {
        GamePlayState::load_from_path(&::resource::get_resource_dir()?.join("levels/ram.txt"))
    }

    pub fn single_from_path(path: &Path) -> std::io::Result<GamePlayState> {
        let encoding = ::encoding::Encoding::get_encoding("437")?;
        let grid = ByteGrid::load(path, &encoding)?;
        Ok(GamePlayState::from_grid(grid))
    }

    pub fn new_empty() -> GamePlayState {
        let mut state = GamePlayState::new();
        let grid = ByteGrid::new();
        state.player = PlayerPos::Pos(GamePlayState::get_start(&grid));
        state.pages.insert(DEFAULT_PAGE, PageState::from_grid(grid));
        if let Some(page) = &mut state.pages.get_mut(&DEFAULT_PAGE) {
            if let PlayerPos::Pos(p) = &state.player {
                page.memory[*p] = 0;
            }
        }
        state.set_initial_page(DEFAULT_PAGE);
        state
    }

    fn detect_level_format(path: &Path) -> std::io::Result<LevelFormat> {
        if path.is_dir() {
            return Ok(LevelFormat::Folder);
        }
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == ".storage" {
                    return Ok(LevelFormat::RBStorage);
                }
            }
            return Ok(LevelFormat::SingleGrid);
        }
        return Err(::std::io::Error::new(
            ::std::io::ErrorKind::InvalidData,
            "Unrecognized level format",
        ));
    }

    pub fn load_from_folder(path: &Path) -> std::io::Result<GamePlayState> {
        //let docs = ::yaml_rust::YamlLoader::
        let config_path = path.join("config.yaml");
        let level_config = if config_path.exists() {
            LevelConfig::load(&config_path.to_path_buf())?
        } else {
            LevelConfig::new()
        };
        let encoding = Encoding::get_encoding(&level_config.encoding)?;
        let mut game_state = GamePlayState::new();
        game_state.game_rules = level_config.rules;

        //pages in yaml
        for page_config in &level_config.page_descr {
            let file_name = if let Some(name) = &page_config.file_name {
                name.clone()
            } else {
                let name = format!("{}.pdiff", page_config.id);
                if path.join(&name).exists() {
                    name
                } else {
                    format!("{}.txt", page_config.id)
                }
            };
            //TODO: finish implementing pdiff support
            let byte_grid = ByteGrid::load(&path.join(file_name), &encoding)?;
            let mut page_state = PageState::from_grid(byte_grid);
            for trigger in &page_config.extra_triggers {
                page_state.triggers.insert(
                    joinu8(trigger.pos.x as u8, trigger.pos.y as u8),
                    trigger.clone(),
                );
            }
            game_state.pages.insert(page_config.id, page_state);
        }

        // rest of the pages named number.txt
        for file in path.iter() {
            let path = path.join(file);
            if !path.is_file() {
                continue;
            }
            let name = file.to_str().unwrap_or("not");
            if let Ok(number) = name.parse::<u8>() {
                if !game_state.pages.contains_key(&number) {
                    let byte_grid = ByteGrid::load(&path, &encoding)?;
                    game_state
                        .pages
                        .insert(number, PageState::from_grid(byte_grid));
                }
            }
        }

        if let Some(page_id) = level_config.initial_page {
            game_state.set_initial_page(page_id);
        } else if game_state.pages.len() == 1 {
            if let Some(id) = game_state.pages.keys().next() {
                game_state.set_initial_page(*id);
            } else {
                game_state.set_initial_page(DEFAULT_PAGE);
            }
        } else {
            game_state.set_initial_page(DEFAULT_PAGE);
        }

        let initial_pos = if let Some(pos) = level_config.initial_pos {
            pos
        } else {
            GamePlayState::get_start(&game_state.current_page().memory)
        };
        game_state.player = PlayerPos::Pos(initial_pos);
        let player_mask = game_state.player_mask();
        if let Some(page) = game_state.pages.get_mut(&game_state.player_page) {
            if page.memory[initial_pos] == player_mask {
                page.memory[initial_pos] = 0;
            }
        }

        Ok(game_state)
    }

    pub fn load_from_path(path: &Path) -> std::io::Result<GamePlayState> {
        let level_format = GamePlayState::detect_level_format(path)?;

        match level_format {
            LevelFormat::SingleGrid => GamePlayState::single_from_path(path),
            LevelFormat::Folder => GamePlayState::load_from_folder(path),
            LevelFormat::RBStorage => {
                unimplemented!();
            }
        }
    }

    pub fn accessible(&self, p: u8) -> bool {
        return (p & (self.player_mask())) == 0;
    }

    pub fn current_page(&self) -> &PageState {
        let page_id = self.player_page;
        self.pages.get(&page_id).unwrap_or(&self.null_page)
    }

    pub fn effective_value(&self, page: &PageState, p: V2) -> u8 {
        let v = page.memory[p];
        if self.player == PlayerPos::Pos(p) {
            let v = page.memory[p];
            v | self.player_mask()
        } else {
            v
        }
    }

    pub fn apply_triggers(&mut self) {
        if let PlayerPos::Pos(pos) = self.player {
            let effect = if let Some(page) = self.pages.get_mut(&self.player_page) {
                //TODO: what happens when player is in inactive page
                if let Some(trigger) = page.triggers.get_mut(&joinu16(pos)) {
                    if !trigger.is_active() {
                        return;
                    }
                    trigger.triggered = true;
                    trigger.effect
                } else {
                    return;
                }
            } else {
                return;
            };
            match effect {
                TriggerKind::SetPC(new_pc) => {
                    self.cpu[0].pc = new_pc;
                }
            }
        };
    }

    pub fn move_player(&mut self, dir: MoveDir) {
        self.cpu[0].get_register(RegisterId::Page);
        let current_page = self.current_page();
        match self.player {
            PlayerPos::Pos(v) => {
                let target = step(v, dir, self.game_rules.wrap_mode);
                if self.accessible(self.effective_value(current_page, target)) {
                    self.player = PlayerPos::Pos(target);
                }
            }
            PlayerPos::Register(r) => {
                //TODO:implement register move
            }
        }
        self.apply_triggers();
        if self.player_page == self.cpu[0].get_register(RegisterId::Page).value {
            self.step_cpu(0);
        }
    }

    fn step_cpu(&mut self, id: usize) {
        let player_mask = self.player_mask();
        let cpu = &mut self.cpu[id];
        let page_id = cpu.get_register_effective(RegisterId::Page, self.player, player_mask);
        let pc = cpu.pc;
        let instr = self.read_instruction(pc, page_id);
        let cpu = &mut self.cpu[id];
        cpu.pc = pc.checked_add(1).unwrap_or(pc);
        let compare_value =
            cpu.get_register_effective(RegisterId::Compare, self.player, player_mask);
        match instr {
            Instruction::Swap(pos) => {
                let v = cpu.get_register(RegisterId::Data).value;
                if let Some(page) = self.pages.get_mut(&page_id) {
                    cpu.set_register(RegisterId::Data, page.memory[pos]);
                    page.memory[pos] = v;
                    if self.player_page == page_id && self.player == PlayerPos::Pos(splitu16(pos)) {
                        self.player = PlayerPos::Register(RegisterId::Data)
                    } else if self.player == PlayerPos::Register(RegisterId::Data) {
                        self.player = PlayerPos::Pos(splitu16(pos));
                        self.player_page = page_id;
                    }
                }
            }
            Instruction::Jump(target) => {
                cpu.pc = target;
            }
            Instruction::JumpEqual(target) => {
                if compare_value == 0 {
                    cpu.pc = target;
                }
            }
            Instruction::JumpGreater(target) => {
                if compare_value == 1 {
                    cpu.pc = target;
                }
            }
            Instruction::JumpLess(target) => {
                if compare_value > 1 {
                    cpu.pc = target;
                }
            }
            Instruction::Compare(v) => {
                let data = cpu.get_register_effective(RegisterId::Data, self.player, player_mask);
                cpu.set_register(
                    RegisterId::Compare,
                    if data > v {
                        1
                    } else if data < v {
                        !0
                    } else {
                        0
                    },
                );
            }
            Instruction::Page(v) => {
                unimplemented!();
            }
            Instruction::Add(v) => {
                let data = cpu.get_register_effective(RegisterId::Data, self.player, player_mask);
                //TODO: check how player bit gets handled
                cpu.set_register(RegisterId::Data, data.wrapping_add(v));
            }
            Instruction::None => {
                cpu.pc = pc;
            }
        }
    }

    fn read_instruction(&self, pc: u16, page_id: u8) -> Instruction {
        let page = self.pages.get(&page_id).unwrap_or(&self.null_page);
        let p = splitu16(pc);
        let instr = self.effective_value(page, p);
        let arg_u8 = || {
            let a0 = p + V2::make(1, 0);
            if a0.x < 256 {
                self.effective_value(page, a0)
            } else {
                0
            }
        };
        let arg_u16 = || {
            let a0 = p + V2::make(1, 0);
            if a0.x < 256 {
                let high = self.effective_value(page, a0);
                let a1 = a0 + V2::make(1, 0);
                let low = if a1.x < 256 {
                    self.effective_value(page, a1)
                } else {
                    0
                };
                ((high as u16) << 8) | (low as u16)
            } else {
                0
            }
        };

        match instr {
            b'j' => Instruction::Jump(arg_u16()),
            b's' => Instruction::Swap(arg_u16()),
            b'c' => Instruction::Compare(arg_u8()),
            b'e' => Instruction::JumpEqual(arg_u16()),
            b'l' => Instruction::JumpLess(arg_u16()),
            b'g' => Instruction::JumpGreater(arg_u16()),
            b'a' => Instruction::Add(arg_u8()),
            b'p' => Instruction::Page(arg_u8()),
            _ => Instruction::None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct PageDescr {
    #[serde(default)]
    extra_triggers: Vec<Trigger>,
    id: u8,
    base_name: Option<String>,
    file_name: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct LevelConfig {
    #[serde(default)]
    initial_page: Option<u8>,
    #[serde(default)]
    initial_pos: Option<V2>,
    #[serde(default)]
    rules: GameRules,
    #[serde(default = "LevelConfig::default_encoding")]
    encoding: String,
    #[serde(default)]
    page_descr: Vec<PageDescr>,
}

impl LevelConfig {
    fn new() -> LevelConfig {
        LevelConfig {
            initial_page: None,
            initial_pos: None,
            rules: GameRules::new(),
            encoding: "437".to_owned(),
            page_descr: Vec::new(),
        }
    }

    fn default_encoding() -> String {
        "437".to_owned()
    }

    fn load(path: &Path) -> std::io::Result<LevelConfig> {
        let file = std::fs::File::open(path)?;
        let y: serde_yaml::Result<LevelConfig> = ::serde_yaml::from_reader(file);
        match y {
            Ok(res) => Ok(res),
            Err(e) => {
                eprintln!("Level loading error: {}", e);
                Err(Error::new(ErrorKind::InvalidData, e))
            }
        }
    }
}

enum Instruction {
    Swap(u16),
    Jump(u16),
    Compare(u8),
    JumpEqual(u16),
    JumpLess(u16),
    JumpGreater(u16),
    Add(u8),
    Page(u8),
    None,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum RegisterId {
    Data = 0,
    Page = 1,
    Compare = 2,
}

pub struct CPU {
    registers: Vec<Register>,
    pub pc: u16,
}

pub struct Register {
    pub value: u8,
    pub protected: bool,
    pub name: String,
}

impl CPU {
    pub fn new() -> CPU {
        CPU {
            pc: 0,
            registers: vec![
                Register {
                    value: 0,
                    protected: false,
                    name: "data".to_owned(),
                },
                Register {
                    value: 0,
                    protected: false,
                    name: "page".to_owned(),
                },
                Register {
                    value: 0xff,
                    protected: false,
                    name: "compare".to_owned(),
                },
            ],
        }
    }
    pub fn get_register(&self, id: RegisterId) -> &Register {
        &self.registers[id as usize]
    }
    pub fn set_register(&mut self, id: RegisterId, value: u8) {
        self.registers[id as usize].value = value;
    }
    pub fn get_register_effective(
        &self,
        id: RegisterId,
        player_pos: PlayerPos,
        player_mask: u8,
    ) -> u8 {
        let v = self.registers[id as usize].value;
        match player_pos {
            PlayerPos::Register(r) if r == id => (v | player_mask),
            _ => v,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_pos_normal() {
        let tests = vec![
            (V2 { x: 1, y: 1 }, MoveDir::Up, V2 { x: 1, y: 0 }),
            (V2 { x: 1, y: 1 }, MoveDir::Left, V2 { x: 0, y: 1 }),
            (V2 { x: 1, y: 1 }, MoveDir::Down, V2 { x: 1, y: 2 }),
            (V2 { x: 1, y: 1 }, MoveDir::Right, V2 { x: 2, y: 1 }),
            (V2 { x: 10, y: 5 }, MoveDir::Up, V2 { x: 10, y: 4 }),
            (V2 { x: 10, y: 5 }, MoveDir::Left, V2 { x: 9, y: 5 }),
            (V2 { x: 10, y: 5 }, MoveDir::Down, V2 { x: 10, y: 6 }),
            (V2 { x: 10, y: 5 }, MoveDir::Right, V2 { x: 11, y: 5 }),
        ];
        for test_mode in vec![
            WrapingMode::Block,
            WrapingMode::WrapLine,
            WrapingMode::WrapGrid,
        ] {
            for (p0, dir, expected) in &tests {
                assert_eq!(step(*p0, *dir, test_mode), *expected);
            }
        }
    }

    #[test]
    fn add_pos_wrap() {
        let tests = vec![
            (
                V2 { x: 0, y: 0 },
                MoveDir::Up,
                V2 { x: 0, y: 0 },
                V2 { x: 0, y: 255 },
                V2 { x: 255, y: 255 },
            ),
            (
                V2 { x: 0, y: 0 },
                MoveDir::Left,
                V2 { x: 0, y: 0 },
                V2 { x: 255, y: 0 },
                V2 { x: 255, y: 0 },
            ),
            (
                V2 { x: 255, y: 255 },
                MoveDir::Down,
                V2 { x: 255, y: 255 },
                V2 { x: 255, y: 0 },
                V2 { x: 0, y: 0 },
            ),
            (
                V2 { x: 255, y: 255 },
                MoveDir::Right,
                V2 { x: 255, y: 255 },
                V2 { x: 0, y: 255 },
                V2 { x: 0, y: 255 },
            ),
            (
                V2 { x: 5, y: 0 },
                MoveDir::Up,
                V2 { x: 5, y: 0 },
                V2 { x: 5, y: 255 },
                V2 { x: 4, y: 255 },
            ),
            (
                V2 { x: 5, y: 255 },
                MoveDir::Down,
                V2 { x: 5, y: 255 },
                V2 { x: 5, y: 0 },
                V2 { x: 6, y: 0 },
            ),
        ];
        for (p0, dir, expected_block, expected_line, expected_grid) in &tests {
            assert_eq!(step(*p0, *dir, WrapingMode::Block), *expected_block);
            assert_eq!(step(*p0, *dir, WrapingMode::WrapLine), *expected_line);
            assert_eq!(step(*p0, *dir, WrapingMode::WrapGrid), *expected_grid);
        }
    }

    #[test]
    fn effective_value() {
        let st = GamePlayState::new_empty();
        let current_page = st.current_page();
        let v = st.effective_value(current_page, V2::make(0, 0));
        assert_eq!(v, b'@');
        let v = st.effective_value(current_page, V2::make(0, 1));
        assert_eq!(v, 0u8);

        //TODO: add combination with other bytes
    }

    #[test]
    fn trigger() {
        let grid = ByteGrid::from_str("@");
        let mut game = GamePlayState::from_grid(grid);
        game.pages.get_mut(&DEFAULT_PAGE).map(|page| {
            page.triggers.insert(
                0x0100,
                Trigger {
                    pos: V2::make(1, 0),
                    effect: TriggerKind::SetPC(0x1010),
                    triggered: false,
                    one_time: true,
                },
            );
            page.triggers.insert(
                0x0200,
                Trigger {
                    pos: V2::make(2, 0),
                    effect: TriggerKind::SetPC(0x1110),
                    triggered: false,
                    one_time: false,
                },
            );
            page.triggers.insert(
                0x0300,
                Trigger {
                    pos: V2::make(3, 0),
                    effect: TriggerKind::SetPC(0x1210),
                    triggered: false,
                    one_time: true,
                },
            );
        });

        assert_eq!(game.cpu[0].pc, 0);
        {
            let trigger = game.current_page().triggers.get(&0x100).unwrap();
            assert_eq!(trigger.is_active(), true);
        }
        game.move_player(MoveDir::Right);
        assert_eq!(game.cpu[0].pc, 0x1010);
        {
            let trigger = game.current_page().triggers.get(&0x100).unwrap();
            assert_eq!(trigger.is_active(), false);
        }
        game.move_player(MoveDir::Right);
        assert_eq!(game.cpu[0].pc, 0x1110);
        game.move_player(MoveDir::Right);
        assert_eq!(game.cpu[0].pc, 0x1210);
        game.move_player(MoveDir::Left);
        assert_eq!(game.cpu[0].pc, 0x1110);
        game.move_player(MoveDir::Left);
        assert_eq!(game.cpu[0].pc, 0x1110);
    }
}
