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
}

pub struct UiEvent {
    pub id: UiId,
    pub e: UiEventType,
}

pub trait UiWidget {
    fn print(&self, ui: &mut UiContext) -> std::io::Result<()>;
    fn input(&mut self, _e: &Event) -> Option<UiEvent> {
        return None;
    }
    //fn childWidgets(&mut self) -> Vec<&UiWidget> { Vec::new() }
    fn resize(&mut self, widget_size: &Rectangle, window: &V2);
    fn get_id(&self) -> UiId;
    fn event(&self, e: UiEventType) -> Option<UiEvent> {
        Some(UiEvent {
            id: self.get_id(),
            e,
        })
    }
}

pub const DEFAULT_WINDOW_SIZE: Rectangle = Rectangle {
    pos: V2 { x: 0, y: 0 },
    size: V2 { x: 80, y: 24 },
};

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

    fn resize(&mut self, _widget_size: &Rectangle, _window: &V2) {} //TODO: respect size
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct UiId(NonZeroU64);

pub struct UiContext<'a> {
    pub raw_out: AlternateScreen<::termion::raw::RawTerminal<::std::io::StdoutLock<'a>>>,
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

    pub fn goto(&mut self, p: V2) -> std::io::Result<()> {
        write!(
            self.raw_out,
            "{}",
            ::termion::cursor::Goto(1 + p.x as u16, 1 + p.y as u16),
        )
    }

    pub fn run(&mut self, widget: &mut UiWidget) -> std::io::Result<()> {
        widget.print(self)?;
        let main_id = widget.get_id();
        let mut last_size = (0u16, 0u16);
        loop {
            let mut has_input = false;
            let mut retry = 5;
            let new_size = ::termion::terminal_size()?;
            if new_size != last_size {
                let window_size = V2::make(new_size.0 as i32, new_size.1 as i32);
                widget.resize(
                    &Rectangle {
                        pos: V2::make(0, 0),
                        size: window_size,
                    },
                    &window_size,
                );
                last_size = new_size;
            }
            while !has_input && retry > 0 {
                while let Some(t) = (&mut self.async_in).events().next() {
                    match t {
                        Ok(Event::Key(Key::Ctrl('c'))) => {
                            return Ok(());
                        }
                        _ => {}
                    }
                    if let Some(ui_event) = widget.input(&t.unwrap()) {
                        if ui_event.id == main_id {
                            match ui_event.e {
                                UiEventType::Canceled => return Ok(()),
                                UiEventType::Ok => return Ok(()),
                                UiEventType::Result(_) => return Ok(()),
                                _ => {}
                            }
                        }
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
