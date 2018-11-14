use gameplay::*;
use std::io::Write;
use termion::event::{Event, Key};
use termion::{color, style};
use ui::*;
use vecmath::*;

enum GameState {
    MainMenu,
    Gameplay,
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

    fn print(&self, ui: &mut UiContext) -> std::io::Result<()> {
        self.current_widget().print(ui)
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
                                        //self.gameplay_ui = box GamePlayUI::new(ui)
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

    fn resize(&mut self, widget_size: &Rectangle, window: &V2) {
        self.main_menu.resize(widget_size, window);
        self.gameplay_ui.resize(widget_size, window);
    }
}

struct GamePlayUI {
    id: UiId,
    size: Rectangle,
    game: GamePlayState,
}

impl GamePlayUI {
    fn new(ui: &mut UiContext) -> GamePlayUI {
        GamePlayUI {
            id: ui.next_id(),
            size: DEFAULT_WINDOW_SIZE,
            game: GamePlayState::new_empty(),
        }
    }

    fn set_state(&mut self, new_state: GamePlayState) {
        self.game = new_state;
    }
}

impl UiWidget for GamePlayUI {
    fn print(&self, ui: &mut UiContext) -> std::io::Result<()> {
        write!(
            ui.raw_out,
            "{}{}",
            ::termion::clear::All,
            ::termion::cursor::Hide
        )?;
        ui.goto(self.size.pos)?;
        match self.game.player {
            PlayerPos::Pos(p) => {
                write!(ui.raw_out, "Player location: SYSTEM RAM (page:{})", self.game.cpu[0].get_register(RegisterId::Page).value)?;
            }
            PlayerPos::Register(_) => {
                write!(ui.raw_out, "Player location: Register")?;
            }
        }
        let middle = V2::make((self.size.pos.x + self.size.size.x)/2, self.size.pos.y);
        ui.goto(middle)?;
        match self.game.player {
            PlayerPos::Pos(p) => {
                write!(ui.raw_out, "Player position: {:3},{:3} ({}{:02x}{:02x}{})", p.x, p.y,
                       color::Fg(color::Red),
                       p.x, p.y,
                       color::Fg(color::Reset)
                )?;
            }
            PlayerPos::Register(_) => {
                write!(ui.raw_out, "Register")?;
            }
        }
        ui.raw_out.flush()?;
        Ok(())
    }

    fn input(&mut self, e: &Event) -> Option<UiEvent> {
        //TODO:keybindings
        match e {
            Event::Key(Key::Up) | Event::Key(Key::Char('k')) => {
                self.game.move_player(MoveDir::Up);
            }
            Event::Key(Key::Left)|Event::Key(Key::Char('h')) => {
                self.game.move_player(MoveDir::Left);
            }
            Event::Key(Key::Down)|Event::Key(Key::Char('j')) => {
                self.game.move_player(MoveDir::Down);
            }
            Event::Key(Key::Right)|Event::Key(Key::Char('l')) => {
                self.game.move_player(MoveDir::Right);
            }
            _ => {}
        }
        return self.event(UiEventType::None);
    }

    fn resize(&mut self, widget_size: &Rectangle, _window: &V2) {
        self.size = *widget_size;
    }

    fn get_id(&self) -> UiId {
        self.id
    }
}


