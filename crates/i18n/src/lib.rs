//! 后端用户可见文案的语言选择。
//!
//! 报错、状态这类文案由后端生成、界面原样显示，所以必须跟随界面语言。应用在启动和
//! 用户切换语言时调用 [`set`]；各 crate 用 [`tr!`] 一次写出中、英、日三份，按当前语言取用。
//! 未设置时为英文，与界面的默认语言一致。

use std::sync::atomic::{AtomicU8, Ordering};

/// 界面支持的语言。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Zh,
    Ja,
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

/// 设置后端文案的语言（进程内全局）。已经生成的文案不会改变。
pub fn set(lang: Lang) {
    let v = match lang {
        Lang::En => 0,
        Lang::Zh => 1,
        Lang::Ja => 2,
    };
    CURRENT.store(v, Ordering::Relaxed);
}

/// 当前语言。
pub fn current() -> Lang {
    match CURRENT.load(Ordering::Relaxed) {
        1 => Lang::Zh,
        2 => Lang::Ja,
        _ => Lang::En,
    }
}

/// 界面保存的语言代码（`settings.json` 的 `lang`）。未知取值一律按英文，与界面的 `resolveLang` 一致。
pub fn resolve(code: &str) -> Lang {
    match code {
        "zh" => Lang::Zh,
        "ja" => Lang::Ja,
        _ => Lang::En,
    }
}

/// 按当前语言格式化文案：`tr!(中文, English, 日本語, 参数…)`，返回 `String`。
///
/// 三个格式串必须是字面量，用法与 `format!` 相同（可以内联捕获变量，也可以跟位置参数）；
/// 三份都要用到全部参数，漏掉的会编译失败。
#[macro_export]
macro_rules! tr {
    ($zh:literal, $en:literal, $ja:literal $(, $arg:expr)* $(,)?) => {
        match $crate::current() {
            $crate::Lang::Zh => ::std::format!($zh $(, $arg)*),
            $crate::Lang::En => ::std::format!($en $(, $arg)*),
            $crate::Lang::Ja => ::std::format!($ja $(, $arg)*),
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_codes_like_the_ui() {
        assert_eq!(resolve("zh"), Lang::Zh);
        assert_eq!(resolve("ja"), Lang::Ja);
        assert_eq!(resolve("en"), Lang::En);
        assert_eq!(resolve("fr"), Lang::En);
        assert_eq!(resolve(""), Lang::En);
    }

    // 全局语言只在这一个测试里改，避免并行测试互相影响
    #[test]
    fn follows_the_current_language() {
        assert_eq!(current(), Lang::En, "默认英文");
        let n = 3;
        let e = "x";
        // 内联捕获和位置参数两种写法
        let inline = || tr!("中 {n} {e}", "en {n} {e}", "日 {n} {e}");
        let positional = || tr!("中 {} {}", "en {} {}", "日 {} {}", n, e);
        assert_eq!(inline(), "en 3 x");
        assert_eq!(positional(), "en 3 x");
        set(Lang::Zh);
        assert_eq!(inline(), "中 3 x");
        set(Lang::Ja);
        assert_eq!(positional(), "日 3 x");
        set(Lang::En);
        assert_eq!(inline(), "en 3 x");
    }
}
