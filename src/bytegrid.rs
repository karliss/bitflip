use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufReader};
use std::io::{Error, ErrorKind};
use std::ops::{Index, IndexMut};
use std::path::{Path, PathBuf};

use encoding::Encoding;

const N: usize = 256;

struct ByteGrid {
    data: Box<[[u8; N]; N]>,
}

impl ByteGrid {
    pub fn new() -> ByteGrid {
        ByteGrid {
            data: Box::new([[0u8; N]; N]),
        }
    }

    //TODO: pub fn load_raw newlines?

    pub fn load(path: &Path, encoding: &Encoding) -> Result<ByteGrid, Error> {
        let mut result = ByteGrid::new();
        let reader = BufReader::new(File::open(&path)?);
        for (i, line) in reader.lines().enumerate() {
            let line = line?;
            let line = line.trim_end_matches('\r');
            let (_, tail) = encoding.decode_utf8(line.chars(), &mut result.data[i])?;
            if tail.is_empty() {
                eprintln!("Warning: trailing chars on line {}", i + 1);
            }
        }

        Ok(result)
    }
}

impl Index<(u8, u8)> for ByteGrid {
    type Output = u8;

    fn index(&self, idx: (u8, u8)) -> &u8 {
        &self.data[idx.0 as usize][idx.1 as usize]
    }
}

impl IndexMut<(u8, u8)> for ByteGrid {
    fn index_mut(&mut self, idx: (u8, u8)) -> &mut u8 {
        &mut self.data[idx.0 as usize][idx.1 as usize]
    }
}

impl Index<u16> for ByteGrid {
    type Output = u8;

    fn index(&self, idx: u16) -> &u8 {
        let high = (idx >> 8) as u8;
        let low = (idx & 0xff) as u8;
        &self[(high, low)]
    }
}

impl IndexMut<u16> for ByteGrid {
    fn index_mut(&mut self, idx: u16) -> &mut u8 {
        let high = (idx >> 8) as u8;
        let low = (idx & 0xff) as u8;
        &mut self[(high, low)]
    }
}
