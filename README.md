[![Repository](https://img.shields.io/badge/repository-GitHub-brightgreen.svg)](https://github.com/balt-dev/descape)
[![Documentation](https://docs.rs/descape/badge.svg)](https://docs.rs/descape)
[![Latest version](https://img.shields.io/crates/v/descape.svg)](https://crates.io/crates/descape)
[![MSRV](https://img.shields.io/badge/MSRV-1.52.1-gold)](https://gist.github.com/alexheretic/d1e98d8433b602e57f5d0a9637927e0c)
[![License](https://img.shields.io/crates/l/descape.svg)](https://github.com/balt-dev/descape/blob/master/LICENSE-MIT)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)

Adds a single extension trait for `&str` to unescape any backslashes. Supports `no-std`.

Unescaping is designed to support as many languages as possible.

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

---

```rust
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
```
