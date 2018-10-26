use std::collections::HashMap;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::str;

pub struct Encoding {
    pub byte_to_char: [char; 256],
    pub char_to_byte: HashMap<char, u8>,
}

impl Encoding {
    fn new() -> Encoding {
        Encoding {
            byte_to_char: ['?'; 256],
            char_to_byte: HashMap::new(),
        }
    }

    fn get_encoding_dir() -> Result<PathBuf, std::io::Error> {
        let current_exe = ::std::env::current_exe()?;
        let current_dir = current_exe.parent().unwrap();
        let test_path = current_dir.join("../../resource/encodings");
        if test_path.exists() {
            return Ok(test_path);
        }
        let test_path = current_dir.join("../../../resource/encodings");
        if test_path.exists() {
            return Ok(test_path);
        }
        let test_path = current_dir.join("resource/encodings");
        if test_path.exists() {
            return Ok(test_path);
        }
        Err(Error::new(ErrorKind::NotFound, "Resource dir not found"))
    }

    pub fn get_encoding(name: &str) -> Result<Encoding, std::io::Error> {
        let encoding_dir = Encoding::get_encoding_dir()?;
        eprintln!("!!! {:?}", encoding_dir);
        Encoding::load(&encoding_dir.join(name))
    }

    pub fn load(path: &Path) -> Result<Encoding, std::io::Error> {
        let mut result = Encoding::new();
        let buf = fs::read_to_string(&path)?;
        let mut i = 0;
        let mut done = false;
        for c in buf.chars() {
            if i >= 256 {
                break;
            }
            match c {
                '\n' => {
                    if !done {
                        result.byte_to_char[i] = ' '; //editors tend to strip trailing space
                        result.char_to_byte.entry(' ').or_insert(i as u8);
                    } else {
                        done = false;
                    }
                    i += 1;
                }
                _ => {
                    if !done {
                        done = true;
                        result.byte_to_char[i] = c;
                        result.char_to_byte.entry(c).or_insert(i as u8);
                    }
                }
            }
        }
        if i != 256 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("Incorrect height {} expected 256", i),
            ));
        }

        Ok(result)
    }

    pub fn decode_utf8<'a>(
        &self,
        mut input: str::Chars<'a>,
        out: &mut [u8],
    ) -> Result<(usize, &'a str), std::io::Error> {
        let mut produced = 0 as usize;
        let n = out.len();
        while produced < n {
            if let Some(c) = input.next() {
                if let Some(byte) = self.char_to_byte.get(&c) {
                    out[produced] = *byte;
                    produced += 1;
                } else {
                    return Err(Error::from(ErrorKind::InvalidData));
                }
            } else {
                return Ok((produced, ""));
            }
        }
        Ok((produced, input.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn read_err() {
        let encoding = Encoding::load(Path::new("doesn't exist"));
        assert_eq!(encoding.is_err(), true);
    }

    #[test]
    fn appproximate_437_check() {
        let encoding = Encoding::get_encoding("437").unwrap();

        assert_eq!(encoding.char_to_byte.get(&'a'), Some(&b'a'));
        assert_eq!(encoding.char_to_byte.get(&'Z'), Some(&b'Z'));
        assert_eq!(encoding.byte_to_char[b'a' as usize], 'a');
        assert_eq!(encoding.byte_to_char[b'Z' as usize], 'Z');

        //last symbol
        assert_eq!(encoding.char_to_byte.get(&'\u{00a0}'), Some(&255));
        assert_eq!(encoding.byte_to_char[255], '\u{00a0}');

        // Non symetric matching and possibly empty line
        assert_eq!(encoding.char_to_byte.get(&' '), Some(&0));
        assert_eq!(encoding.byte_to_char[0], ' ');
        assert_eq!(encoding.byte_to_char[b' ' as usize], ' ');

        //low
        assert_eq!(encoding.byte_to_char[1], '☺');
        assert_eq!(encoding.char_to_byte.get(&'☺'), Some(&1));

        //high
        assert_eq!(encoding.byte_to_char[230], 'µ');
        assert_eq!(encoding.char_to_byte.get(&'µ'), Some(&230));

        //nonexisting
        assert_eq!(encoding.char_to_byte.get(&'\n'), None);
    }

    #[test]
    fn from_utf8() {
        let encoding = Encoding::get_encoding("437").unwrap();

        let mut buf = [0u8; 256];
        let txt = "abcdefABCDEF123456";
        let (len, tail) = encoding.decode_utf8(txt.chars(), &mut buf).unwrap();
        assert_eq!(len, txt.len());
        assert_eq!(&buf[..len], b"abcdefABCDEF123456");
        assert_eq!(tail, "");

        let mut buf = [0u8; 5];
        let txt = "123456";
        assert_eq!(
            encoding.decode_utf8(txt.chars(), &mut buf).unwrap(),
            (5, "6")
        );
        assert_eq!(&buf, b"12345");

        assert!(encoding.decode_utf8("āēūž".chars(), &mut buf).is_err());
    }
}
