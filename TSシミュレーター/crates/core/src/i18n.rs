//! 表示の言語（日本語・英語）。画面に出す文はコアで作るので、言語もコアで持つ。
//! AIに渡す「事実」は日本語のまま（AIには、選んだ言語で答えさせる）。

/// 表示の言語
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lang {
    #[default]
    Ja,
    En,
}

/// 言語で文を選ぶ。`tr!(lang, "日本語", "English")`、書式つきは `tr!(lang, "手順{}", "Step {}", n)`
macro_rules! tr {
    ($lang:expr, $ja:literal, $en:literal $(,)?) => {
        match $lang {
            $crate::i18n::Lang::Ja => ::std::string::String::from($ja),
            $crate::i18n::Lang::En => ::std::string::String::from($en),
        }
    };
    ($lang:expr, $ja:literal, $en:literal, $($arg:tt)*) => {
        match $lang {
            $crate::i18n::Lang::Ja => format!($ja, $($arg)*),
            $crate::i18n::Lang::En => format!($en, $($arg)*),
        }
    };
}

/// 言語で決まった文を選ぶ（書式の無い短い文）
pub fn pick(lang: Lang, ja: &'static str, en: &'static str) -> &'static str {
    match lang {
        Lang::Ja => ja,
        Lang::En => en,
    }
}
