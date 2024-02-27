#![no_std]
#![forbid(unsafe_code)]
#![warn(clippy::pedantic, clippy::perf, missing_docs, clippy::panic, clippy::cargo)]

//! Provides a function for simply unescaping a string.
//!
//! Designed with [`char::escape_default`] in mind,
//! but should also be compatible with strings from most languages.

extern crate alloc;

use alloc::{
    borrow::Cow,
    string::{
        String,
        ToString
    }
};

mod sealed {
    pub trait Sealed {}
    impl Sealed for &'_ str {}
}

/// Extension trait for &str to allow unescaping of strings.
pub trait UnescapeExt: sealed::Sealed {

    /**
    Unescapes a string, returning an [`alloc::borrow::Cow`].
    Will only allocate if the string has any escape sequences.

    The following escapes are valid:
    - `\\n` -> `\n`
    - `\\r` -> `\r`
    - `\\t` -> `\t`
    - `\\b` -> `\x08`
    - `\\f` -> `\x0C`
    - `\\'` -> `'`
    - `\\"` -> `"`
    - `\\\\` -> `\\`
    - `\\xNN` -> `\xNN`
    - `\\o` -> `\o`
    - `\\oo` -> `\oo`
    - `\\ooo` -> `\ooo`
    - `\\uXXXX` -> `\u{XXXX}`
    - `\\u{HEX}` -> `\u{HEX}`

    # Errors
    Errors if there's an invalid escape sequence in the string.
    Passes back the byte index of the invalid character.

    # Examples
    ```rust
    # extern crate alloc;

    use alloc::borrow::Cow;
    use descape::UnescapeExt;

    let escaped = "Hello,\\nworld!".to_unescaped();
    assert_eq!(
        escaped,
        Ok(Cow::Owned::<'_, str>("Hello,\nworld!".to_string()))
    );

    let no_escapes = "No escapes here!".to_unescaped();
    assert_eq!(
        no_escapes,
        Ok(Cow::Borrowed("No escapes here!"))
    );

    //                              v  invalid at index 9
    let invalid_escape = "Uh oh! \\xJJ".to_unescaped();
    assert_eq!(
        invalid_escape,
        Err(9)
    );
    */
    fn to_unescaped(&self) -> Result<Cow<'_, str>, usize>;
}

impl<'orig> UnescapeExt for &'orig str {
    fn to_unescaped(&self) -> Result<Cow<'orig, str>, usize> {
        // Iterates over each character as a UTF-8 string slice
        let slices = self.split("");
        let indices = self.char_indices().map(|(idx, _)| idx);
        let mut iter = slices
            .zip(indices);
        let mut seen = ""; // Shouldn't need to be initialized but rustc can't tell
        let len = self.len();
        let mut owned = None::<String>;

        while let Some((slice, end_index)) = iter.next() {
            if slice.is_empty() {
                continue;
            }
            if slice != r"\" {
                if let Some(owned) = &mut owned {
                    owned.push_str(slice);
                } else {
                    seen = &self[..end_index];
                }
                continue;
            }
            let owned = owned.get_or_insert_with(
                || seen.to_string()
            );
            if let Some((slice, end_index)) = iter.next() {
                match slice {
                    "b" => owned.push('\x08'),
                    "f" => owned.push('\x0C'),
                    "n" => owned.push('\n'),
                    "t" => owned.push('\t'),
                    "r" => owned.push('\r'),
                    "'" => owned.push('\''),
                    "\"" => owned.push('"'),
                    "\\" => owned.push('\\'),
                    "u" => {
                        let (char, skip) = unescape_unicode(
                            self, end_index, (&mut iter).map(|(slice, _)| slice), slice, len
                        )?;
                        // Skip the needed amount of characters
                        for _ in 0..skip {
                            iter.next();
                        }
                        owned.push(char);
                    },
                    "x" => {
                        owned.push(unescape_hex(self, end_index)?);
                        // Skip two characters
                        iter.next();
                        iter.next();
                    },
                    c if c.chars().next().unwrap().is_digit(8) => {
                        let (char, skip) = unescape_oct(self, end_index)?;
                        // Skip the needed amount of characters
                        for _ in 0..skip {
                            iter.next();
                        }
                        owned.push(char);
                    },
                    _ => return Err(end_index),
                }
            } else {
                // No matches found
                return Err(owned.len());
            }
        }

        match owned {
            Some(string) => Ok(Cow::Owned(string)),
            None => Ok(Cow::Borrowed(self)),
        }
    }
}

fn unescape_unicode<'s>(
    string: &'s str,
    start_index: usize,
    mut iter: impl Iterator<Item = &'s str>,
    slice: &'s str,
    len: usize,
) -> Result<(char, usize), usize> {
    let next = iter.next().ok_or(len)?;
    let end_index = start_index + slice.len();
    if next == "{" {
        // \u{HEX}
        let end = string[end_index ..].find('}')
            .ok_or(start_index - 2)?;
        let num = &string[end_index .. end_index + end];
        let codepoint = u32::from_str_radix(num, 16)
            .map_err(|_| start_index - 2)?;
        char::from_u32(codepoint).ok_or(start_index - 2).map(|v| (v, end + 1))
    } else {
        // \uNNNN
        // If any of these are non-ASCII, then it's already invalid,
        // so a direct slice is fine
        let next_four = string.get(start_index .. start_index + 4)
            .ok_or(start_index - 2)?;
        let codepoint = u32::from_str_radix(next_four, 16).map_err(|_| start_index - 2)?;
        // Encode the u32
        char::from_u32(codepoint).ok_or(start_index - 2).map(|v| (v, 3))
    }
}

// FIXME: This could be factored out along with part of unescape_unicode into its own function.
fn unescape_hex(
    slice: &str,
    start_index: usize,
) -> Result<char, usize> {
    // Must be \xNN
    let codepoint = slice.get(start_index .. start_index + 2)
        .and_then(|num| u32::from_str_radix(num, 16).ok())
        .ok_or(start_index - 2)?;
    char::from_u32(codepoint).ok_or(start_index)
}

fn unescape_oct(
    string: &str,
    start_index: usize,
) -> Result<(char, usize), usize> {
    // Could be \o, \oo, or \ooo
    let (index, (len, char)) = string[start_index..].char_indices()
        .take(3)
        .take_while(|(_, c)| c.is_digit(8))
        .enumerate()
        .last()
        .ok_or(start_index)?;
    let end_index = index + char.len_utf8();
    let num = &string[start_index..end_index];
    let codepoint = u32::from_str_radix(num, 8)
        .map_err(|_| start_index)?;
    char::from_u32(codepoint).map(|v| (v, len - 1)).ok_or(start_index)
}

