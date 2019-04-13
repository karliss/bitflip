use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::{Error, ErrorKind};
use std::ops::{Index, IndexMut};
use std::path::Path;

use crate::encoding::Encoding;
use tgame::vecmath::V2;

const N: usize = 256;

pub type ByteGrid = Grid<u8>;

#[derive(Clone)]
pub struct Grid<T> {
    data: Box<[[T; N]; N]>,
}

impl<T: PartialEq> PartialEq for Grid<T> {
    fn eq(&self, other: &Grid<T>) -> bool {
        self.data
            .iter()
            .zip(other.data.iter())
            .all(|(a, b)| a.iter().eq(b.iter()))
    }
}
impl<T: Eq> Eq for Grid<T> {}

impl Grid<u8> {
    pub fn new() -> Grid<u8> {
        Grid {
            data: Box::new([[0u8; N]; N]),
        }
    }

    pub fn load(path: &Path, encoding: &Encoding) -> Result<Grid<u8>, Error> {
        let mut result = ByteGrid::new();
        let reader = BufReader::new(File::open(&path)?);
        for (i, line) in reader.lines().enumerate() {
            let line = line?;
            let line = line.trim_end_matches('\r');
            let (_, tail) = encoding.decode_utf8(line.chars(), &mut result.data[i])?;
            if !tail.is_empty() {
                eprintln!("Warning: trailing chars on line {}", i + 1);
            }
        }

        Ok(result)
    }

    pub fn from_raw_str(data: &[u8]) -> Grid<u8> {
        let mut result = ByteGrid::new();
        let mut px = 0usize;
        let mut py = 0usize;
        for c in data {
            if *c == b'\n' {
                px = 0;
                py += 1;
            } else {
                result
                    .data
                    .get_mut(py)
                    .map(|row| row.get_mut(px).map(|cell| *cell = *c));
                px += 1;
            }
        }
        result
    }

    pub fn save(&self, out: &mut ::std::io::Write, encoding: &Encoding) -> Result<(), Error> {
        let mut buf = [0u8; 4 * N + 8];
        for line in self.data.iter() {
            let mut offset = 0 as usize;
            for byte in line.iter() {
                let c = encoding.byte_to_char[*byte as usize];
                let utf8_char = c.encode_utf8(&mut buf[offset..]);
                offset += utf8_char.len();
            }
            {
                let e = '\n'.encode_utf8(&mut buf[offset..]);
                offset += e.len();
            }
            match out.write(&buf[..offset]) {
                Ok(size) => {
                    if size != offset {
                        return Err(Error::new(ErrorKind::Interrupted, "Write interrupter?"));
                    }
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }
        Ok(())
    }

    pub fn diff(&self, after: &Grid<u8>) -> ByteGridDiff {
        let mut result = ByteGridDiff::new();
        for i in 0u16..=::std::u16::MAX {
            if self[i] != after[i] {
                let mut add_new_hunk = true;
                if let Some(last_hunk) = result.hunks.last_mut() {
                    match last_hunk {
                        DiffHunk::Seq(pos, data) => {
                            let end = *pos as usize + data.len();
                            if i as usize - end <= 3 {
                                for j in end as u16..=i {
                                    data.push(after[j]);
                                }
                                add_new_hunk = false;
                            }
                        }
                    }
                }
                if add_new_hunk {
                    result.hunks.push(DiffHunk::Seq(i, vec![after[i]]));
                }
            }
        }
        result
    }

    pub fn patch(&mut self, diff: &ByteGridDiff) {
        for hunk in &diff.hunks {
            match hunk {
                DiffHunk::Seq(pos, data) => {
                    let l = std::cmp::min(data.len(), std::u16::MAX as usize + 1 - *pos as usize);
                    for (idx, v) in data[0..l].iter().enumerate() {
                        self[(pos + idx as u16)] = *v;
                    }
                }
            }
        }
    }
}

enum DiffHunk {
    Seq(u16, Vec<u8>),
}

pub struct ByteGridDiff {
    hunks: Vec<DiffHunk>,
}

impl ByteGridDiff {
    fn new() -> ByteGridDiff {
        ByteGridDiff { hunks: Vec::new() }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();
        for hunk in &self.hunks {
            match hunk {
                DiffHunk::Seq(pos, data) => {
                    let mut current_pos = *pos as usize;
                    for fragment in data.chunks(256) {
                        result.push(current_pos as u8);
                        result.push((current_pos >> 8) as u8);
                        result.push((fragment.len() - 1) as u8);
                        result.extend_from_slice(fragment);
                        current_pos += fragment.len();
                    }
                }
            }
        }
        result
    }

    pub fn deserialize(data: &Vec<u8>) -> Result<ByteGridDiff, ()> {
        let mut result = ByteGridDiff::new();
        let mut pos = 0 as usize;
        while data.len() - pos >= 4 {
            let grid_pos = data[pos] as u16 + ((data[pos + 1] as u16) << 8);
            let len = (data[pos + 2] as usize) + 1;
            pos += 3;
            if let Some(hunk_data) = data.get(pos..pos + len) {
                result.hunks.push(DiffHunk::Seq(grid_pos, hunk_data.into()));
            } else {
                return Err(());
            }
            pos += len;
        }
        if data.len() - pos > 0 {
            Err(())
        } else {
            Ok(result)
        }
    }
}

impl Index<(u8, u8)> for ByteGrid {
    type Output = u8;

    fn index(&self, idx: (u8, u8)) -> &u8 {
        &self.data[idx.1 as usize][idx.0 as usize]
    }
}

impl IndexMut<(u8, u8)> for ByteGrid {
    fn index_mut(&mut self, idx: (u8, u8)) -> &mut u8 {
        &mut self.data[idx.1 as usize][idx.0 as usize]
    }
}

impl Index<u16> for ByteGrid {
    type Output = u8;
    fn index(&self, idx: u16) -> &u8 {
        let x = (idx >> 8) as u8;
        let y = (idx & 0xff) as u8;
        &self[(x, y)]
    }
}

impl IndexMut<u16> for ByteGrid {
    fn index_mut(&mut self, idx: u16) -> &mut u8 {
        let x = (idx >> 8) as u8;
        let y = (idx & 0xff) as u8;
        &mut self[(x, y)]
    }
}

impl Index<V2> for ByteGrid {
    type Output = u8;
    fn index(&self, idx: V2) -> &u8 {
        &self[(idx.x as u8, idx.y as u8)]
    }
}

impl IndexMut<V2> for ByteGrid {
    fn index_mut(&mut self, idx: V2) -> &mut u8 {
        &mut self[(idx.x as u8, idx.y as u8)]
    }
}

const BITS_IN_BYTE: usize = 8;
type WordType = u8;

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct Bits256 {
    data: [WordType; Bits256::WORD_COUNT],
}

impl Bits256 {
    pub const WORD_SIZE: usize = std::mem::size_of::<WordType>() * BITS_IN_BYTE;
    pub const WORD_COUNT: usize = 256 / Bits256::WORD_SIZE;
    pub const BIT_COUNT: usize = Bits256::WORD_COUNT * Bits256::WORD_SIZE;

    pub fn new() -> Bits256 {
        Bits256 {
            data: [0u8; Bits256::WORD_COUNT],
        }
    }

    pub fn set(&mut self, idx: u8, value: bool) {
        let byte = idx as usize / BITS_IN_BYTE;
        let bit = (1 as WordType) << (idx as usize % BITS_IN_BYTE);
        if value {
            self.data[byte] |= bit;
        } else {
            self.data[byte] &= !bit;
        }
    }

    pub fn get(&self, idx: u8) -> bool {
        let byte = idx as usize / BITS_IN_BYTE;
        (self.data[byte] >> (idx as usize % BITS_IN_BYTE)) & 1 == 1
    }

    pub fn clear(&mut self) {
        for v in &mut self.data {
            *v = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_data() -> Vec<(ByteGrid, ByteGrid)> {
        let mut ans = Vec::new();

        //everything differs
        let a = ByteGrid::new();
        let mut b = ByteGrid::new();
        for i in 0u16..=std::u16::MAX {
            b[i] = i as u8;
        }
        ans.push((a, b.clone()));

        let mut a = b.clone();
        a[10u16] = 1;
        a[11u16] = 2;
        a[200u16] = 3;
        ans.push((b, a));

        ans
    }

    #[test]
    fn empty_patch() {
        let mut before = ByteGrid::new();
        for i in 0u16..=std::u16::MAX {
            before[i] = (i ^ (i >> 8)) as u8;
        }
        let diff = before.diff(&before);
        assert!(diff.hunks.is_empty());
        let mut other = before.clone();
        other.patch(&diff);
        assert!(before == other);
    }

    #[test]
    fn diff_patch() {
        let test_data = get_test_data();
        for (a, b) in test_data {
            let patch = a.diff(&b);
            let mut c = a.clone();
            c.patch(&patch);
            assert!(c == b);
        }
    }

    #[test]
    fn diff_serialize() {
        let test_data = get_test_data();
        for (a, b) in test_data {
            let patch = a.diff(&b);
            let serialized: Vec<u8> = patch.serialize();
            let deseriaized = ByteGridDiff::deserialize(&serialized).unwrap();
            let mut c = a.clone();
            c.patch(&deseriaized);
            assert!(c == b);
        }
    }

    #[test]
    fn from_str() {
        let test_data = ByteGrid::from_raw_str(b"aa\nbbb");
        debug_assert_eq!(test_data[(0, 0)], b'a');
        debug_assert_eq!(test_data[(1, 0)], b'a');
        debug_assert_eq!(test_data[(2, 0)], 0u8);
        debug_assert_eq!(test_data[(0, 1)], b'b');
        debug_assert_eq!(test_data[(2, 1)], b'b');
        debug_assert_eq!(test_data[(3, 1)], 0u8);
        debug_assert_eq!(test_data[(0, 2)], 0u8);
    }

    #[test]
    fn test_bits256() {
        let mut a = Bits256::new();
        for i in 0..Bits256::BIT_COUNT {
            assert_eq!(false, a.get(i as u8));
        }
        a.set(0, true);
        a.set(255, true);
        a.set(7, true);
        a.set(8, true);
        for i in 0..Bits256::BIT_COUNT {
            assert_eq!(i == 0 || i == 7 || i == 8 || i == 255, a.get(i as u8));
        }
        let b = a.clone();
        a.set(1, false);
        assert_eq!(true, a.get(0));
        assert_eq!(b, a);
        a.set(0, false);
        assert_eq!(false, a.get(0));
        assert_eq!(true, a.get(7));

        a.set(21, true);
        for i in 16u8..24u8 {
            assert_eq!(i == 21, a.get(i));
        }

        a.clear();
        assert_eq!(a, Bits256::new());
    }
}
