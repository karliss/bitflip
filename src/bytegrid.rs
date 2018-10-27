use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader};
use std::io::{Error, ErrorKind};
use std::ops::{Index, IndexMut};
use std::path::{Path, PathBuf};

use encoding::Encoding;

const N: usize = 256;

#[derive(Clone)]
struct ByteGrid {
    data: Box<[[u8; N]; N]>,
}

impl PartialEq for ByteGrid {
    fn eq(&self, other: &ByteGrid) -> bool {
        self.data.iter().zip(other.data.iter())
            .all(|(a,b)|
                a.iter().eq(b.iter()))
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
            if tail.is_empty() {
                eprintln!("Warning: trailing chars on line {}", i + 1);
            }
        }

        Ok(result)
    }

    pub fn diff(&self, after: &ByteGrid) -> ByteGridDiff {
        let mut result = ByteGridDiff::new();
        for i in 0u16 ..= ::std::u16::MAX {
            if self[i] != after[i] {
                let mut add_new_hunk= true;
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
    Seq(u16, Vec<u8>)
}

struct ByteGridDiff {
    hunks: Vec<DiffHunk>
}

impl ByteGridDiff {
    fn new() -> ByteGridDiff {
        ByteGridDiff {
            hunks: Vec::new()
        }
    }

    //TODO: serialize
    //TODO: unserialize
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

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_data() -> Vec<(ByteGrid, ByteGrid)> {
        let mut ans = Vec::new();

        //everything differs
        let mut a = ByteGrid::new();
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
            before[i] = (i ^ (i>>8)) as u8;
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
}