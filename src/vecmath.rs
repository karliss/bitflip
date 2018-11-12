use std::ops::{Add, Sub};

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct V2 {
    pub x: i32,
    pub y: i32,
}

impl V2 {
    pub fn new() -> V2 {
        V2 { x: 0, y: 0 }
    }
}

impl Add for V2 {
    type Output = V2;
    fn add(self, other: V2) -> V2 {
        V2 {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for V2 {
    type Output = V2;
    fn sub(self, other: V2) -> V2 {
        V2 {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

pub struct Rectangle {
    pos: V2,
    size: V2,
}
