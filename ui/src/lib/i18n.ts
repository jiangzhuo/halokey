export const EN: Record<string, string> = {
  "键位": "Keymap", "设置": "Settings", "已连接": "Connected", "未连接": "Disconnected", "空闲中，已交还控制": "Idle, control handed back",
  "接收器在，本体关机": "Receiver present, key unit asleep", "电量": "Battery", "固件": "Firmware", "未保存": "Unsaved",
  "顶部按键": "Top key", "中间按键": "Middle key", "底部按键": "Bottom key", "旋钮按下": "Knob press", "右旋": "Turn right", "左旋": "Turn left",
  "单击": "Press", "双击": "Double", "长按": "Long press", "保持按住": "Hold", "短按一次": "Tap once", "连按": "Burst", "次": "×",
  "常用": "Common", "字母": "Letters", "功能键": "F keys", "数字键盘": "Keypad", "鼠标": "Mouse", "组合键": "Combo", "APP": "APP", "宏": "Macro", "光环": "Halo", "指针": "Pointer",
  "撤销更改": "Discard changes", "保存到本机": "Save locally", "已保存": "Saved", "透明继承": "Transparent", "不执行动作": "No action", "关闭手势": "Disable gesture",
  "本层背光": "Layer backlight", "编辑…": "Edit…", "跟随 Layer 0": "Follow Layer 0",
  "Vibe Key": "Vibe Key", "灯光": "Lighting", "麦克风": "Microphone", "电源": "Power", "光环转盘": "Halo wheel", "键位配置": "Keymap file", "通用": "General",
  "状态": "Status", "本体在线": "Key unit online", "本体离线": "Key unit offline", "空闲后交还控制": "Hand back when idle", "永不": "Never", "分钟": "min",
  "登录时启动": "Launch at login", "界面语言": "Language", "刷新状态": "Refresh", "灯光模式": "Light mode", "全灭": "Off", "全亮": "All on", "工作": "Work",
  "全亮亮度": "All-on brightness", "休眠 / 自动关机": "Sleep / auto power-off", "待机延时": "Standby delay", "收音": "Capture", "硬件降噪": "Noise reduction",
  "各灯效果在键位页按 Layer 设置": "Per-light effects are set per layer on the Keymap page", "当前文件": "Current file", "关于": "About",
  "其余写入项在后续计划中补全": "Remaining settings become editable in later phases",
  "出现位置": "Position", "指针周围": "Around the pointer", "屏幕中央": "Screen centre", "自动关闭": "Auto close", "秒": "s", "大小": "Size", "小": "Small", "中": "Medium", "大": "Large",
  "点击上方的手势，再从下面选一个键码": "Pick a gesture above, then a keycode below", "指针步长": "Pointer step", "像素": "px",
  "在本机构建里还没有的功能": "Not available in this build yet", "复杂动作": "Complex action", "无": "None",
  "切换到": "Switch to", "动作库": "Library", "心跳": "heartbeats", "回显": "echoes",
  "滚轮↑": "Wheel ↑", "滚轮↓": "Wheel ↓", "左键": "Left button", "右键": "Right button", "中键": "Middle button", "鼠标键 4": "Mouse 4", "鼠标键 5": "Mouse 5",
  "类型": "type ", "无效码": "Invalid", "左 Control": "Left Control", "左 Shift": "Left Shift", "左 Alt": "Left Alt", "左 Super": "Left Super",
  "右 Control": "Right Control", "右 Shift": "Right Shift", "右 Alt": "Right Alt", "右 Super": "Right Super",
  "拧旋钮选择 · 按下确认 · 按任意键取消": "Turn to choose · press to confirm · any key cancels",
  "未设置（不管灯）": "Not set (lights untouched)", "亮度": "Brightness", "收起": "Collapse", "关": "Off", "常亮": "Solid", "呼吸": "Breathe",
  "不管灯（删除本层背光）": "Leave lights alone (remove this layer's backlight)", "键 1": "Key 1", "键 2": "Key 2", "键 3": "Key 3", "旋钮": "Knob", "设备当前按住的层": "Layer the device is holding", "编辑光环": "Edit halo", "新建光环": "New halo", "向下滚动查看更多": "Scroll down for more",
  "光环名称": "Halo name", "返回": "Back", "名称": "Name", "（空）": "(empty)", "槽位": "Slot", "删除光环": "Delete halo", "取消": "Cancel", "保存光环": "Save halo",
  "被以下位置引用，无法删除": "Cannot delete, referenced by", "光环选项不能引用功能库、切层或自己": "A halo option cannot use the library, a layer switch or this halo itself", "（未命名）": "(unnamed)",
};
let current: "zh" | "en" = "zh";
try { current = (localStorage.getItem("halokey.lang") as "zh" | "en") || (navigator.language.startsWith("zh") ? "zh" : "en"); } catch { /* jsdom 或隐私模式 */ }
export const lang = () => current;
export const t = (zh: string) => (current === "en" ? EN[zh] ?? zh : zh);
export function setLang(l: "zh" | "en") {
  current = l;
  try { localStorage.setItem("halokey.lang", l); } catch { /* 忽略 */ }
  if (typeof window !== "undefined") window.dispatchEvent(new Event("halokey-lang"));
}
