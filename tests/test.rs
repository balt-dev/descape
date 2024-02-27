static ESCAPED: &str =
    r#"\b \f \n \t \r \' \" \\ \u{0} \u{21} \u{433} \u{FFFD} \u0000 \u0021 \uFFFD \x7E \xFF"#;
static UNESCAPED: &str =
    "\x08 \x0C \n \t \r \' \" \\ \u{0} \u{21} \u{433} \u{FFFD} \u{0000} \u{0021} \u{FFFD} \x7E \u{FF}";

static BAD_ESCAPE: &str = r"\l";
static BAD_UNIC: &str = r"\u{This is definitely not hexadecimal}";
static EMPTY_UNIC: &str = r"\u{}";
static CUT_UNIC: &str = r"\u{03";
static BAD_HEX: &str = r"\xGG";
static CUT_HEX: &str = r"\xA";
static EMPTY_HEX: &str = r"\x";
static NON_UNIC: &str = r"\u{D800}";


use descape::UnescapeExt;

macro_rules! ensure_err {
    ($($name: ident),+) => {$(
        assert_eq!($name.to_unescaped(), Err(0), "{} parsed successfully when invalid", stringify!($name));
    )+};
}

#[test]
fn test_escapes() {
    assert_eq!(
        ESCAPED.to_unescaped()
            .expect("should not reject legal escaped string"),
        UNESCAPED
    );

    ensure_err!(
        BAD_ESCAPE,
        BAD_UNIC,
        EMPTY_UNIC,
        CUT_UNIC,
        BAD_HEX,
        CUT_HEX,
        EMPTY_HEX,
        NON_UNIC
    );
}
