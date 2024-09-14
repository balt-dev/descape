use std::borrow::Cow;
use std::str::CharIndices;
use descape::UnescapeExt;

macro_rules! ensure_err {
    ($($name: ident),+) => {$(
        assert_eq!($name.to_unescaped(), Err(0), "{} parsed successfully when invalid", stringify!($name));
    )+};
}

#[test]
fn test_escapes() {
    static ESCAPED: &str =
        r#"\a \b \t \n \v❤️\f \r \e \' \" \` \\ \u{0} \u{21}❤️\u{433} \u{FFFD} \u0000 \u0021 \uFFFD \x7E \xFF \0 \11 \100"#;
    static UNESCAPED: &str =
        "\x07 \x08 \t \n \x0B❤️\x0C \x0D \x1B \' \" ` \\ \u{0} \u{21}❤️\u{433} \u{FFFD} \u{0000} \u{0021} \u{FFFD} \x7E \u{FF} \0 \t @";
    static NO_ESCAPES: &str = "No escapes here!";
    static BAD_ESCAPE: &str = r"\Z";
    static CUT_ESCAPE: &str = r"\";
    static BAD_UNICODE: &str = r"\u{This is definitely not hexadecimal}";
    static EMPTY_UNICODE: &str = r"\u{}";
    static CUT_UNICODE: &str = r"\u{03";
    static BAD_HEX: &str = r"\xGG";
    static CUT_HEX: &str = r"\xA";
    static EMPTY_HEX: &str = r"\x";
    static NON_UNICODE: &str = r"\u{D800}";

    assert_eq!(
        ESCAPED.to_unescaped()
            .map_err(|idx| &ESCAPED[..idx])
            .expect("should not reject legal escaped string"),
        Cow::Owned::<'_, str>(UNESCAPED.to_string())
    );

    assert_eq!(
        NO_ESCAPES.to_unescaped()
            .map_err(|idx| &ESCAPED[..idx])
            .expect("should not reject legal escaped string"),
        Cow::Borrowed(NO_ESCAPES)
    );

    ensure_err!(
        BAD_ESCAPE,
        CUT_ESCAPE,
        BAD_UNICODE,
        EMPTY_UNICODE,
        CUT_UNICODE,
        BAD_HEX,
        CUT_HEX,
        EMPTY_HEX,
        NON_UNICODE
    );
}

fn custom_esc(_: usize, chr: char, iter: &mut CharIndices<'_>) -> Result<Option<char>, ()> {
    if chr == 'T' {
        let (_, next) = iter.next().ok_or(())?;
        return Ok(Some(match next {
            'a' => 'g',
            'o' => 'p',
            _ => Err(())?
        }));
    }
    Ok(None)
}

#[test]
fn test_customs() {
    r"Hello \T world".to_unescaped_with(custom_esc)
        .expect_err(r"custom escape should fail for \T");
    r"Foo \Tg bar".to_unescaped_with(custom_esc)
        .expect_err(r"custom escape should fail for \Tg");
    assert_eq!(
        r"Spam E\Tags".to_unescaped_with(custom_esc).expect(r"custom escape should succeed for \Ta"),
        Cow::<'static, str>::Owned(String::from("Spam Eggs")),
        "custom escape gave incorrect result"
    );
    assert_eq!(
        r"Bee\To \n Boop".to_unescaped_with(custom_esc).expect(r"custom escape should succeed for \To"),
        Cow::<'static, str>::Owned(String::from("Beep  Boop")),
        "custom escape gave incorrect result"
    );
}