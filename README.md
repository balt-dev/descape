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
- All escapes mentioned in the documentation of `core::ascii::Char`
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



## Examples

### Parsing an escaped string
```rust
let escaped = "Hello,\\nworld!".to_unescaped();
assert_eq!(
    escaped,
    Ok(Cow::Owned::<'_, str>("Hello,\nworld!".to_string()))
);
```

### Not allocating for a string without escapes
```rust
let no_escapes = "No escapes here!".to_unescaped();
assert_eq!(
    no_escapes,
    Ok(Cow::Borrowed("No escapes here!"))
);
```

### Erroring for invalid escapes
```rust
let invalid_escape = r"Uh oh! \xJJ".to_unescaped();
assert_eq!(
    invalid_escape,
    Err(7)
);
```

### Custom escape handlers
```rust
fn raw(idx: usize, chr: char, _: &mut CharIndices) -> Result<Option<char>, ()> {
    Ok(Some(chr))
}

let escaped = r"\H\e\l\l\o \n \W\o\r\l\d";
let unescaped = escaped.to_unescaped_with(raw).expect("this is fine");
assert_eq!(unescaped, "Hello n World");
```