#![no_std]
#![forbid(unsafe_code)]
#![warn(clippy::pedantic, clippy::perf, missing_docs, clippy::panic, clippy::cargo)]
#![allow(clippy::type_complexity)]
#![cfg_attr(docsrs, feature(doc_cfg))]


/*!

# descape

Provides utilities for easily parsing escape sequences in a string via [`UnescapeExt`], using [`alloc::borrow::Cow`] to only borrow when needed.

This library supports many escape sequences:
- `\\a` -> `\x07`
- `\\b` -> `\x08`
- `\\t` -> `\x09`
- `\\n` -> `\x0A`
- `\\v` -> `\x0B`
- `\\f` -> `\x0C`
- `\\r` -> `\x0D`
- `\\e` -> `\x1B`
- `\\'` -> `'`
- `\\"` -> `"`
- <code>&bsol;&bsol;&grave;</code> -> <code>&grave;</code>
- `\\\\` -> `\\`
- `\\xNN` -> `\xNN`
- `\\o` -> `\o`, for all octal digits `o`
- `\\oo` -> `\oo`, for all octal digits `o`
- `\\ooo` -> `\ooo`, for all octal digits `o`
- `\\uXXXX` -> `\u{XXXX}`
- `\\u{HEX}` -> `\u{HEX}`

Along with this, you can define your own custom escape handlers! See [`UnescapeExt::to_unescaped_with`] for more information on that.

This crate supports `no-std`.

Optionally, this crate has the `std` and `core_error` features, 
to allow the error type of an invalid escape to implement the `Error` trait.

`std` uses `std::error::Error`, and `core_error` depends on `core::error::Error`, which is stable on Rust 1.82.0 or greater.

*/


#[cfg(any(feature = "std", docsrs))]
extern crate std;
#[cfg(any(feature = "std", docsrs))]
use std::error::Error as ErrorTrait;
#[cfg(all(feature = "core_error", not(feature = "std")))]
use core::error::Error as ErrorTrait;

extern crate alloc;

use alloc::{
    borrow::Cow,
    string::String,
    str::CharIndices
};

mod sealed {
    pub trait Sealed {}
    impl Sealed for str {}
}


#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
/// An error representing an invalid escape sequence in a string.
pub struct InvalidEscape {
    /// The index of the invalid escape sequence.
    pub index: usize,
}

impl InvalidEscape {
    /// Constructs an invalid escape error from an index.
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self { index }
    }
}

impl core::fmt::Display for InvalidEscape {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "invalid escape sequence at index {}", self.index)?;
        Ok(())
    }
}

#[cfg_attr(docsrs, doc(cfg(any(feature = "std", feature = "core_error"))))]
#[cfg(any(feature = "std", feature = "core_error", docsrs))]
impl ErrorTrait for InvalidEscape {}

/// An enum defining all possible non-error returns for a macro.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EscapeValue<'s> {
    /// Remove the escape sequence entirely.
    Remove,
    /// Leave the escape sequence as is.
    Skip,
    /// Replace the escape sequence with a single character.
    Character(char),
    /// Replace the escape sequence with a string of characters.
    String(Cow<'s, str>)
}

impl From<char> for EscapeValue<'static> {
    fn from(value: char) -> Self {
        Self::Character(value)
    }
}

impl<'string> From<&'string str> for EscapeValue<'string> {
    fn from(value: &'string str) -> Self {
        Self::String(Cow::Borrowed(value))
    }
}

impl<'string> From<Cow<'string, str>> for EscapeValue<'string> {
    fn from(value: Cow<'string, str>) -> Self {
        Self::String(value)
    }
}

impl<'string> From<String> for EscapeValue<'string> {
    fn from(value: String) -> Self {
        Self::String(Cow::Owned(value))
    }
}

/// A trait distinguishing an object as a handler for custom escape sequences.
/// 
/// For convenience, this trait is **automatically implemented** for all implementors of `FnMut` with the correct signature.
/// 
pub trait EscapeHandler {
    /// Definition of a custom escape handler.
    /// 
    /// Custom escape handlers are called before parsing any escape sequences,
    /// and are given 3 arguments:
    /// - `idx`: The index of the current character (e.g. `Hello\nthere` gets `5`)
    /// - `chr`: The current character in the string (e.g. `\\n` gets `'n'`)
    /// - `iter`: A mutable reference to the underlying character iterator -
    ///     use this to get the rest of the string via `CharIndices::as_str`,
    ///     or get the next characters
    /// 
    /// Handlers return a `Result<Option<char>, ()>`.
    /// Returning `Ok(Some(char))` replaces the sequence with the given character,
    /// returning `Ok(None)` removes the sequence entirely,
    /// and returning `Err` errors the unescaping at the index of the escape sequence.
    /// 
    /// 
    /// # Examples

    /// ## Permitting any escape, handing it back raw
    /// ```rust
    /// # use descape::*; use std::str::CharIndices;
    /// fn raw<'i, 's>(idx: usize, chr: char, iter: &'i mut CharIndices<'s>) -> Result<EscapeValue<'s>, ()> {
    ///     Ok(chr.into())
    /// }
    
    /// let escaped = r"\H\e\l\l\o \n \W\o\r\l\d";
    /// let unescaped = escaped.to_unescaped_with(raw).unwrap();
    /// assert_eq!(unescaped, "Hello n World");
    /// ```

    /// ## Removing escape sequences entirely
    /// ```rust
    /// # use descape::*; use std::str::CharIndices;
    /// fn raw<'i, 's>(idx: usize, chr: char, iter: &'i mut CharIndices<'s>) -> Result<EscapeValue<'s>, ()> {
    ///     Ok(EscapeValue::Remove)
    /// }

    /// let escaped = r"What if I want a \nnewline?";
    /// let unescaped = escaped.to_unescaped_with(raw).unwrap();
    /// assert_eq!(unescaped, "What if I want a newline?");
    /// ```

    /// ## Replacing escape sequences with their character number
    /// ```rust
    /// # use descape::*; use std::str::CharIndices;
    /// fn charnum<'i, 's>(idx: usize, chr: char, iter: &'i mut CharIndices<'s>) -> Result<EscapeValue<'s>, ()> {
    ///     Ok(format!("[{:02X}]", chr as u32).into())
    /// }

    /// let escaped = r"Well this\Ais a\Zthing.";
    /// let unescaped = escaped.to_unescaped_with(charnum).unwrap();
    /// assert_eq!(unescaped, "Well this[41]is a[5A]thing.");
    /// ```

    /// ## Not allowing escape sequences unsupported by Rust
    /// ```rust
    /// # use descape::*; use std::str::CharIndices;
    /// fn rust_only<'i, 's>(idx: usize, chr: char, iter: &'i mut CharIndices<'s>) -> Result<EscapeValue<'s>, ()> {
    ///     match chr {
    ///         'a' | 'b' | 'v' | 'f' | 'e' | '`' => Err(()),
    ///         _ => descape::DefaultEscapeHandler.escape(idx, chr, iter)
    ///     }
    /// }
    
    /// r"This is \nfine".to_unescaped_with(rust_only).expect(r"\n is valid");
    /// r"This is not \fine".to_unescaped_with(rust_only).expect_err(r"\f is invalid");
    /// ```
    
    /// # An informal note
    /// Ideally, this trait would return `Result<EscapeValue<'source>, Option<Box<dyn Error>>>`, but `Error` has only been in `core`
    /// since Rust version `1.82.0`. Using it would bump the MSRV by a tremendous amount,
    /// and as such it has been left out.
    #[allow(clippy::result_unit_err, clippy::missing_errors_doc)]
    fn escape<'iter, 'source>(&mut self, idx: usize, chr: char, iter: &'iter mut CharIndices<'source>) -> Result<EscapeValue<'source>, ()>;
    /// The escape prefix to use for this handler.
    /// Defaults to `\\`.
    ///
    /// # Examples
    /// ```rust
    /// # use descape::*; use std::str::CharIndices;
    /// struct PercentEscape;
    /// impl EscapeHandler for PercentEscape {
    ///     fn prefix(&self) -> char { '%' }
    ///     fn escape<'iter, 'source>(&mut self, idx: usize, chr: char, iter: &'iter mut CharIndices<'source>) -> Result<EscapeValue<'source>, ()> {
    ///         descape::DefaultEscapeHandler.escape(idx, chr, iter)
    ///     }
    /// }
    ///
    /// assert_eq!(
    ///     r"Hello,%tworld!".to_unescaped_with(PercentEscape).unwrap(),
    ///     "Hello,\tworld!"
    /// )
    /// ```
    fn prefix(&self) -> char { '\\' }
}

impl<F> EscapeHandler for F 
    where F: for<'iter, 'source> FnMut(usize, char, &'iter mut CharIndices<'source>) -> Result<EscapeValue<'source>, ()>
{
    fn escape<'iter, 'source>(&mut self, idx: usize, chr: char, iter: &'iter mut CharIndices<'source>) -> Result<EscapeValue<'source>, ()> {
        self(idx, chr, iter)
    }
}

/// An extension trait for [`&str`](str) to allow parsing escape sequences in strings, only copying when needed.
pub trait UnescapeExt: sealed::Sealed {

    /**
    Unescapes a string, returning an [`alloc::borrow::Cow`].
    Will only allocate if the string has any escape sequences.

    Uses [`crate::DefaultHandler`].

    # Errors
    Errors if there's an invalid escape sequence in the string.
    Passes back the byte index of the invalid character.
     */
    fn to_unescaped(&self) -> Result<Cow<'_, str>, InvalidEscape>;
    /**
    Unescapes a string using a custom escape handler. See the documentation of [`crate::EscapeHandler`] for more details.

    # Errors

    Errors if there's an invalid escape sequence in the string.
    Passes back the byte index of the invalid character.

    */
    fn to_unescaped_with(
        &self,
        callback: impl EscapeHandler
    ) -> Result<Cow<'_, str>, InvalidEscape>;
}


impl UnescapeExt for str {
    #[inline]
    fn to_unescaped(&'_ self) -> Result<Cow<'_, str>, InvalidEscape> {
        self.to_unescaped_with(DefaultEscapeHandler)
    }

    // Put this outside to prevent monomorphization bloat
    fn to_unescaped_with(
        &'_ self,
        mut callback: impl EscapeHandler
    ) -> Result<Cow<'_, str>, InvalidEscape> {
        to_unescaped_with_mono(self, &mut callback)
    }
}

fn to_unescaped_with_mono<'this, 'cb>(
    this: &'this str,
    callback: &'cb mut dyn EscapeHandler
) -> Result<Cow<'this, str>, InvalidEscape> {
    // Iterates over each character as a UTF-8 string slice
    let mut iter = this.char_indices();
    let mut seen: &'this str = "";
    let mut owned = None::<String>;
    let prefix = callback.prefix();

    while let Some((index, chr)) = iter.next() {
        if chr != prefix {
            if let Some(owned) = &mut owned {
                owned.push(chr);
            } else {
                seen = &this[..index + chr.len_utf8()];
            }
            continue;
        }
        if let Some((i, chr)) = iter.next() {
            let res = callback.escape(index, chr, &mut iter)
                .map_err(|()| InvalidEscape { index })?;
            if res == EscapeValue::Skip && owned.is_none() {
                seen = &this[..i + chr.len_utf8()];
                continue;
            }
            let owned = owned.get_or_insert_with(|| seen.into());
            match res {
                EscapeValue::Character(c) => { owned.push(c); continue; },
                EscapeValue::String(str) => { owned.push_str(&*str); continue; },
                EscapeValue::Remove => { continue; },
                EscapeValue::Skip => { owned.push(prefix); owned.push(chr); continue; }
            }
        } else {
            // No matches found
            return Err(InvalidEscape::new(index));
        }
    }

    match owned {
        Some(string) => Ok(Cow::Owned(string)),
        None => Ok(Cow::Borrowed(this)),
    }
}

/// The default escape sequence handler. 
///
/// The following escapes are valid:
/// - `\\a` -> `\x07`
/// - `\\b` -> `\x08`
/// - `\\t` -> `\x09`
/// - `\\n` -> `\x0A`
/// - `\\v` -> `\x0B`
/// - `\\f` -> `\x0C`
/// - `\\r` -> `\x0D`
/// - `\\e` -> `\x1B`
/// - `\\'` -> `'`
/// - `\\"` -> `"`
/// - <code>&bsol;&bsol;&grave;</code> -> <code>&grave;</code>
/// - `\\\\` -> `\\`
/// - `\\xNN` -> `\xNN`
/// - `\\o` -> `\o`, for all octal digits `o`
/// - `\\oo` -> `\oo`, for all octal digits `o`
/// - `\\ooo` -> `\ooo`, for all octal digits `o`
/// - `\\uXXXX` -> `\u{XXXX}`
/// - `\\u{HEX}` -> `\u{HEX}`
///
pub struct DefaultEscapeHandler;

impl EscapeHandler for DefaultEscapeHandler {
    fn escape<'iter, 'source>(&mut self, _idx: usize, chr: char, iter: &'iter mut CharIndices<'source>) -> Result<EscapeValue<'source>, ()> {
        Ok( match chr {
            'a' => '\x07'.into(),
            'b' => '\x08'.into(),
            't' => '\x09'.into(),
            'n' => '\x0A'.into(),
            'v' => '\x0B'.into(),
            'f' => '\x0C'.into(),
            'r' => '\x0D'.into(),
            'e' => '\x1B'.into(),
            '`' => '`'.into(),
            '\'' => '\''.into(),
            '"' => '"'.into(),
            '\\' => '\\'.into(),
            'u' => {
                let (chr, skip) = unescape_unicode(iter).ok_or(())?;
                // Skip the needed amount of characters
                for _ in 0..skip { iter.next(); }
                chr.into()
            },
            'x' => {
                // Skip two characters
                let res = unescape_hex(iter).ok_or(())?;
                iter.next();
                iter.next();
                res.into()
            },
            c if c.is_digit(8) => {
                let (chr, skip) = unescape_oct(c, iter).ok_or(())?;
                for _ in 0..skip { iter.next(); }
                chr.into()
            },
            _ => return Err(()),
        } )
    }
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

#[allow(clippy::cast_possible_truncation)] // Can't actually happen
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
        .map_or(0, |(idx, _)| idx + 1);
    let num = &str[ .. end];
    // These are the characters _after_ the first
    let mut codepoint = if num.is_empty() { 0 } else { u32::from_str_radix(num, 8).ok()? };
    // Add the first character at the top of the number
    codepoint += (chr as u32 - '0' as u32) * 8u32.pow(end as u32);
    char::from_u32(codepoint).map(|chr| (chr, end))
}

