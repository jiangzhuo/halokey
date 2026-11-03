use core::fmt;

use halokey_i18n::tr;

/// 配置校验错误，界面可直接显示；文案跟随当前语言（中文与 olanzi 保持一致）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    UnsupportedVersion,
    InvalidControls,
    InvalidLayer,
    InvalidLayerAction,
    InvalidTiming,
    InvalidLongPressTapCount,
    InvalidPressTapCount,
    /// 背光亮度越界。
    InvalidLighting,
    /// 背光记录不是四盏灯。
    InvalidLightingCount,
    InvalidDoublePressBehavior,
    InvalidInheritedGestures,
    UnsupportedGesture,
    InvalidName,
    TooLarge,
    InvalidApplication,
    InvalidStepCount,
    InvalidDelay,
    InvalidLibraryName,
    InvalidLibrarySlot,
    InvalidLibraryAction,
    DuplicateLibraryEntry,
    MissingLibraryAction,
    UnsupportedKeyEntry(u8, u8),
    TooManyKeyEntries,
    InvalidPointerDelta,
    InvalidScrollDelta,
    NotQueueable,
    InvalidHalo,
    MissingHalo,
    Json(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Error::UnsupportedVersion => tr!(
                "配置版本不受支持。",
                "Unsupported config version.",
                "サポートされていない設定バージョンです。"
            ),
            Error::InvalidControls => tr!(
                "配置必须包含且仅包含六个控件，每个控件只能出现一次。",
                "The config must contain exactly the six controls, each only once.",
                "設定には 6 つのコントロールをちょうど 1 回ずつ含める必要があります。"
            ),
            Error::InvalidLayer => tr!(
                "Layer 编号必须为 1–3 且不能重复，每层控件索引必须唯一且在 0–5 之间。",
                "Layer numbers must be 1–3 without duplicates, and each layer's control indexes must be unique and within 0–5.",
                "レイヤー番号は 1–3 で重複不可、各レイヤーのコントロール番号は 0–5 の範囲で重複不可です。"
            ),
            Error::InvalidLayerAction => tr!(
                "切层动作只能放在按键或旋钮按下上，不能放在旋钮转动上；MO 只支持单击和长按，单击 MO 时不能同时设置双击；MO / TG / OSL 的目标层为 1–3，TO 为 0–3。",
                "Layer actions can only go on keys or the knob press, not on knob turns; MO supports only press and long press, and a press MO cannot be combined with a double press; MO / TG / OSL target layers 1–3, TO targets 0–3.",
                "レイヤー切替はキーかノブ押下にのみ設定でき、ノブの回転には設定できません。MO は押す・長押しのみで、押すに MO を設定するとダブルは設定できません。MO / TG / OSL の対象は 1–3、TO は 0–3 です。"
            ),
            Error::InvalidTiming => tr!(
                "双击间隔应为 0.15–0.5 秒，长按阈值应为 0.3–2 秒，且长按阈值必须大于双击间隔。",
                "The double-press window must be 0.15–0.5 s and the long-press threshold 0.3–2 s, and the long-press threshold must exceed the double-press window.",
                "ダブルの間隔は 0.15–0.5 秒、長押しのしきい値は 0.3–2 秒で、しきい値はダブルの間隔より長くする必要があります。"
            ),
            Error::InvalidInheritedGestures => tr!(
                "基础层不能继承其他层的动作。",
                "The base layer cannot inherit actions from other layers.",
                "ベースレイヤーは他のレイヤーの動作を継承できません。"
            ),
            Error::InvalidPressTapCount => tr!(
                "单击和双击的连按次数应为 2–20 次。",
                "Burst count for press and double press must be 2–20.",
                "押す・ダブルの連打回数は 2–20 回にしてください。"
            ),
            Error::InvalidLighting => tr!(
                "背光亮度必须在 0–20 之间。",
                "Backlight brightness must be between 0 and 20.",
                "バックライトの明るさは 0–20 の範囲にしてください。"
            ),
            Error::InvalidLightingCount => tr!(
                "背光记录必须有四盏灯。",
                "A backlight entry must cover four lights.",
                "バックライトの設定には 4 つのライトが必要です。"
            ),
            Error::InvalidDoublePressBehavior => tr!(
                "双击动作不能保持按住，请选择按一次或连按。",
                "A double press cannot hold the key; choose tap once or burst.",
                "ダブルではキーを押し続けられません。1 回押すか連打を選んでください。"
            ),
            Error::InvalidLongPressTapCount => tr!(
                "长按连按次数应为 2–20 次。",
                "Burst count for long press must be 2–20.",
                "長押しの連打回数は 2–20 回にしてください。"
            ),
            Error::UnsupportedGesture => tr!(
                "旋钮转动不支持双击或长按动作。",
                "Knob turns do not support double press or long press.",
                "ノブの回転ではダブルや長押しは使えません。"
            ),
            Error::InvalidName => tr!(
                "配置名称不能为空，且不能超过 80 个字符。",
                "The config name must not be empty or longer than 80 characters.",
                "設定名は空にできず、80 文字以内にしてください。"
            ),
            Error::TooLarge => tr!(
                "配置文件不能超过 32 KiB。",
                "The config file must not exceed 32 KiB.",
                "設定ファイルは 32 KiB 以内にしてください。"
            ),
            Error::InvalidApplication => tr!(
                "请选择有效的应用程序。",
                "Please choose a valid application.",
                "有効なアプリを選んでください。"
            ),
            Error::InvalidStepCount => tr!(
                "宏必须包含 1–32 个步骤。",
                "A macro must have 1–32 steps.",
                "マクロには 1–32 個のステップが必要です。"
            ),
            Error::InvalidDelay => tr!(
                "宏的等待时间必须为 0.05–10 秒。",
                "Macro waits must be 0.05–10 s.",
                "マクロの待機時間は 0.05–10 秒にしてください。"
            ),
            Error::InvalidLibraryName => tr!(
                "功能名称不能为空，且不能超过 80 个字符。",
                "The library item name must not be empty or longer than 80 characters.",
                "ライブラリ項目の名前は空にできず、80 文字以内にしてください。"
            ),
            Error::InvalidLibrarySlot => tr!(
                "宏和 APP 功能各支持 16 个编号（0–15）。",
                "Macros and APP items each have 16 slots (0–15).",
                "マクロとアプリはそれぞれ 16 個の番号（0–15）まで使えます。"
            ),
            Error::InvalidLibraryAction => tr!(
                "功能库只能保存 APP 或宏，不能嵌套引用。",
                "The library can hold only APP items or macros, and they cannot reference each other.",
                "ライブラリに保存できるのはアプリかマクロのみで、入れ子の参照はできません。"
            ),
            Error::DuplicateLibraryEntry => tr!(
                "功能库存在重复的标识或同类编号。",
                "The library has a duplicate ID or a duplicate slot of the same kind.",
                "ライブラリに重複した ID か同種の番号があります。"
            ),
            Error::MissingLibraryAction => tr!(
                "引用的功能不存在，请重新选择。",
                "The referenced library item no longer exists; please choose again.",
                "参照しているライブラリ項目が存在しません。選び直してください。"
            ),
            Error::UnsupportedKeyEntry(kind, code) => tr!(
                "不支持类型 0x{kind:02X} 键码 0x{code:02X} 的键盘条目。",
                "Unsupported key entry: type 0x{kind:02X}, keycode 0x{code:02X}.",
                "種類 0x{kind:02X}・キーコード 0x{code:02X} のキーエントリはサポートされていません。"
            ),
            Error::TooManyKeyEntries => tr!(
                "一个动作最多包含 24 个键盘条目。",
                "An action can have at most 24 key entries.",
                "1 つの動作に含められるキーエントリは 24 個までです。"
            ),
            Error::InvalidPointerDelta => tr!(
                "指针移动的位移不能为零，且每个方向不超过 1000 像素。",
                "Pointer movement must be non-zero and at most 1000 px in each direction.",
                "ポインターの移動量は 0 以外で、各方向 1000 ピクセル以内にしてください。"
            ),
            Error::InvalidScrollDelta => tr!(
                "滚动的格数不能为零，且每个方向不超过 10 格。",
                "Scroll amount must be non-zero and at most 10 notches in each direction.",
                "スクロール量は 0 以外で、各方向 10 ノッチ以内にしてください。"
            ),
            Error::NotQueueable => tr!(
                "该动作由手势路由直接执行，不进入动作队列。",
                "This action is run directly by gesture routing and cannot be queued.",
                "この動作はジェスチャー処理で直接実行され、キューには入りません。"
            ),
            Error::InvalidHalo => tr!(
                "光环菜单需要标识、名称和 1–8 个带名字的选项，选项不能引用功能库或切层。",
                "A halo menu needs an ID, a name and 1–8 named options; options cannot use the library or switch layers.",
                "ハローメニューには ID・名前・名前付きの選択肢 1–8 個が必要で、選択肢にライブラリやレイヤー切替は使えません。"
            ),
            Error::MissingHalo => tr!(
                "引用的光环菜单不存在。",
                "The referenced halo menu does not exist.",
                "参照しているハローメニューが存在しません。"
            ),
            Error::Json(message) => tr!(
                "配置格式无效：{message}",
                "Invalid config format: {message}",
                "設定の形式が不正です：{message}"
            ),
        };
        f.write_str(&text)
    }
}

impl std::error::Error for Error {}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(e.to_string())
    }
}
