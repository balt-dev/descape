[![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/balt-dev/descape/.github%2Fworkflows%2Frust.yml?branch=master&style=flat&label=tests)](https://github.com/balt-dev/descape/actions/)
[![Coverage](https://coveralls.io/repos/github/balt-dev/descape/badge.svg?branch=master)](https://coveralls.io/github/balt-dev/descape/)
[![Documentation](https://docs.rs/descape/badge.svg)](https://docs.rs/descape)
[![MSRV](https://img.shields.io/badge/MSRV-1.52.1-gold)](https://gist.github.com/alexheretic/d1e98d8433b602e57f5d0a9637927e0c)
[![Repository](https://img.shields.io/badge/-GitHub-%23181717?style=flat&logo=github&labelColor=%23555555&color=%23181717)](https://github.com/balt-dev/descape)
[![Latest version](https://img.shields.io/crates/v/descape.svg)](https://crates.io/crates/descape)
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

UTF-8 escapes specified by multiple consecutive `\xNN` escapes will not work as intended, producing [mojibake](https://en.wikipedia.org/wiki/Mojibake).
It's assumed that the escaped data is already UTF-8 encoded.

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

//                           v  invalid at index 7
let invalid_escape = "Uh oh! \\xJJ".to_unescaped();
assert_eq!(
    invalid_escape,
    Err(7)
);
```
