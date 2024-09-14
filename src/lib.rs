//#![no_std]
#![forbid(unsafe_code)]
#![warn(clippy::pedantic, clippy::perf, missing_docs, clippy::panic, clippy::cargo)]
#![allow(clippy::type_complexity)]

/*!

# descape

Provides utilities for easily parsing escape sequences in a string, using `alloc::borrow::Cow` to only borrow when needed.

This library supports many escape sequences:
- All escapes mentioned in the documentation of `core::ascii::Char`
- `\\'` -> `'`
- `\\"` -> `"`
- `\\\`` -> `\``
- `\\\\` -> `\\`
- `\\xNN` -> `\xNN`
- `\\o` -> `\o`, for all octal digits `o`
- `\\oo` -> `\oo`, for all octal digits `o`
- `\\ooo` -> `\ooo`, for all octal digits `o`
- `\\uXXXX` -> `\u{XXXX}`
- `\\u{HEX}` -> `\u{HEX}`

Along with this, you can define your own custom escape handlers! See [`UnescapeExt::to_unescaped_with`] for more information on that.

This crate supports `no-std`.

*/

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
    impl Sealed for str {}
}

/// Extension trait for &str to allow unescaping of strings.
pub trait UnescapeExt: sealed::Sealed {

    /**
    Unescapes a string, returning an [`alloc::borrow::Cow`].
    Will only allocate if the string has any escape sequences.

    Uses [`descape::default_handler`].

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
    /**
    Unescapes a string, using a method to allow custom escape sequences.

    Custom escape handlers are called before parsing any escape sequences,
    and are given 3 arguments:
    - `idx`: The index of the current character (e.g. `Hello\nthere` gets `5`)
    - `chr`: The current character in the string (e.g. `\\n` gets `'n'`)
    - `iter`: A mutable reference to the underlying character iterator -
        use this to get the rest of the string via `CharIndices::as_str`,
        or get the next characters
    
    Handlers return a `Result<Option<char>, ()>`.
    Returning `Ok(Some(char))` replaces the sequence with the given character,
    returning `Ok(None)` removes the sequence entirely,
    and returning `Err(())` errors the unescaping at the current index.
    
    In the case of an error, handlers should return the index of the leading `\\` (i.e. `idx - 1`).

    # Examples
    Permitting any escape, handing it back raw:
    ```rust
    # use descape::UnescapeExt; use std::str::CharIndices;
    fn raw(idx: usize, chr: char, _: &mut CharIndices) -> Result<Option<char>, ()> {
        Ok(Some(chr))
    }
    
    let escaped = r"\H\e\l\l\o \n \W\o\r\l\d";
    let unescaped = escaped.to_unescaped_with(raw).expect("this is fine");
    assert_eq!(unescaped, "Hello n World");
    ```

    Not allowing escape sequences unsupported by Rust:
    ```rust
    # use descape::UnescapeExt; use std::str::CharIndices;
    fn rust_only(idx: usize, chr: char, iter: &mut CharIndices) -> Result<Option<char>, ()> {
        match chr {
            'a' | 'b' | 'v' | 'f' | 'e' | '`' => Err(()),
            _ => descape::default_handler(idx, chr, iter)
        }
    }
    
    r"This is \nfine".to_unescaped_with(rust_only).expect(r"\n is valid");
    r"This is not \fine".to_unescaped_with(rust_only).expect_err(r"\f is invalid");
    ```

    Logging escape indices
    ```rust
    # use descape::UnescapeExt; use std::str::CharIndices;
    let mut escape_indices = Vec::new();
    r"Look at\n all\r these escape\tsequences!".to_unescaped_with(|idx, chr, iter| {
        escape_indices.push(idx);
        descape::default_handler(idx, chr, iter)
    }).expect(r"this is valid");

    assert_eq!(escape_indices, vec![7, 13, 28]);
    ```
    */
    fn to_unescaped_with<'this>(
        &'this self,
        callback: impl for<'iter> FnMut(usize, char, &'iter mut CharIndices<'this>) -> Result<Option<char>, ()>
    ) -> Result<Cow<'_, str>, usize>;
}


impl UnescapeExt for str {
    #[inline]
    fn to_unescaped(&self) -> Result<Cow<str>, usize> {
        self.to_unescaped_with(default_handler)
    }

    // Put this outside to prevent monomorphization bloat
    fn to_unescaped_with<'this>(
        &'this self, 
        mut callback: impl for<'iter> FnMut(usize, char, &'iter mut CharIndices<'this>) -> Result<Option<char>, ()>
    ) -> Result<Cow<'this, str>, usize> {
        to_unescaped_with_mono(self, &mut callback)
    }
}

fn to_unescaped_with_mono<'this, 'cb>(
    this: &'this str,
    mut callback: &'cb mut dyn for<'iter> FnMut(usize, char, &'iter mut CharIndices<'this>) -> Result<Option<char>, ()>
) -> Result<Cow<'this, str>, usize> {
    // Iterates over each character as a UTF-8 string slice
    let mut iter = this.char_indices();
    let mut seen: &'this str = "";
    let mut owned = None::<String>;

    while let Some((idx, chr)) = iter.next() {
        if chr != '\\' {
            if let Some(owned) = &mut owned {
                owned.push(chr);
            } else {
                seen = &this[..idx + chr.len_utf8()];
            }
            continue;
        }
        let owned = owned.get_or_insert_with(|| {
            let mut string = seen.to_string();
            string.reserve_exact(this.len() - seen.len());
            string
        });
        if let Some((_, chr)) = iter.next() {
            if let Some(res) = callback(idx, chr, &mut iter).map_err(|_| idx)? {
                owned.push(res);
                continue;
            }
        } else {
            // No matches found
            return Err(owned.len());
        }
    }

    match owned {
        Some(string) => Ok(Cow::Owned(string)),
        None => Ok(Cow::Borrowed(this)),
    }
}

/// The default escape sequence handler. 
///
/// Meant to be passed to `UnescapeExt::to_unescaped_with`, or used in the definition of a custom one.
///
/// The following escapes are valid:
//     - All escapes mentioned in the documentation of `core::ascii::Char`
//     - `\\'` -> `'`
//     - `\\"` -> `"`
//     - `\\\`` -> `\``
//     - `\\\\` -> `\\`
//     - `\\xNN` -> `\xNN`
//     - `\\o` -> `\o`, for all octal digits `o`
//     - `\\oo` -> `\oo`, for all octal digits `o`
//     - `\\ooo` -> `\ooo`, for all octal digits `o`
//     - `\\uXXXX` -> `\u{XXXX}`
//     - `\\u{HEX}` -> `\u{HEX}`
//
// # Errors
//
// Errors if there's an invalid escape sequence in the string.
// Passes back the byte index of the invalid character.
pub fn default_handler(_: usize, chr: char, iter: &mut CharIndices) -> Result<Option<char>, ()> {
    Ok( match chr {
        'a' => Some('\x07'),
        'b' => Some('\x08'),
        't' => Some('\x09'),
        'n' => Some('\x0A'),
        'v' => Some('\x0B'),
        'f' => Some('\x0C'),
        'r' => Some('\x0D'),
        'e' => Some('\x1B'),
        '`' => Some('`'),
        '\'' => Some('\''),
        '"' => Some('"'),
        '\\' => Some('\\'),
        'u' => {
            let (chr, skip) = unescape_unicode(iter).ok_or(())?;
            // Skip the needed amount of characters
            for _ in 0..skip { iter.next(); }
            Some(chr)
        },
        'x' => {
            // Skip two characters
            let res = unescape_hex(iter).ok_or(())?;
            iter.next();
            iter.next();
            Some(res)
        },
        c if c.is_digit(8) => {
            let (chr, skip) = unescape_oct(c, iter).ok_or(())?;
            for _ in 0..skip { iter.next(); }
            Some(chr)
        },
        _ => return Err(()),
    } )
}

fn unescape_unicode(
    iter: &mut CharIndices
) -> Option<(char, usize)> {
    let string = iter.as_str();
    let (_, next) = iter.next()?;
    if next == '{' {
        // \u{HEX}
        let end = string[1 ..].find('}')?;
        let num = &string[1 ..= end];
        let codepoint = u32::from_str_radix(num, 16).ok()?;
        char::from_u32(codepoint).map(|v| (v, end + 1))
    } else {
        // \uNNNN
        // If any of these are non-ASCII, then it's already invalid,
        // so a direct slice is fine
        let next_four = string.get( ..4 )?;
        let codepoint = u32::from_str_radix(next_four, 16).ok()?;
        // Encode the u32
        char::from_u32(codepoint).map(|v| (v, 3))
    }
}

// FIXME: This could be factored out along with part of unescape_unicode into its own function.
fn unescape_hex(
    iter: &mut CharIndices
) -> Option<char> {

    // Must be \xNN
    let codepoint = iter.as_str()
        .get(..2)
        .and_then(|num| u32::from_str_radix(num, 16).ok())?;
    char::from_u32(codepoint)
}

fn unescape_oct(
    chr: char,
    iter: &mut CharIndices
) -> Option<(char, usize)> {

    // Could be \o, \oo, or \ooo
    let str = iter.as_str();
    let end = iter.clone() // Cloning this is pretty cheap
        .take(2)
        .take_while(|(_, c)| c.is_digit(8))
        .enumerate()
        .last()
        .map(|(idx, _)| idx + 1)
        .unwrap_or(0);
    let num = &str[ .. end];
    // These are the characters _after_ the first
    let mut codepoint = if num.is_empty() { 0 } else { u32::from_str_radix(num, 8).ok()? };
    // Add the first character at the top of the number
    codepoint += (chr as u32 - '0' as u32) * 8u32.pow(end as u32);
    char::from_u32(codepoint).map(|chr| (chr, end))
}

