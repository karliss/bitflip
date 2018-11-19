use std::io::Write;

use termion::color;
use termion::event::{Event, Key};

use encoding::Encoding;
use gameplay::*;
use ui::*;
use vecmath::*;

enum GameState {
    MainMenu,
    Gameplay,
}

enum PanelType {
    Top = 0,
    Binary = 1,
    Text = 2,
    Right = 3,
    Last = 4,
}

pub struct GameUi {
    id: UiId,
    state: GameState,
    main_menu: Menu,
    gameplay_ui: GamePlayUI,
    result: Option<Result<(), ()>>,
}

impl GameUi {
    pub fn new(context: &mut UiContext) -> GameUi {
        GameUi {
            id: context.next_id(),
            state: GameState::MainMenu,
            main_menu: {
                let result = Menu::new(
                    vec!["New game".to_owned(), "Exit".to_owned()],
                    false,
                    context,
                );
                result
            },
            gameplay_ui: GamePlayUI::new(context),
            result: None,
        }
    }

    fn current_widget_mut(&mut self) -> &mut UiWidget {
        match self.state {
            GameState::MainMenu => &mut self.main_menu,
            GameState::Gameplay => &mut self.gameplay_ui,
        }
    }
    fn current_widget(&self) -> &UiWidget {
        match self.state {
            GameState::MainMenu => &self.main_menu,
            GameState::Gameplay => &self.gameplay_ui,
        }
    }
}

impl UiWidget for GameUi {
    fn get_id(&self) -> UiId {
        return self.id;
    }

    fn print(&mut self, ui: &mut UiContext) -> std::io::Result<()> {
        //write!(ui.raw_out, "{}", ::termion::clear::All)?;
        self.current_widget_mut().print(ui)
    }

    fn input(&mut self, e: &Event) -> Option<UiEvent> {
        let result = self.current_widget_mut().input(e);
        let main_menu_id = self.main_menu.get_id();
        match result {
            None => {
                return None;
            }
            Some(r) => {
                if r.id == main_menu_id {
                    match r.e {
                        UiEventType::Result(selected) => {
                            if let Ok(v) = selected.downcast::<usize>() {
                                match *v {
                                    0 => {
                                        self.state = GameState::Gameplay;
                                        let game_state = GamePlayState::from_path();
                                        if let Ok(gs) = game_state {
                                            self.gameplay_ui.set_state(gs);
                                        } else {
                                            return self.event(UiEventType::Canceled);
                                        }
                                        return self.event(UiEventType::None);
                                    }
                                    1 => return self.event(UiEventType::Canceled),
                                    _ => {}
                                }
                            }
                        }
                        UiEventType::Canceled => return self.event(UiEventType::Canceled),
                        _ => {}
                    }
                }
                return None;
            }
        }
    }

    fn resize(&mut self, widget_size: &Rectangle) {
        self.main_menu.resize(widget_size);
        self.gameplay_ui.resize(widget_size);
    }

    fn update(&mut self) {
        self.current_widget_mut().update();
    }

    fn child_widgets(&self) -> Vec<&UiWidget> {
        vec![&self.main_menu, &self.gameplay_ui]
    }

    fn child_widgets_mut(&mut self) -> Vec<&mut UiWidget> {
        vec![&mut self.main_menu, &mut self.gameplay_ui]
    }
}

struct GamePlayUI {
    id: UiId,
    size: Rectangle,
    game: GamePlayState,
    panel_sizes: [Rectangle; PanelType::Last as usize],
    byte_view: ByteView,
    text_view: TextView,
    last_pos: V2,
    need_clean: i32,
    show_encoding: bool,
    encoding_view: EncodingTable,
}

impl GamePlayUI {
    fn new(ui: &mut UiContext) -> GamePlayUI {
        GamePlayUI {
            id: ui.next_id(),
            size: DEFAULT_WINDOW_SIZE,
            game: GamePlayState::new_empty(),
            panel_sizes: [DEFAULT_WINDOW_SIZE; PanelType::Last as usize],
            last_pos: V2::new(),
            byte_view: ByteView::new(ui),
            text_view: TextView::new(ui),
            need_clean: 0,
            show_encoding: false,
            encoding_view: EncodingTable::new(ui, Encoding::get_encoding("437").unwrap()), //TODO get rid of unwrap
        }
    }

    fn set_state(&mut self, new_state: GamePlayState) {
        self.game = new_state;
    }

    fn print_hbox_grid(&self, ui: &mut UiContext, sizes: &[Rectangle]) -> std::io::Result<()> {
        if sizes.is_empty() {
            return Ok(());
        }
        let boxg = sizes[0].grow(1);

        //│ ┤ ╡ ╢ ╖ ╕ ╣ ║ ╗ ╝ ╜ ╛ ┐ └ ┴ ┬ ├ ─ ┼ ╞ ╟ ╚ ╔ ╩ ╦ ╠ ═ ╬ ╧ ╨ ╤ ╥ ╙ ╘ ╒ ╓ ╫ ╪ ┘ ┌
        ui.goto(boxg.pos)?;
        if boxg.size.x >= 2 {
            write!(ui.raw_out, "{:═<1$}", "╔", (boxg.size.x - 1) as usize)?;
        }
        for rec in &sizes[1..] {
            if rec.size.x >= 0 {
                write!(ui.raw_out, "{:═<1$}", "╦", (rec.size.x + 1) as usize)?;
            }
        }
        write!(ui.raw_out, "╗")?;

        ui.goto(boxg.bottom_left())?;
        if boxg.size.x >= 2 {
            write!(ui.raw_out, "{:═<1$}", "╚", (boxg.size.x - 1) as usize)?;
        }
        for rec in &sizes[1..] {
            if rec.size.x >= 0 {
                write!(ui.raw_out, "{:═<1$}", "╩", (rec.size.x + 1) as usize)?;
            }
        }
        write!(ui.raw_out, "╝")?;

        let right = sizes.last().unwrap().right() + 1;
        for y in sizes[0].top()..(sizes[0].bottom() + 1) {
            for rec in sizes {
                ui.goto(V2::make(rec.left() - 1, y))?;
                write!(ui.raw_out, "║")?;
            }
            ui.goto(V2::make(right, y))?;
            write!(ui.raw_out, "║")?;
        }

        Ok(())
    }

    fn print_edges(&self, ui: &mut UiContext) -> std::io::Result<()> {
        let sizes = [
            *self.get_panel_size(PanelType::Binary),
            *self.get_panel_size(PanelType::Text),
            *self.get_panel_size(PanelType::Right),
        ];
        self.print_hbox_grid(ui, &sizes)?;
        Ok(())
    }

    fn print_top_panel(&self, ui: &mut UiContext) -> std::io::Result<()> {
        write!(ui.raw_out, "{}", ::termion::cursor::Hide)?;
        ui.goto(self.size.pos)?;
        match self.game.player {
            PlayerPos::Pos(_) => {
                write!(
                    ui.raw_out,
                    "Player location: SYSTEM RAM (page:{})",
                    self.game.cpu[0].get_register(RegisterId::Page).value
                )?;
            }
            PlayerPos::Register(_) => {
                write!(ui.raw_out, "Player location: Register")?;
            }
        }
        let middle = V2::make((self.size.pos.x + self.size.size.x) / 2, self.size.pos.y);
        ui.goto(middle)?;
        match self.game.player {
            PlayerPos::Pos(p) => {
                write!(
                    ui.raw_out,
                    "Player position: {:3},{:3} ({}{:02x}{:02x}{})",
                    p.x,
                    p.y,
                    color::Fg(color::Red),
                    p.x,
                    p.y,
                    color::Fg(color::Reset)
                )?;
            }
            PlayerPos::Register(_) => {
                write!(ui.raw_out, "Player position: Register")?;
            }
        }
        Ok(())
    }

    fn get_panel_size(&self, kind: PanelType) -> &Rectangle {
        return &self.panel_sizes[kind as usize];
    }

    fn get_popup_mut(&mut self) -> Option<&mut UiWidget> {
        if self.show_encoding {
            Some(&mut self.encoding_view)
        } else {
            None
        }
    }
}

impl UiWidget for GamePlayUI {
    fn print(&mut self, ui: &mut UiContext) -> std::io::Result<()> {
        if let Some(popup) = self.get_popup_mut() {
            popup.print(ui)?;
        } else {
            if self.need_clean > 0 {
                write!(ui.raw_out, "{}", ::termion::clear::All)?;
            }
            self.print_top_panel(ui)?;
            self.print_edges(ui)?;
            self.byte_view.print_data(ui, (&self.game, self.last_pos))?;
            self.text_view.print_data(ui, (&self.game, self.last_pos))?;
        }
        ui.raw_out.flush()?;
        Ok(())
    }

    fn input(&mut self, e: &Event) -> Option<UiEvent> {
        if self.show_encoding {
            match self.encoding_view.input(e) {
                Some(UiEvent {
                    id: _,
                    e: UiEventType::Ok,
                }) => {
                    self.show_encoding = false;
                    self.need_clean = 2;
                    return self.event(UiEventType::None);
                }
                _ => {}
            }
            return None;
        }
        //TODO:keybindings
        match e {
            Event::Key(Key::Up) | Event::Key(Key::Char('k')) => {
                self.game.move_player(MoveDir::Up);
            }
            Event::Key(Key::Left) | Event::Key(Key::Char('h')) => {
                self.game.move_player(MoveDir::Left);
            }
            Event::Key(Key::Down) | Event::Key(Key::Char('j')) => {
                self.game.move_player(MoveDir::Down);
            }
            Event::Key(Key::Right) | Event::Key(Key::Char('l')) => {
                self.game.move_player(MoveDir::Right);
            }
            Event::Key(Key::Char('x')) => {
                self.show_encoding = true;
                self.encoding_view.resize(&self.size);
                self.encoding_view.init();
            }
            Event::Key(Key::Char('p')) => {
                self.byte_view.mode = match self.byte_view.mode {
                    ByteViewMode::Hex => ByteViewMode::Bits,
                    ByteViewMode::Bits => ByteViewMode::Hex,
                };
            }
            _ => {}
        }
        return self.event(UiEventType::None);
    }

    fn child_widgets(&self) -> Vec<&UiWidget> {
        vec![&self.byte_view, &self.text_view, &self.encoding_view]
    }

    fn child_widgets_mut(&mut self) -> Vec<&mut UiWidget> {
        vec![
            &mut self.byte_view,
            &mut self.text_view,
            &mut self.encoding_view,
        ]
    }

    fn resize(&mut self, widget_size: &Rectangle) {
        self.size = *widget_size;
        let top_size = 3;
        let bottom_size = std::cmp::max(self.size.size.y - top_size - 2, 0);
        self.panel_sizes[PanelType::Top as usize] = Rectangle {
            pos: self.size.pos,
            size: V2::make(self.size.size.x, top_size),
        };
        let bottom_top = self.size.pos + V2::make(0, top_size);
        let right_size = 10;
        let data_width = std::cmp::max(self.size.size.x - right_size - 4, 0);
        let left_width = data_width / 2;
        let middle_width = data_width - left_width;

        let binary_size = Rectangle {
            pos: bottom_top + V2::make(1, 1),
            size: V2::make(left_width, bottom_size),
        };
        self.panel_sizes[PanelType::Binary as usize] = binary_size;
        self.byte_view.resize(&binary_size);

        let text_size = Rectangle {
            pos: self.byte_view.size.top_right() + V2::make(2, 0),
            size: V2::make(middle_width, bottom_size),
        };
        self.text_view.resize(&text_size);
        self.panel_sizes[PanelType::Text as usize] = text_size;

        let right_pos = self.get_panel_size(PanelType::Text).top_right() + V2::make(2, 0);
        self.panel_sizes[PanelType::Right as usize] = Rectangle {
            pos: right_pos,
            size: V2::make(right_size, bottom_size),
        };
        self.need_clean = 2;

        if let Some(popup) = self.get_popup_mut() {
            popup.resize(widget_size);
        }
    }

    fn get_id(&self) -> UiId {
        self.id
    }

    fn update(&mut self) {
        match self.game.player {
            PlayerPos::Pos(p) => {
                self.last_pos = p;
            }
            _ => {}
        }
        for w in self.child_widgets_mut() {
            w.update();
        }
        if self.need_clean > 0 {
            self.need_clean -= 1;
        }
    }
}

enum ByteViewMode {
    Bits,
    Hex,
}

struct ByteView {
    id: UiId,
    size: Rectangle,
    mode: ByteViewMode,
}

impl ByteView {
    fn new(ui: &mut UiContext) -> ByteView {
        ByteView {
            id: ui.next_id(),
            size: DEFAULT_WINDOW_SIZE,
            mode: ByteViewMode::Bits,
        }
    }
}

impl DataWidget<(&GamePlayState, V2)> for ByteView {
    fn print_data(
        &mut self,
        ui: &mut UiContext,
        (data, player): (&GamePlayState, V2),
    ) -> std::io::Result<()> {
        let block_width = match self.mode {
            ByteViewMode::Bits => 8,
            ByteViewMode::Hex => 2,
        };
        let block_count = (self.size.size.x + 1) / (block_width + 1);
        for y in 0..self.size.size.y {
            ui.goto(self.size.pos + V2::make(0, y))?;
            let my = player.y + y - (self.size.size.y / 2);
            if my < 0 || my >= 256 {
                write!(ui.raw_out, "{:1$}", " ", self.size.size.x as usize)?;
            } else {
                let mut px = 0;
                for block_id in 0..block_count {
                    let mx = player.x + block_id - (block_count / 2);
                    if block_id > 0 {
                        write!(ui.raw_out, " ")?;
                        px += 1;
                    }

                    if mx < 0 || mx >= 256 {
                        write!(ui.raw_out, "{:1$}", " ", block_width as usize)?;
                    } else {
                        let pos = V2::make(mx, my);
                        let byte = data.effective_value(data.current_page().unwrap(), pos);
                        let is_player_pos = data.player == PlayerPos::Pos(pos);
                        if !is_player_pos {
                            if data.accessible(byte) {
                                write!(ui.raw_out, "{}", color::Fg(color::LightWhite))?;
                            } else {
                                write!(ui.raw_out, "{}", color::Fg(color::Cyan))?;
                            }
                        }
                        match self.mode {
                            ByteViewMode::Bits => {
                                if is_player_pos {
                                    let left_part_size = 8 - PLAYER_OFFSET - 1;
                                    let left_part = byte >> (8 - left_part_size);
                                    let right_part = byte & (PLAYER_VAL - 1);
                                    let right_part_size = PLAYER_OFFSET;
                                    write!(
                                        ui.raw_out,
                                        "{color_back}{left_part:0>left_width$b}{color_bit}1{color_back}{right_part:0>right_width$b}",
                                        color_back=color::Fg(color::LightWhite),
                                        left_part=left_part,
                                        left_width = left_part_size,
                                        color_bit=color::Fg(color::Yellow),
                                        right_part=right_part,
                                        right_width = right_part_size
                                    )?;
                                } else {
                                    write!(ui.raw_out, "{:08b}", byte)?;
                                }
                            }
                            ByteViewMode::Hex => {
                                if is_player_pos {
                                    write!(ui.raw_out, "{}", color::Fg(color::Yellow))?;
                                }
                                write!(ui.raw_out, "{:02x}", byte)?;
                            }
                        }
                    }
                    px += block_width;
                }
                let padding = self.size.size.x - px;
                if padding > 0 {
                    write!(ui.raw_out, "{:1$}", " ", padding as usize)?;
                }
            }
        }
        write!(ui.raw_out, "{}", color::Fg(color::Reset))?;
        Ok(())
    }
}

impl UiWidget for ByteView {
    fn print(&mut self, _ui: &mut UiContext) -> std::io::Result<()> {
        Ok(())
    }

    fn child_widgets(&self) -> Vec<&UiWidget> {
        Vec::new()
    }

    fn child_widgets_mut(&mut self) -> Vec<&mut UiWidget> {
        Vec::new()
    }

    fn resize(&mut self, widget_size: &Rectangle) {
        self.size = *widget_size;
    }

    fn get_id(&self) -> UiId {
        self.id
    }
}

struct TextView {
    id: UiId,
    size: Rectangle,
    encoding: Encoding,
}

impl TextView {
    fn new(ui: &mut UiContext) -> TextView {
        TextView {
            id: ui.next_id(),
            size: DEFAULT_WINDOW_SIZE,
            encoding: Encoding::get_encoding("437").unwrap(),
        }
    }
}

impl DataWidget<(&GamePlayState, V2)> for TextView {
    fn print_data(
        &mut self,
        ui: &mut UiContext,
        (data, last_pos): (&GamePlayState, V2),
    ) -> std::io::Result<()> {
        let mut buf = [0u8; 16];
        for y in 0..self.size.size.y {
            ui.goto(self.size.pos + V2::make(0, y))?;
            let my = last_pos.y + y - (self.size.size.y / 2);
            if my < 0 || my >= 256 {
                write!(ui.raw_out, "{:1$}", " ", self.size.size.x as usize)?;
            } else {
                for column in 0..self.size.size.x {
                    let mx = last_pos.x + column - (self.size.size.x / 2);

                    if mx < 0 || mx >= 256 {
                        write!(ui.raw_out, " ")?;
                    } else {
                        let pos = V2::make(mx, my);
                        let byte = data.effective_value(data.current_page().unwrap(), pos);
                        let is_player_pos = data.player == PlayerPos::Pos(pos);
                        let c = self.encoding.byte_to_char[byte as usize];
                        let str = c.encode_utf8(&mut buf);
                        if is_player_pos {
                            write!(ui.raw_out, "{}", color::Fg(color::Yellow))?;
                        }
                        ui.raw_out.write_all(str.as_bytes())?;
                        if is_player_pos {
                            write!(ui.raw_out, "{}", color::Fg(color::Reset))?;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl UiWidget for TextView {
    fn print(&mut self, _ui: &mut UiContext) -> std::io::Result<()> {
        Ok(())
    }

    fn child_widgets(&self) -> Vec<&UiWidget> {
        Vec::new()
    }

    fn child_widgets_mut(&mut self) -> Vec<&mut UiWidget> {
        Vec::new()
    }

    fn resize(&mut self, widget_size: &Rectangle) {
        self.size = *widget_size;
    }

    fn get_id(&self) -> UiId {
        self.id
    }
}

struct EncodingTable {
    id: UiId,
    size: Rectangle,
    redraw: bool,
    offset: i32,
    encoding: Encoding,
    rows: i32,
    columns: i32,
    padding: i32,
}

impl EncodingTable {
    fn new(ui: &mut UiContext, encoding: Encoding) -> EncodingTable {
        let mut result = EncodingTable {
            id: ui.next_id(),
            size: DEFAULT_WINDOW_SIZE,
            offset: 0,
            encoding,
            redraw: true,
            rows: 10,
            columns: 10,
            padding: 0,
        };

        result.resize(&DEFAULT_WINDOW_SIZE);
        result
    }

    fn init(&mut self) {
        self.offset = 0;
        self.redraw = true;
    }
}

impl UiWidget for EncodingTable {
    fn print(&mut self, ui: &mut UiContext) -> std::io::Result<()> {
        if !self.redraw {
            return Ok(());
        }
        self.redraw = false;

        write!(ui.raw_out, "{}", ::termion::clear::All)?;
        if self.columns <= 0 || self.rows <= 0 {
            return Ok(());
        }

        let header = format!("HEX DEC {:>8} S|", "BINARY");

        ui.goto(V2::make(self.padding, self.padding))?;
        for _ in 0..self.columns {
            write!(ui.raw_out, "{}", header)?;
        }
        for row in 0..self.rows {
            if row > std::u8::MAX as i32 {
                break;
            }
            ui.goto(V2::make(self.padding, row + self.padding + 1))?;
            let mut p = self.offset + row;
            let mut column = 0;
            while p < 256 && column < self.columns {
                write!(
                    ui.raw_out,
                    " {:02x} {:3} {:08b} {}|",
                    p, p, p, self.encoding.byte_to_char[p as usize]
                )?;
                p += self.rows;
                column += 1;
            }
        }
        if self.padding > 0 && self.columns * self.rows < 256 {
            ui.goto(V2::make(0, self.size.size.y - 1))?;
            write!(ui.raw_out, "Arrow keys to scroll")?;
        }
        Ok(())
    }

    fn input(&mut self, e: &Event) -> Option<UiEvent> {
        match e {
            Event::Key(Key::Char('x')) => self.event(UiEventType::Ok),
            Event::Key(Key::Down) | Event::Key(Key::Right) => {
                if self.offset < 254 {
                    self.offset = std::cmp::min(self.offset + self.rows, 256 - self.rows);
                    if self.offset < 0 {
                        self.offset = 0;
                    }
                }
                self.redraw = true;
                self.event(UiEventType::Changed)
            }
            Event::Key(Key::Up) | Event::Key(Key::Left) => {
                if self.offset > 0 {
                    self.offset = std::cmp::max(self.offset - self.rows, 0);
                }
                self.redraw = true;
                self.event(UiEventType::Changed)
            }
            _ => None,
        }
    }

    fn child_widgets(&self) -> Vec<&UiWidget> {
        Vec::new()
    }

    fn child_widgets_mut(&mut self) -> Vec<&mut UiWidget> {
        Vec::new()
    }

    fn resize(&mut self, widget_size: &Rectangle) {
        self.size = *widget_size;
        self.redraw = true;

        self.padding = if self.size.size.y > 10 { 2 } else { 0 };
        let header = format!("HEX DEC {:>8} S|", "BINARY");
        self.rows = self.size.size.y - 2 * self.padding - 1;
        if self.rows <= 0 {
            self.rows = 0;
            self.columns = 0;
            return;
        }
        let columns_max = self.size.size.x / header.len() as i32;
        self.columns = std::cmp::min((256 + self.rows - 1) / self.rows, columns_max);
        if self.columns <= 0 {
            self.rows = 0;
            self.columns = 0;
            return;
        }
        let visible = self.columns * self.rows;
        if visible > 256 {
            let mut rows_next = self.rows & 0xf0;
            while rows_next * columns_max >= 256 {
                self.rows = rows_next;
                rows_next = rows_next - 256;
                self.columns = (256 + self.rows - 1) / self.rows;
            }
        }
        self.offset = 0;
    }

    fn get_id(&self) -> UiId {
        self.id
    }
}
