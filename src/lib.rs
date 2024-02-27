//#![no_std]
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
    },
    str::{
        CharIndices
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

    //                           v  invalid at index 7
    let invalid_escape = "Uh oh! \\xJJ".to_unescaped();
    assert_eq!(
        invalid_escape,
        Err(7)
    );
    ```
     */
    fn to_unescaped(&self) -> Result<Cow<'_, str>, usize>;
}

impl<'orig> UnescapeExt for &'orig str {
    fn to_unescaped(&self) -> Result<Cow<'orig, str>, usize> {
        // Iterates over each character as a UTF-8 string slice
        let mut iter = self.char_indices();
        let mut seen = ""; // Shouldn't need to be initialized but rustc can't tell
        let mut owned = None::<String>;

        while let Some((idx, chr)) = iter.next() {
            if chr != '\\' {
                if let Some(owned) = &mut owned {
                    owned.push(chr);
                } else {
                    seen = &self[..idx + chr.len_utf8()];
                }
                continue;
            }
            let owned = owned.get_or_insert_with(
                || seen.to_string()
            );
            if let Some((idx, chr)) = iter.next() {
                match chr {
                    'b' => owned.push('\x08'),
                    'f' => owned.push('\x0C'),
                    'n' => owned.push('\n'),
                    't' => owned.push('\t'),
                    'r' => owned.push('\r'),
                    '\'' => owned.push('\''),
                    '"' => owned.push('"'),
                    '\\' => owned.push('\\'),
                    'u' => {
                        let (chr, skip) = unescape_unicode(
                            self, idx, &mut iter
                        )?;
                        // Skip the needed amount of characters
                        for _ in 0..skip {
                            iter.next();
                        }
                        owned.push(chr);
                    },
                    'x' => {
                        owned.push(unescape_hex(self, idx)?);
                        // Skip two characters
                        iter.next();
                        iter.next();
                    },
                    c if c.is_digit(8) => {
                        let (chr, skip) = unescape_oct(self, idx)?;
                        for _ in 0..skip {
                            iter.next();
                        }
                        owned.push(chr);
                    },
                    _ => return Err(idx - 1),
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
    idx: usize,
    iter: &mut CharIndices<'s>
) -> Result<(char, usize), usize> {
    let (next_idx, next) = iter.next().ok_or(string.len())?;
    if next == '{' {
        // \u{HEX}
        let hex_idx = next_idx + 1; // '{'.len_utf8() == 1
        let end = string[hex_idx..].find('}').ok_or(idx - 1)?;
        let num = &string[hex_idx..hex_idx + end];
        let codepoint = u32::from_str_radix(num, 16).map_err(|_| idx - 1)?;
        char::from_u32(codepoint).ok_or(idx-1).map(|v| (v, end + 1))
    } else {
        // \uNNNN
        // If any of these are non-ASCII, then it's already invalid,
        // so a direct slice is fine
        let next_four = string.get(next_idx .. next_idx + 4).ok_or(idx - 1)?;
        let codepoint = u32::from_str_radix(next_four, 16).map_err(|_| idx - 1)?;
        // Encode the u32
        char::from_u32(codepoint).ok_or(idx - 1).map(|v| (v, 3))
    }
}

// FIXME: This could be factored out along with part of unescape_unicode into its own function.
fn unescape_hex(
    slice: &str,
    idx: usize,
) -> Result<char, usize> {
    // Must be \xNN
    let codepoint = slice.get(idx + 1 .. idx + 3)
        .and_then(|num| u32::from_str_radix(num, 16).ok())
        .ok_or(idx - 1)?;
    char::from_u32(codepoint).ok_or(idx - 1)
}

fn unescape_oct(
    string: &str,
    idx: usize,
) -> Result<(char, usize), usize> {
    // Could be \o, \oo, or \ooo
    let (last_idx, (skip_count, last_digit)) = dbg!(&string[idx..])
        .char_indices()
        .take(3)
        .take_while(|(_, c)| dbg!(c).is_digit(8))
        .enumerate()
        .last()
        .ok_or(idx - 1)?;
    let end_index = idx + last_idx + last_digit.len_utf8();
    let num = &string[idx..end_index];
    let codepoint = u32::from_str_radix(num, 8)
        .map_err(|_| idx - 1)?;
    char::from_u32(codepoint).map(|chr| (chr, skip_count)).ok_or(idx - 1)
}

