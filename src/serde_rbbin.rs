use byteorder::{LittleEndian, ReadBytesExt};
use core::fmt;
use serde::de::{self, Deserialize, DeserializeSeed, MapAccess, SeqAccess, Visitor};

use std::fmt::Display;
use std::io::{Cursor, Seek, SeekFrom};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    Message(String),
    Eof,
    Syntax,
    TrailingCharacters,
}

impl Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(std::error::Error::description(self))
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Message(ref msg) => msg,
            Error::Eof => "unexpected end of input",
            Error::Syntax => "Syntax error",
            Error::TrailingCharacters => "Trailing characters",
        }
    }
}

enum DeserializerState {
    Typed,
    ValueToken,
}

pub struct Deserializer<'de> {
    input: Cursor<&'de [u8]>,
    state: DeserializerState,
}

impl<'de> Deserializer<'de> {
    pub fn from_bytes(input: &'de [u8]) -> Self {
        Deserializer {
            input: Cursor::new(input),
            state: DeserializerState::Typed,
        }
    }
}

pub fn from_bytes<'a, T>(s: &'a [u8]) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_bytes(s);
    let t = T::deserialize(&mut deserializer)?;
    if !deserializer.input.read_u8().is_ok() {
        Ok(t)
    } else {
        Err(Error::TrailingCharacters)
    }
}

impl<'de> Deserializer<'de> {
    fn peek_byte(&mut self) -> Result<u8> {
        match self.input.get_ref().get(self.input.position() as usize) {
            Some(v) => Ok(*v),
            None => Err(Error::Eof),
        }
    }

    fn next_byte(&mut self) -> Result<u8> {
        self.input.read_u8().map_err(|_| Error::Eof)
    }

    fn read_u32(&mut self) -> Result<u32> {
        let t = self.input.read_u32::<LittleEndian>();
        t.map_err(|_| Error::Eof)
    }

    fn read_i32(&mut self) -> Result<i32> {
        self.input
            .read_i32::<LittleEndian>()
            .map_err(|_| Error::Eof)
    }

    fn read_str(&mut self, size: usize) -> Result<&'de str> {
        let r = &self
            .input
            .get_ref()
            .get(self.input.position() as usize..)
            .and_then(|v| v.get(..size))
            .ok_or(Error::Eof)?;
        self.input
            .seek(SeekFrom::Current(size as i64))
            .map_err(|_| Error::Message("Seek error should not happen".to_owned()))?;
        std::str::from_utf8(r).map_err(|_| Error::Message("Bad string".to_owned()))
    }

    fn read_str_size<'s>(&'s mut self) -> Result<&'de str>
    where
        'de: 's,
    {
        let size = self.read_u32()?;
        self.read_str(size as usize)
    }

    fn read_str_int<T>(&mut self) -> Result<T>
    where
        T: std::str::FromStr,
        T: 'static,
    {
        self.read_str_size()?
            .parse::<T>()
            .map_err(|_| Error::Syntax)
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.state {
            DeserializerState::ValueToken => self.deserialize_str(visitor),
            DeserializerState::Typed => match self.peek_byte() {
                Ok(1) | Ok(b'c') => self.deserialize_map(visitor),
                Ok(2) => self.deserialize_map(visitor),
                Ok(3) => self.deserialize_seq(visitor),
                Ok(a) => {
                    let pos = self.input.position();
                    Err(Error::Message(format!("Unexpected type {} at {}", a, pos)))
                }
                Err(v) => Err(v),
            },
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bool(self.read_str_int::<u8>()? == 1)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i8(self.read_str_int()?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i16(self.read_str_int()?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i32(self.read_str_int()?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.read_str_int()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(self.read_str_int()?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u16(self.read_str_int()?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u32(self.read_str_int()?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.read_str_int()?)
    }

    fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.read_str_size()?)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    // The `Serializer` implementation on the previous page serialized byte
    // arrays as JSON arrays of bytes. Handle that representation here.
    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
    }

    fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    // Unit struct means a named value containing no data.
    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    // As is done here, serializers are encouraged to treat newtype structs as
    // insignificant wrappers around the data they contain. That means not
    // parsing anything other than the contained value.
    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let v = self.next_byte()?;
        if v != 3 {
            return Err(Error::Message(format!(
                "Expected list at {} got {}",
                self.input.position(),
                v
            )));
        }
        let elements = self.read_u32()?;
        let result = visitor.visit_seq(ListReader::new(&mut self, elements as usize));
        if result.is_ok() {
            let v = self.next_byte()?;
            if v != b'c' {
                return Err(Error::Message(format!(
                    "Expected end of list at {} got {}",
                    self.input.position(),
                    v
                )));
            }
            result
        } else {
            result
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    // Tuple structs look just like sequences in JSON.
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    // Much like `deserialize_seq` but calls the visitors `visit_map` method
    // with a `MapAccess` implementation, rather than the visitor's `visit_seq`
    // method with a `SeqAccess` implementation.
    fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(MapReader::new(&mut self))
    }

    // Structs look just like maps in JSON.
    //
    // Notice the `fields` parameter - a "struct" in the Serde data model means
    // that the `Deserialize` implementation is required to know what the fields
    // are before even looking at the input data. Any key-value pairing in which
    // the fields cannot be known ahead of time is probably a map.
    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!();
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    // Like `deserialize_any` but indicates to the `Deserializer` that it makes
    // no difference which `Visitor` method is called because the data is
    // ignored.
    //
    // Some deserializers are able to implement this more efficiently than
    // `deserialize_any`, for example by rapidly skipping over matched
    // delimiters without paying close attention to the data in between.
    //
    // Some formats are not able to implement this at all. Formats that can
    // implement `deserialize_any` and `deserialize_ignored_any` are known as
    // self-describing.
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

struct MapReader<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    next_value: u8,
}

impl<'a, 'de> MapReader<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        MapReader { de, next_value: 0 }
    }
}

impl<'de, 'a> MapAccess<'de> for MapReader<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        let kind = self.de.next_byte()?;
        match kind {
            b'c' => return Ok(None),
            1 => {
                self.de.state = DeserializerState::ValueToken;
                seed.deserialize(&mut *self.de).map(Some)
            }
            2 => {
                self.de.state = DeserializerState::Typed;
                seed.deserialize(&mut *self.de).map(Some)
            }
            _ => Err(Error::Syntax),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        // Deserialize a map value.
        seed.deserialize(&mut *self.de)
    }
}

struct ListReader<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    count: usize,
}

impl<'a, 'de> ListReader<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, count: usize) -> Self {
        ListReader { de, count }
    }
}

impl<'de, 'a> SeqAccess<'de> for ListReader<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        if self.count == 0 {
            return Ok(None);
        }
        self.count -= 1;
        self.de.state = DeserializerState::Typed;
        seed.deserialize(&mut *self.de).map(Some)
    }
}

////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_simple_map() {
        // empty
        let data = [b'c'];
        assert_eq!(Ok(json!({})), from_bytes(&data));
        // single string
        let data = [1u8, 1, 0, 0, 0, b'a', 1, 0, 0, 0, b'b', b'c'];
        assert_eq!(Ok(json!({"a": "b"})), from_bytes(&data));
        // mutliple string
        let data = [
            1u8, 1, 0, 0, 0, b'a', 1, 0, 0, 0, b'b', 1u8, 2, 0, 0, 0, b'a', b'b', 1, 0, 0, 0, b'f',
            b'c',
        ];
        assert_eq!(Ok(json!({"a": "b", "ab": "f"})), from_bytes(&data));
    }

    #[test]
    fn test_empty_value() {
        // single string
        let data = [1u8, 1, 0, 0, 0, b'a', 0, 0, 0, 0, b'c'];
        assert_eq!(Ok(json!({"a": ""})), from_bytes(&data));

        // single string
        let data = [
            1u8, 1, 0, 0, 0, b'a', 0, 0, 0, 0, 1u8, 1, 0, 0, 0, b'b', 0, 0, 0, 0, 1u8, 1, 0, 0, 0,
            b'c', 0, 0, 0, 0, b'c',
        ];
        assert_eq!(Ok(json!({"a": "", "b": "", "c": ""})), from_bytes(&data));
    }

    #[test]
    fn nested_struct() {
        // single nested empty list
        let data = [2u8, 1, 0, 0, 0, b'a', b'c', b'c'];
        assert_eq!(Ok(json!({"a": {}})), from_bytes(&data));

        // non-empty nested list
        let data = [
            2u8, 1, 0, 0, 0, b'a', 1, 1, 0, 0, 0, b'b', 1, 0, 0, 0, b'c', b'c', b'c',
        ];
        assert_eq!(Ok(json!({"a": { "b" : "c"}})), from_bytes(&data));

        // multiple levels and fields
        let data = [
            2u8, 1, 0, 0, 0, b'a', 2, 1, 0, 0, 0, b'b', 1, 1, 0, 0, 0, b'c', 1, 0, 0, 0, b'd',
            b'c', 1, 1, 0, 0, 0, b'e', 1, 0, 0, 0, b'f', b'c', b'c',
        ];
        assert_eq!(
            Ok(json!({"a": { "b" : {"c": "d"}, "e": "f"}})),
            from_bytes(&data)
        );
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct TestStructI32 {
        a: i32,
    }

    #[test]
    fn test_i32() {
        // single positive
        let data = [1u8, 1, 0, 0, 0, b'a', 1, 0, 0, 0, b'5', b'c'];
        assert_eq!(Ok(TestStructI32 { a: 5 }), from_bytes(&data));

        // negative
        let data = [1u8, 1, 0, 0, 0, b'a', 2, 0, 0, 0, b'-', b'5', b'c'];
        assert_eq!(Ok(TestStructI32 { a: -5 }), from_bytes(&data));

        // multiple digits
        let data = [1u8, 1, 0, 0, 0, b'a', 2, 0, 0, 0, b'5', b'4', b'c'];
        assert_eq!(Ok(TestStructI32 { a: 54 }), from_bytes(&data));
    }

    #[test]
    fn test_array() {
        // two empty lists
        let data = [2u8, 1, 0, 0, 0, b'a', 3, 2, 0, 0, 0, b'c', b'c', b'c', b'c'];
        assert_eq!(Ok(json!({"a": [{}, {}]})), from_bytes(&data));

        // non empty lists
        let data = [
            2u8, 1, 0, 0, 0, b'a', 3, 2, 0, 0, 0, 1, 1, 0, 0, 0, b'a', 0, 0, 0, 0, b'c', 1, 1, 0,
            0, 0, b'a', 0, 0, 0, 0, b'c', b'c', b'c',
        ];
        assert_eq!(Ok(json!({"a": [{"a": ""}, {"a": ""}]})), from_bytes(&data));
    }

    #[test]
    fn nested_array() {
        // two empty lists
        let data = [
            2u8, 1, 0, 0, 0, b'a', 3, 2, 0, 0, 0, 3, 1, 0, 0, 0, 3, 0, 0, 0, 0, b'c', b'c', b'c',
            b'c', b'c',
        ];
        assert_eq!(Ok(json!({"a": [[[]], {}]})), from_bytes(&data));
    }
}
