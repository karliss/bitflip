use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::{Error, ErrorKind};
use std::ops::{Index, IndexMut};
use std::path::Path;

use encoding::Encoding;
use vecmath::V2;

const N: usize = 256;

#[derive(Clone)]
pub struct ByteGrid {
    data: Box<[[u8; N]; N]>,
}

impl PartialEq for ByteGrid {
    fn eq(&self, other: &ByteGrid) -> bool {
        self.data
            .iter()
            .zip(other.data.iter())
            .all(|(a, b)| a.iter().eq(b.iter()))
    }
}
impl Eq for ByteGrid {}

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
            if !tail.is_empty() {
                eprintln!("Warning: trailing chars on line {}", i + 1);
            }
        }

        Ok(result)
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

    pub fn diff(&self, after: &ByteGrid) -> ByteGridDiff {
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
}
