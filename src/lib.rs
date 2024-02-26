#![no_std]
#![forbid(unsafe_code)]
#![warn(clippy::pedantic, clippy::perf, missing_docs, clippy::panic, clippy::cargo)]

//! Provides a function for simply unescaping a string.
//!
//! Designed with [`char::escape_default`] in mind,
//! but should also be compatible with strings from most languages.
//!
//! Will not pull in panicking code in release mode.

extern crate alloc;
use alloc::{
    borrow::Cow,
    string::{String, ToString},
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
        let mut iter = self.char_indices();
        let len = self.len();
        let mut owned = None::<String>;

        while let Some((i, c)) = iter.next() {
            if c != '\\' {
                if let Some(owned) = &mut owned {
                    owned.push(c);
                }
                continue;
            }
            let owned = owned.get_or_insert_with(|| self[..i].to_string());
            match iter.next().map(|(_, c)| c) {
                Some('b') => owned.push('\x08'),
                Some('f') => owned.push('\x0C'),
                Some('n') => owned.push('\n'),
                Some('t') => owned.push('\t'),
                Some('r') => owned.push('\r'),
                Some('\'') => owned.push('\''),
                Some('"') => owned.push('"'),
                Some('\\') => owned.push('\\'),
                Some('u') => owned.push(unescape_unicode(&mut iter, len)?),
                Some('x') => owned.push(unescape_hex(&mut iter, len)?),
                Some(c) if c.is_digit(8) => owned.push(unescape_oct(c, i, &mut iter, len)?),
                _ => return Err(i),
            }
        }

        match owned {
            Some(string) => Ok(Cow::Owned(string)),
            None => Ok(Cow::Borrowed(self)),
        }
    }
}

fn get_hex(mut iter: impl Iterator<Item = (usize, char)>, len: usize) -> Result<char, usize> {
    if let Some((idx, c)) = iter.next() {
        if !c.is_ascii_hexdigit() {
            return Err(idx);
        }
        return Ok(c);
    }
    Err(len)
}

fn unescape_unicode(
    mut iter: impl Iterator<Item = (usize, char)>,
    len: usize,
) -> Result<char, usize> {
    let (start_idx, next) = iter.next().ok_or(len)?;
    match next {
        '{' => {
            // \u{HEX}
            let mut acc = String::new();
            loop {
                let (idx, c) = iter.next().ok_or(len)?;
                match c {
                    '}' => {
                        let codepoint = u32::from_str_radix(&acc, 16).map_err(|_| start_idx)?;
                        return char::from_u32(codepoint).ok_or(idx);
                    }
                    c if c.is_ascii_hexdigit() => acc.push(c),
                    _ => return Err(idx),
                }
            }
        }
        _ if next.is_ascii_hexdigit() => {
            // \uNNNN
            let chars = <[char; 4]>::into_iter([
                next,
                get_hex(&mut iter, len)?,
                get_hex(&mut iter, len)?,
                get_hex(&mut iter, len)?,
            ])
            .collect::<String>();
            // FIXME:
            //     We checked earlier in get_hex that the string is a valid hexadecimal string.
            //     This will always be correct.
            //     This could be an unwrap_unchecked()? Unsure if adding unsafe is worth it.
            let codepoint = u32::from_str_radix(&chars, 16).map_err(|_| start_idx)?;
            char::from_u32(codepoint).ok_or(len)
        },
        _ => Err(start_idx)
    }
}

// FIXME: This could be factored out along with part of unescape_unicode into its own function.
fn unescape_hex(mut iter: impl Iterator<Item = (usize, char)>, len: usize) -> Result<char, usize> {
    // Must be \xNN
    let (start_idx, c) = iter.next().ok_or(len)?;
    if !c.is_ascii_hexdigit() {
        return Err(start_idx);
    }
    let chars = <[char; 2]>::into_iter([c, get_hex(iter, len)?]).collect::<String>();
    // FIXME: See unescape_unicode.
    let codepoint = u32::from_str_radix(&chars, 16).map_err(|_| start_idx)?;
    char::from_u32(codepoint).ok_or(len)
}

fn unescape_oct(
    start: char,
    idx: usize,
    iter: &mut core::str::CharIndices<'_>,
    len: usize
) -> Result<char, usize> {
    // Could be \o, \oo, or \ooo
    let oct_string = core::iter::once(start)
        .chain(
            iter.as_str()
            .chars()
            .take(2)
            .take_while(|c| c.is_digit(8))
        ).collect::<String>();
    // FIXME: See unescape_unicode.
    let codepoint = u32::from_str_radix(&oct_string, 8).map_err(|_| idx)?;
    char::from_u32(codepoint).ok_or(len)
}
