//! 托盘菜单与系统通知的三语文案（与界面语言 `en` / `zh` / `ja` 一致）。

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    En,
    Zh,
    Ja,
}

/// 未知取值一律按英文处理（与 UI 的 `resolveLang` 一致）。
pub fn resolve_lang(s: &str) -> Lang {
    match s {
        "zh" => Lang::Zh,
        "ja" => Lang::Ja,
        _ => Lang::En,
    }
}

pub struct TrayText {
    pub open: &'static str,
    pub quit: &'static str,
}

pub fn tray_text(lang: Lang) -> TrayText {
    match lang {
        Lang::En => TrayText {
            open: "Open settings",
            quit: "Quit",
        },
        Lang::Zh => TrayText {
            open: "打开设置",
            quit: "退出",
        },
        Lang::Ja => TrayText {
            open: "設定を開く",
            quit: "終了",
        },
    }
}

pub fn install_update_label(lang: Lang, version: &str) -> String {
    match lang {
        Lang::En => format!("Install update v{version}…"),
        Lang::Zh => format!("安装更新 v{version}…"),
        Lang::Ja => format!("アップデート v{version} をインストール…"),
    }
}

/// 发现新版本时的系统通知：（标题，正文）。
// 更新检查（后续任务）接入前暂未被调用。
#[allow(dead_code)]
pub fn update_notification(lang: Lang, version: &str) -> (String, String) {
    match lang {
        Lang::En => (
            format!("halokey v{version} is available"),
            "Open Settings to install it.".into(),
        ),
        Lang::Zh => (
            format!("halokey v{version} 可以更新了"),
            "打开设置即可安装。".into(),
        ),
        Lang::Ja => (
            format!("halokey v{version} が利用可能です"),
            "設定を開いてインストールしてください。".into(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_language_falls_back_to_english() {
        assert!(matches!(resolve_lang("fr"), Lang::En));
        assert!(matches!(resolve_lang("ja"), Lang::Ja));
        assert_eq!(tray_text(Lang::Zh).open, "打开设置");
        assert_eq!(tray_text(Lang::En).quit, "Quit");
        assert_eq!(tray_text(Lang::Ja).open, "設定を開く");
        assert_eq!(
            install_update_label(Lang::En, "0.2.0"),
            "Install update v0.2.0…"
        );
        assert!(update_notification(Lang::Zh, "0.2.0").0.contains("v0.2.0"));
    }
}
