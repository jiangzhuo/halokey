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

/// 安装更新时用户取消了系统授权：设置页错误行显示这一句，而不是插件的英文错误。
pub fn install_cancelled(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "Authorization cancelled; you can try again later",
        Lang::Zh => "已取消授权，可以稍后再试",
        Lang::Ja => "認証がキャンセルされました。あとでもう一度お試しください",
    }
}

/// 点了安装但已经没有可安装的更新（例如状态刚被别的操作改掉）。
pub fn nothing_to_install(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "No update to install",
        Lang::Zh => "没有可安装的更新",
        Lang::Ja => "インストールできるアップデートはありません",
    }
}

/// 打开发布说明时版本号不合法（只允许数字、字母、`.`、`-`、`+`）。
pub fn invalid_version(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "Invalid version number",
        Lang::Zh => "版本号无效",
        Lang::Ja => "バージョン番号が正しくありません",
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

    #[test]
    fn install_errors_are_localized() {
        assert_eq!(nothing_to_install(Lang::En), "No update to install");
        assert_eq!(nothing_to_install(Lang::Zh), "没有可安装的更新");
        assert_eq!(invalid_version(Lang::En), "Invalid version number");
        assert_eq!(invalid_version(Lang::Zh), "版本号无效");
        assert!(!nothing_to_install(Lang::Ja).is_empty() && !invalid_version(Lang::Ja).is_empty());
    }
}
