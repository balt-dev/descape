[![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/balt-dev/descape/.github%2Fworkflows%2Frust.yml?branch=master&style=flat&label=tests)](https://github.com/balt-dev/descape/actions/)
[![Coverage](https://coveralls.io/repos/github/balt-dev/descape/badge.svg?branch=master)](https://coveralls.io/github/balt-dev/descape/)
[![Documentation](https://docs.rs/descape/badge.svg)](https://docs.rs/descape)
[![MSRV](https://img.shields.io/badge/MSRV-1.52.1-gold)](https://gist.github.com/alexheretic/d1e98d8433b602e57f5d0a9637927e0c)
[![Repository](https://img.shields.io/badge/-GitHub-%23181717?style=flat&logo=github&labelColor=%23555555&color=%23181717)](https://github.com/balt-dev/descape)
[![Latest version](https://img.shields.io/crates/v/descape.svg)](https://crates.io/crates/descape)
[![License](https://img.shields.io/crates/l/descape.svg)](https://github.com/balt-dev/descape/blob/master/LICENSE-MIT)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)


# descape

Provides utilities for easily parsing escape sequences in a string, using `alloc::borrow::Cow` to only borrow when needed.

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

Along with this, you can define your own custom escape handlers! See `UnescapeExt::to_unescaped_with` for more information on that.

This crate supports `no-std`.

Optionally, this crate has the `std` and `core_error` features, 
to allow the error type of an invalid escape to implement the `Error` trait.

`std` uses `std::error::Error`, and `core_error` depends on `core::error::Error`, which is stable on Rust 1.82.0 or greater.

## Examples

### Parsing an escaped string
```rust
let escaped = "Hello,\\nworld!".to_unescaped();
assert_eq!(
    escaped.unwrap(),
    Cow::Owned::<'_, str>("Hello,\nworld!".to_string())
);
```

### Not allocating for a string without escapes
```rust
let no_escapes = "No escapes here!".to_unescaped();
assert_eq!(
    no_escapes.unwrap(),
    Cow::Borrowed("No escapes here!")
);
```

### Erroring for invalid escapes
```rust
//                            v  invalid at index 7
let invalid_escape = r"Uh oh! \xJJ".to_unescaped();
assert_eq!(
    invalid_escape.unwrap_err().index,
    7
);
```

### Permitting any escape, handing it back raw
```rust
fn raw(idx: usize, chr: char, _: &mut CharIndices) -> Result<Option<char>, ()> {
    Ok(Some(chr))
}

let escaped = r"\H\e\l\l\o \n \W\o\r\l\d";
let unescaped = escaped.to_unescaped_with(raw).expect("this is fine");
assert_eq!(unescaped, "Hello n World");
```

### Removing escape sequences entirely
```rust
fn raw(idx: usize, chr: char, _: &mut CharIndices) -> Result<Option<char>, ()> {
    Ok(None)
}

let escaped = r"What if I want a \nnewline?";
let unescaped = escaped.to_unescaped_with(raw).expect("this should work");
assert_eq!(unescaped, "What if I want a newline?");
```

### Not allowing escape sequences unsupported by Rust
```rust
fn rust_only(idx: usize, chr: char, iter: &mut CharIndices) -> Result<Option<char>, ()> {
    match chr {
        'a' | 'b' | 'v' | 'f' | 'e' | '`' => Err(()),
        _ => descape::DefaultHandler.escape(idx, chr, iter)
    }
}

r"This is \nfine".to_unescaped_with(rust_only).expect(r"\n is valid");
r"This is not \fine".to_unescaped_with(rust_only).expect_err(r"\f is invalid");
```