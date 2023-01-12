use std::{collections::HashMap, num::ParseIntError};
use thiserror::Error;

pub fn parse_icc(icc: &str) -> Result<HashMap<String, String>, IccError> {
    let tag_data = get_tag_data();
    let mut i = 0_usize;
    let mut output_data = HashMap::new();
    while i < icc.len() {
        let start_of_element = i;
        let mut tag: String;
        let first_tag_byte = &icc[i..i+2];
        let first_tag_bits = usize::from_str_radix(first_tag_byte, 16)?;
        let tag = if first_tag_bits & 31 == 31 { // check that bit5 -> bit1 are set to 1
            let second_tag_byte = &icc[i+2..i+4];
            format!("{}{}", first_tag_byte, second_tag_byte)
        } else {
            first_tag_byte.into()
        };
        let tag_data = match tag_data.get(&tag.to_uppercase()) {
            Some(tag_data) => tag_data,
            None => return Err(IccError::BadTag(tag.into())) // don't recognise the tag
        };
        i += tag.len(); // move pointer to length encoding
        let (value_byte_length, shift_pointer) = unhexify_length(&icc[i..])?;
        i += shift_pointer;
        let value_string_length = value_byte_length * 2;
        let value_bytes: String = icc[i..i+value_string_length].to_uppercase().to_string();
        if value_bytes.len() / 2 != value_byte_length {
            return Err(IccError::GenericError);
        }
        //  check that string is hex here
        if value_bytes.len() / 2 > tag_data.max_length {
            return Err(IccError::GenericError); // too big for max length
        }
        output_data.insert(tag_data.name.clone(), value_bytes);
        i += value_string_length // go to next tag
    }
    Ok(output_data)
}

fn unhexify_length(slice: &str) -> Result<(usize, usize), IccError> {
    let mut length_in_bytes = 0_usize;
    let mut shift_pointer = 2;
    if slice.len() < 1 {
        return Err(IccError::BadLength("length is missing for this tag!".into()));
    }
    let first_length_byte = usize::from_str_radix(&slice[..2], 16)?;
    if first_length_byte < 128 { // bit8 is 0 so bit7 -> bit1 encode the length (127 max)
        length_in_bytes = first_length_byte;
    } else { // bit8 is 1 so the length is encoded by a number of bytes - that number is encoded by bit7 -> bit1

        let extra_bytes = first_length_byte & 127;
        let extra_length = &slice[2..2+(extra_bytes*2)];
        if extra_length.len() < 1 {
            return Err(IccError::BadLength("length is missing for this tag!".into()));
        }
        length_in_bytes = usize::from_str_radix(extra_length, 16)?;
        shift_pointer += extra_bytes * 2;
    }
    if length_in_bytes < 1 {
        return Err(IccError::BadLength("too short".into()));
    }
    return Ok((length_in_bytes, shift_pointer))
}

fn get_tag_data() -> HashMap<String, TagData> {
    HashMap::from([
        ("9F33".into(), TagData::new("terminalcapabilities", "9F33", 6, 6)),
    ])
}

struct TagData {
    name: String,
    tag: String,
    max_length: usize,
    min_length: usize
}

impl TagData {
    fn new(name: &str, tag: &str, max_length: usize, min_length: usize) -> Self {
        Self { name: name.into(), tag: tag.into(), max_length, min_length }
    }
}

#[derive(Debug, PartialEq, Error)]
pub enum IccError {
    #[error("BadTag: {0}")]
    BadTag(String),
    #[error("BadLength: {0}")]
    BadLength(String),
    #[error("InvalidHex: {0}")]
    InvalidHex(#[from] ParseIntError),

    #[error("something was wrong")]
    GenericError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_correct_icc() {
        let tests = vec![
            ("9F3303E0A8B1", vec![("terminalcapabilities", "E0A8B1")]),
        ];
        for (icc_string, expected) in tests {
            let expected: HashMap<String, String> = expected.into_iter().map(|(s1, s2): (&str, &str)| (s1.to_owned(), s2.to_owned())).collect();
            assert_eq!(Ok(expected), parse_icc(icc_string));
        }
    }

    #[test]
    fn test_parse_incorrect_icc() {
        let tests = vec![
            ("9FXX03E0A8B1", IccError::BadTag("9FXX".into())),
        ];
        for (icc_string, expected) in tests {
            assert_eq!(Err(expected), parse_icc(icc_string));
        }
    }
}