use std::io::{stdin, stdout, Stdout};
use std::io::{Read, Write};
use std::thread;
use std::time::Duration;
use termion::event::{Event, Key};
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::{async_stdin, AsyncReader};

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

impl UiWidget<(), Option<usize>> for Menu {
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
                eprintln!("{:?}", e);
                false
            }
        }
    }

    fn result(&self) -> Option<Option<usize>> {
        self.result
    }

    fn run(&mut self, ui: &mut UiContext) -> Option<Option<usize>> {
        None
    }
}

pub struct UiContext<'a> {
    raw_out: ::termion::raw::RawTerminal<::std::io::StdoutLock<'a>>,
    async_in: AsyncReader,
}

impl<'a> UiContext<'a> {
    pub fn create(out: &'a Stdout) -> Option<UiContext<'a>> {
        if let Ok(a) = out.lock().into_raw_mode() {
            Some(UiContext {
                raw_out: a,
                async_in: async_stdin(),
            })
        } else {
            return None;
        }
    }

    pub fn run<IN, OUT>(&mut self, widget: &mut UiWidget<IN, OUT>) -> Option<OUT> {
        widget.print(self);
        while widget.result().is_none() {
            let mut hasInput = false;
            let mut retry = 5;
            while !hasInput && retry > 0 {
                while let Some(t) = (&mut self.async_in).events().next() {
                    match t {
                        Ok(Event::Key(Key::Ctrl('c'))) => {
                            return None;
                        }
                        _ => {}
                    }
                    widget.input(&t.unwrap());
                    hasInput = true;
                }
                if !hasInput {
                    thread::sleep(Duration::from_millis(10));
                }
                retry -= 1;
            }
            widget.print(self);
        }
        widget.result()
    }
}

pub trait UiWidget<Input, Output> {
    fn print(&self, &mut UiContext) -> std::io::Result<()>;
    fn input(&mut self, e: &Event) -> bool {
        return false;
    }
    fn run(&mut self, ui: &mut UiContext) -> Option<Output>;
    fn result(&self) -> Option<Output>;
}
