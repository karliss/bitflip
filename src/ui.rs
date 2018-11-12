use std::any::Any;
use std::io::{stdin, stdout, Stdout};
use std::io::{Read, Write};
use std::thread;
use std::time::Duration;
use termion::event::{Event, Key};
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::*;
use termion::{async_stdin, AsyncReader};
use vecmath::*;

pub struct Menu {
    entries: Vec<String>,
    cancelable: bool,
    selected: usize,
    result: Option<Option<usize>>,
}

impl Menu {
    pub fn new(entries: Vec<String>, cancelable: bool) -> Menu {
        assert!(entries.len() > 0);
        Menu {
            entries,
            cancelable,
            selected: 0,
            result: None,
        }
    }

    fn get_selected(&self) -> usize {
        return self.selected;
    }
    fn get_result(&self) -> Option<Option<usize>> {
        return self.result;
    }
}

impl UiWidget for Menu {
    fn print(&self, ui: &mut UiContext) -> ::std::io::Result<()> {
        write!(
            ui.raw_out,
            "{}{}{}",
            ::termion::clear::All,
            ::termion::cursor::Goto(1, 1),
            ::termion::cursor::Hide
        )?;
        for (i, entry) in self.entries.iter().enumerate() {
            if self.selected != i {
                write!(ui.raw_out, "({}) {}\r\n", i, entry)?;
            } else {
                write!(ui.raw_out, "> ({}) {}\r\n", i, entry)?;
            }
        }
        Ok(())
    }

    fn input(&mut self, e: &Event) -> bool {
        match e {
            Event::Key(Key::Down) => {
                self.selected += 1;
                if self.selected >= self.entries.len() {
                    self.selected = 0;
                }
                true
            }
            Event::Key(Key::Up) => {
                self.selected = if self.selected > 0 {
                    self.selected - 1
                } else {
                    self.entries.len() - 1
                };
                true
            }
            Event::Key(Key::Char(number @ '0'...'9')) => {
                let n = number.to_digit(10).unwrap() as usize;
                if n < self.entries.len() {
                    self.selected = n;
                }
                true
            }
            Event::Key(Key::Char('\n')) => {
                self.result = Some(Some(self.selected));
                true
            }

            Event::Key(Key::Esc) => {
                if self.cancelable {
                    self.result = None;
                    true
                } else {
                    false
                }
            }
            _ => {
                false
            }
        }
    }

    fn result(&self) -> Option<Box<dyn Any>> {
        self.result.map(|v| Box::new(v) as Box<Any>)
    }

    fn run(&mut self, ui: &mut UiContext) -> Option<Box<Any>> {
        None
    }

    fn resize(&mut self, _: Rectangle, _: V2) {}
}

pub struct UiContext<'a> {
    raw_out: AlternateScreen<::termion::raw::RawTerminal<::std::io::StdoutLock<'a>>>,
    async_in: AsyncReader,
}

impl<'a> UiContext<'a> {
    pub fn create(out: &'a Stdout) -> Option<UiContext<'a>> {
        if let Ok(a) = out.lock().into_raw_mode() {
            Some(UiContext {
                raw_out: AlternateScreen::from(a),
                async_in: async_stdin(),
            })
        } else {
            return None;
        }
    }

    pub fn run(&mut self, widget: &mut UiWidget) -> Option<Box<Any>> {
        widget.print(self);
        while widget.result().is_none() {
            let mut has_input = false;
            let mut retry = 5;
            while !has_input && retry > 0 {
                while let Some(t) = (&mut self.async_in).events().next() {
                    match t {
                        Ok(Event::Key(Key::Ctrl('c'))) => {
                            return None;
                        }
                        _ => {}
                    }
                    widget.input(&t.unwrap());
                    has_input = true;
                }
                if !has_input {
                    thread::sleep(Duration::from_millis(10));
                }
                retry -= 1;
            }
            widget.print(self);
        }
        widget.result()
    }
}

pub trait UiWidget {
    fn print(&self, ui: &mut UiContext) -> std::io::Result<()>;
    fn input(&mut self, e: &Event) -> bool {
        return false;
    }
    //fn childWidgets(&mut self) -> Vec<&UiWidget> { Vec::new() }
    fn resize(&mut self, widget_size: Rectangle, window: V2);
    fn run(&mut self, ui: &mut UiContext) -> Option<Box<Any>>;
    fn result(&self) -> Option<Box<Any>>;
}

enum GameState {
    MainMenu,
    Gameplay,
}
pub struct GameUi {
    state: GameState,
    main_menu: Menu,
    result: Option<Result<(), ()>>,
}

impl GameUi {
    pub fn new() -> GameUi {
        GameUi {
            state: GameState::MainMenu,
            main_menu: {
                let result = Menu::new(vec!["New game".to_owned(), "Exit".to_owned()], false);
                result
            },
            result: None,
        }
    }

    fn current_widget_mut(&mut self) -> &mut UiWidget {
        match self.state {
            GameState::MainMenu => &mut self.main_menu,
            GameState::Gameplay => {
                unimplemented!();
            }
        }
    }
    fn current_widget(&self) -> &UiWidget {
        match self.state {
            GameState::MainMenu => &self.main_menu,
            GameState::Gameplay => {
                unimplemented!();
            }
        }
    }
}

impl UiWidget for GameUi {
    fn print(&self, ui: &mut UiContext) -> std::io::Result<()> {
        self.current_widget().print(ui)
    }

    fn input(&mut self, e: &Event) -> bool {
        let result = self.current_widget_mut().input(e);
        match self.state {
            GameState::MainMenu => {
                if let Some(st) = self.main_menu.result() {
                    if let Ok(v) = st.downcast::<Option<usize>>() {
                        match v {
                            box Some(1) => {
                                self.result = Some(Result::Ok(()));
                            }
                            _ => {}
                        }
                    }
                }
            }
            GameState::Gameplay => {}
        }
        result
    }

    fn resize(&mut self, widget_size: Rectangle, window: V2) {
        self.main_menu.resize(widget_size, window)
    }

    fn run(&mut self, ui: &mut UiContext) -> Option<Box<Any>> {
        unimplemented!()
    }

    fn result(&self) -> Option<Box<Any>> {
        self.result.map(|v| Box::new(v) as Box<Any>)
    }
}
