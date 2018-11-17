use std::collections::HashMap;

use bytegrid::ByteGrid;
use vecmath::*;

const GRID_MAX: u8 = 0xff;
const PLAYER_VAL: u8 = b'@';
const DEFAULT_PAGE: u8 = 42;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum PlayerPos {
    Pos(V2),
    Register(i32),
}

enum TriggerKind {
    SetPC(u16),
}

struct Trigger {
    pos: V2,
    kind: TriggerKind,
    one_time: bool,
    triggered: bool,
}

pub struct PageState {
    pub memory: ByteGrid,
    triggers: HashMap<u16, Trigger>,
}

fn joinu8(x: u8, y: u8) -> u16 {
    ((x as u16) << 8) + y as u16
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
                        kind: TriggerKind::SetPC(joinu8(tx, ty)),
                        one_time: true,
                        triggered: false,
                    },
                );
            }
            trigger_offset += 1;
        }
        ans
    }
}

struct GameRules {
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
    pub player_bit: u8,
    pages: HashMap<u8, PageState>,
    pub cpu: Vec<CPU>,
    game_rules: GameRules,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum MoveDir {
    Up,
    Left,
    Down,
    Right,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
enum WrapingMode {
    Block,
    WrapLine,
    WrapGrid,
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

impl GamePlayState {
    pub fn new() -> GamePlayState {
        GamePlayState {
            player: PlayerPos::Pos(V2::new()),
            player_bit: b'@',
            pages: HashMap::new(),
            cpu: vec![CPU::new()],
            game_rules: GameRules::new(),
        }
    }

    fn get_start(grid: &ByteGrid) -> PlayerPos {
        let mut result = PlayerPos::Pos(V2::new());
        for y in 0u8..=GRID_MAX {
            for x in 0u8..=GRID_MAX {
                if grid[(x, y)] == PLAYER_VAL {
                    result = PlayerPos::Pos(V2 {
                        x: x as i32,
                        y: y as i32,
                    });
                    return result;
                }
            }
        }
        result
    }

    pub fn from_grid(grid: ByteGrid) -> GamePlayState {
        let mut state = GamePlayState::new();
        state.player = GamePlayState::get_start(&grid);
        state.pages.insert(DEFAULT_PAGE, PageState::from_grid(grid));
        if let Some(page) = &mut state.pages.get_mut(&DEFAULT_PAGE) {
            if let PlayerPos::Pos(p) = &state.player {
                page.memory[*p] = 0;
            }
        }
        state.cpu[0].set_register(RegisterId::Page, DEFAULT_PAGE);
        state
    }

    pub fn from_path() -> std::io::Result<GamePlayState> {
        let path = ::resource::get_resource_dir()?.join("levels/ram.txt");
        let encoding = ::encoding::Encoding::get_encoding("437")?;
        let grid = ByteGrid::load(path.as_path(), &encoding)?;
        Ok(GamePlayState::from_grid(grid))
    }

    pub fn new_empty() -> GamePlayState {
        let mut state = GamePlayState::new();
        let grid = ByteGrid::new();
        state.player = GamePlayState::get_start(&grid);
        state.pages.insert(DEFAULT_PAGE, PageState::from_grid(grid));
        if let Some(page) = &mut state.pages.get_mut(&DEFAULT_PAGE) {
            if let PlayerPos::Pos(p) = &state.player {
                page.memory[*p] = 0;
            }
        }
        state.cpu[0].set_register(RegisterId::Page, DEFAULT_PAGE);
        state
    }

    pub fn accessible(&self, p: u8) -> bool {
        return (p & self.player_bit) == 0;
    }

    pub fn current_page(&self) -> Option<&PageState> {
        let page_id = self.cpu[0].get_register(RegisterId::Page).value;
        self.pages.get(&page_id)
    }

    pub fn effective_value(&self, page: &PageState, p: V2) -> u8 {
        let v = page.memory[p];
        if self.player == PlayerPos::Pos(p) {
            let v = page.memory[p];
            v | self.player_bit
        } else {
            v
        }
    }

    pub fn move_player(&mut self, dir: MoveDir) {
        self.cpu[0].get_register(RegisterId::Page);
        let current_page = self.current_page().unwrap();
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
        //TODO:trigers
    }
}

pub enum RegisterId {
    Data = 0,
    Page = 1,
    Compare = 2,
    PC = 3,
}

pub struct CPU {
    registers: Vec<Register>,
}

pub struct Register {
    pub value: u8,
    pub protected: bool,
    pub name: String,
}

impl CPU {
    pub fn new() -> CPU {
        CPU {
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
                Register {
                    value: 0,
                    protected: true,
                    name: "PC".to_owned(),
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
        let current_page = st.current_page().unwrap();
        let v = st.effective_value(current_page, V2::make(0, 0));
        assert_eq!(v, b'@');
        let v = st.effective_value(current_page, V2::make(0, 1));
        assert_eq!(v, 0u8);

        //TODO: add combination with other bytes
    }
}
