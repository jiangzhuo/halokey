use core::fmt;

/// 配置校验错误，文案与 olanzi 保持一致，界面可直接显示。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    UnsupportedVersion,
    InvalidControls,
    InvalidLayer,
    InvalidLayerAction,
    InvalidTiming,
    InvalidLongPressTapCount,
    InvalidPressTapCount,
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
    NotQueueable,
    InvalidHalo,
    MissingHalo,
    Json(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnsupportedVersion => f.write_str("配置版本不受支持。"),
            Error::InvalidControls => f.write_str("配置必须包含且仅包含六个控件，每个控件只能出现一次。"),
            Error::InvalidLayer => f.write_str("Layer 编号必须为 1–3 且不能重复，每层控件索引必须唯一且在 0–5 之间。"),
            Error::InvalidLayerAction => f.write_str("切层动作仅支持按键或旋钮按下的单击和长按，目标层必须为 1–3；单击切层时不能同时设置双击。"),
            Error::InvalidTiming => f.write_str("双击间隔应为 0.15–0.5 秒，长按阈值应为 0.3–2 秒，且长按阈值必须大于双击间隔。"),
            Error::InvalidInheritedGestures => f.write_str("基础层不能继承其他层的动作。"),
            Error::InvalidPressTapCount => f.write_str("单击和双击的连按次数应为 2–20 次。"),
            Error::InvalidDoublePressBehavior => f.write_str("双击动作不能保持按住，请选择按一次或连按。"),
            Error::InvalidLongPressTapCount => f.write_str("长按连按次数应为 2–20 次。"),
            Error::UnsupportedGesture => f.write_str("旋钮转动不支持双击或长按动作。"),
            Error::InvalidName => f.write_str("配置名称不能为空，且不能超过 80 个字符。"),
            Error::TooLarge => f.write_str("配置文件不能超过 32 KiB。"),
            Error::InvalidApplication => f.write_str("请选择有效的应用程序。"),
            Error::InvalidStepCount => f.write_str("宏必须包含 1–32 个步骤。"),
            Error::InvalidDelay => f.write_str("宏的等待时间必须为 0.05–10 秒。"),
            Error::InvalidLibraryName => f.write_str("功能名称不能为空，且不能超过 80 个字符。"),
            Error::InvalidLibrarySlot => f.write_str("宏和 APP 功能各支持 16 个编号（0–15）。"),
            Error::InvalidLibraryAction => f.write_str("功能库只能保存 APP 或宏，不能嵌套引用。"),
            Error::DuplicateLibraryEntry => f.write_str("功能库存在重复的标识或同类编号。"),
            Error::MissingLibraryAction => f.write_str("引用的功能不存在，请重新选择。"),
            Error::UnsupportedKeyEntry(kind, code) => {
                write!(f, "不支持类型 0x{kind:02X} 键码 0x{code:02X} 的键盘条目。")
            }
            Error::TooManyKeyEntries => f.write_str("一个动作最多包含 24 个键盘条目。"),
            Error::InvalidPointerDelta => f.write_str("指针移动的位移不能为零，且每个方向不超过 1000 像素。"),
            Error::NotQueueable => f.write_str("该动作由手势路由直接执行，不进入动作队列。"),
            Error::InvalidHalo => f.write_str("光环菜单需要标识、名称和 1–8 个带名字的选项，选项不能引用功能库或切层。"),
            Error::MissingHalo => f.write_str("引用的光环菜单不存在。"),
            Error::Json(message) => write!(f, "配置格式无效：{message}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(e.to_string())
    }
}
