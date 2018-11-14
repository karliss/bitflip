use std::any::Any;
use std::io::Stdout;
use std::io::Write;
use std::num::NonZeroU64;
use std::thread;
use std::time::Duration;
use termion::event::{Event, Key};
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::*;
use termion::{async_stdin, AsyncReader};
use vecmath::*;

pub enum UiEventType {
    Ok,
    Canceled,
    Result(Box<Any>),
    Changed,
    None,
    NotConsumed,
}

pub struct UiEvent {
    id: UiId,
    e: UiEventType
}

pub struct Menu {
    id: UiId,
    entries: Vec<String>,
    cancelable: bool,
    selected: usize,
    result: Option<Option<usize>>,
}

impl Menu {
    pub fn new(entries: Vec<String>, cancelable: bool, context: &mut UiContext) -> Menu {
        assert!(entries.len() > 0);
        Menu {
            id: context.next_id(),
            entries,
            cancelable,
            selected: 0,
            result: None,
        }
    }

    fn get_selected(&self) -> usize {
        return self.selected;
    }
    fn result(&self) -> Option<Option<usize>> {
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

    fn input(&mut self, e: &Event) -> Option<UiEvent> {
        match e {
            Event::Key(Key::Down) => {
                self.selected += 1;
                if self.selected >= self.entries.len() {
                    self.selected = 0;
                }
                self.event(UiEventType::Changed)
            }
            Event::Key(Key::Up) => {
                self.selected = if self.selected > 0 {
                    self.selected - 1
                } else {
                    self.entries.len() - 1
                };
                self.event(UiEventType::Changed)
            }
            Event::Key(Key::Char(number @ '0'...'9')) => {
                let n = number.to_digit(10).unwrap() as usize;
                if n < self.entries.len() {
                    self.selected = n;
                }
                self.event(UiEventType::Changed)
            }
            Event::Key(Key::Char('\n')) => {
                self.result = Some(Some(self.selected));
                self.event(UiEventType::Result(box self.selected))
            }

            Event::Key(Key::Esc) => {
                if self.cancelable {
                    self.result = None;
                    self.event(UiEventType::Canceled)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn get_id(&self) -> UiId {
        self.id
    }

    fn resize(&mut self, _: Rectangle, _: V2) {}
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct UiId(NonZeroU64);

pub struct UiContext<'a> {
    raw_out: AlternateScreen<::termion::raw::RawTerminal<::std::io::StdoutLock<'a>>>,
    async_in: AsyncReader,

    id_counter: UiId,
}

impl<'a> UiContext<'a> {
    pub fn create(out: &'a Stdout) -> Option<UiContext<'a>> {
        if let Ok(a) = out.lock().into_raw_mode() {
            Some(UiContext {
                raw_out: AlternateScreen::from(a),
                async_in: async_stdin(),
                id_counter: UiId(NonZeroU64::new(1).unwrap()),
            })
        } else {
            return None;
        }
    }

    pub fn run(&mut self, widget: &mut UiWidget) -> std::io::Result<()> {
        widget.print(self)?;
        let main_id = widget.get_id();
        loop {
            let mut has_input = false;
            let mut retry = 5;
            while !has_input && retry > 0 {
                while let Some(t) = (&mut self.async_in).events().next() {
                    match t {
                        Ok(Event::Key(Key::Ctrl('c'))) => {
                            return Ok(());
                        }
                        _ => {}
                    }
                    match widget.input(&t.unwrap()) {
                        Some(UiEvent {id: main_id, e: UiEventType::Canceled}) => {
                            return Ok(())
                        }
                        Some(UiEvent {id: main_id, e: UiEventType::Ok}) => {
                            return Ok(())
                        }
                        Some(UiEvent {id: main_id, e: UiEventType::Result(_)}) => {
                            return Ok(())
                        }
                        _ => {}
                    }
                    has_input = true;
                }
                if !has_input {
                    thread::sleep(Duration::from_millis(10));
                }
                retry -= 1;
            }
            widget.print(self)?;
        }
        Ok(())
    }

    pub fn next_id(&mut self) -> UiId {
        let result = self.id_counter;
        self.id_counter = UiId(NonZeroU64::new(u64::from(self.id_counter.0) + 1u64).unwrap());
        result
    }
}

pub trait UiWidget {
    fn print(&self, ui: &mut UiContext) -> std::io::Result<()>;
    fn input(&mut self, _e: &Event) -> Option<UiEvent> {
        return None;
    }
    //fn childWidgets(&mut self) -> Vec<&UiWidget> { Vec::new() }
    fn resize(&mut self, widget_size: Rectangle, window: V2);
    fn get_id(&self) -> UiId;
    fn event(&self, e: UiEventType) -> Option<UiEvent> {
        Some(UiEvent{id: self.get_id(), e})
    }
}

enum GameState {
    MainMenu,
    Gameplay,
}
pub struct GameUi {
    id: UiId,
    state: GameState,
    main_menu: Menu,
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
            None => { return None; },
            Some(r) => {
                if r.id == main_menu_id {
                    match r.e {
                        UiEventType::Result(selected) => {
                            if let Ok(v) =  selected.downcast::<usize>() {
                                if *v == 1 {
                                    return self.event(UiEventType::Canceled)
                                }
                            }
                        },
                        UiEventType::Canceled => {
                            return self.event(UiEventType::Canceled)
                        }
                        _ => {}
                    }
                }
                return None;
            }
        }
        None
    }

    fn resize(&mut self, widget_size: Rectangle, window: V2) {
        self.main_menu.resize(widget_size, window)
    }
}
