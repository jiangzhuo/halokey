// 界面语言：中文是源文本，英文表按源文本查找；没有翻译的字符串原样显示。
// 语言选择只是本机浏览器偏好，存在 localStorage，不进配置文件。
(() => {
  const EN = {
    "未连接": "Disconnected", "已连接": "Connected", "空闲中，已交还控制": "Idle, control handed back",
    "接收器在，本体关机": "Receiver present, key unit asleep", "电量 —": "Battery —",
    "控件": "Controls", "单击": "Press", "双击": "Double press", "长按": "Long press",
    "从设备重读": "Read device table", "写入设备（离线备用）": "Write to device (offline backup)",
    "光环菜单": "Halo menus", "最近事件": "Recent events", "设备": "Device", "读取设备信息": "Read device info",
    "登录时启动": "Launch at login", "写入灯光": "Apply lighting", "灯光模式": "Light mode",
    "全亮亮度": "All-on brightness", "关": "Off", "常亮": "Solid", "呼吸": "Breathing", "模式 2": "Mode 2",
    "配置 JSON（与 olanzi 兼容，光环与指针移动为 halokey 扩展）": "Configuration JSON (olanzi-compatible; halos and pointer moves are halokey extensions)",
    "保存并应用": "Save and apply", "放弃修改": "Discard changes", "已保存并应用": "Saved and applied",
    "有未保存的改动": "Unsaved changes", "编辑动作": "Edit actions", "收起": "Close",
    "改动会进入下方 JSON，点「保存并应用」生效。": "Changes go into the JSON below; click Save and apply.",
    "键 1": "Key 1", "键 2": "Key 2", "键 3": "Key 3", "旋钮按下": "Knob press", "旋钮 → 右拧": "Knob → right", "旋钮 ← 左拧": "Knob ← left", "旋钮": "Knob",
    "无": "None", "键盘": "Keyboard", "鼠标": "Mouse", "切换应用": "Switch app", "指针移动": "Pointer move", "MO 切层": "MO layer",
    "（复杂动作，请在 JSON 里改）": "(complex action, edit in JSON)", "（仅修饰键）": "(modifiers only)",
    "左键": "Left button", "右键": "Right button", "中键": "Middle button", "鼠标键 4": "Mouse button 4", "鼠标键 5": "Mouse button 5", "滚轮↑": "Wheel up", "滚轮↓": "Wheel down",
    "WM_CLASS，如 google-chrome": "WM_CLASS, e.g. google-chrome", "显示名": "Display name", "可执行文件路径（可选）": "Executable path (optional)",
    "像素": "px", "（先在下方添加光环菜单）": "(add a halo menu below first)", "输出": "Output", "跟随按住": "Hold", "短按一次": "Tap once", "连按": "Burst",
    "删除菜单": "Delete menu", "+ 选项": "+ Option", "+ 新建光环菜单": "+ New halo menu", "新菜单": "New menu", "新选项": "New option", "选项文字": "Option label", "名称": "Name",
    "分钟后交还控制让设备待机（0 = 从不）": "minutes idle → hand control back so the unit can sleep (0 = never)", "空闲": "Idle",
    "休眠 / 自动关机": "Sleep / auto power-off", "从不": "Never", "1 小时": "1 hour", "2 小时": "2 hours", "3 小时": "3 hours", "4 小时": "4 hours",
    "待机延时": "Standby delay", "秒": "s", "麦克风": "Microphone", "降噪": "Noise reduction", "已保存": "Saved",
    "读取中…": "Reading…", "没有读到信息": "Nothing read", "写入中…": "Writing…", "已写入并读回确认": "Written and verified by read-back",
    "接收器已连接": "Receiver connected", "接收器断开": "Receiver disconnected", "光环关闭": "Halo closed",
    "按下": "down", "松开": "up", "设备待机中": "Unit in standby", "设备活动中": "Unit active",
    "把基础层的键盘 / 鼠标条目写进设备自己的表？这会持久保存在设备里，应用不运行时设备按它直出。": "Write the base layer's keyboard / mouse entries into the device's own table? This persists on the device and is what it types when halokey is not running.",
    "拧旋钮选择 · 按下确认 · 按任意键取消": "Turn to select · press to confirm · any key cancels",
  };
  let lang = "zh";
  try { lang = localStorage.getItem("halokey.lang") || (navigator.language.startsWith("zh") ? "zh" : "en"); } catch (_) {}
  window.t = (text) => (lang === "en" && EN[text] != null ? EN[text] : text);
  window.currentLang = () => lang;
  window.setLang = (next) => {
    lang = next;
    try { localStorage.setItem("halokey.lang", next); } catch (_) {}
    applyStatic();
    document.dispatchEvent(new Event("halokey-lang"));
  };
  // 静态文本：元素带 data-i18n（值为中文源文本）。
  function applyStatic() {
    document.querySelectorAll("[data-i18n]").forEach(el => { el.textContent = window.t(el.dataset.i18n); });
    document.querySelectorAll("[data-i18n-ph]").forEach(el => { el.placeholder = window.t(el.dataset.i18nPh); });
    document.documentElement.lang = lang;
  }
  document.addEventListener("DOMContentLoaded", applyStatic);
})();
