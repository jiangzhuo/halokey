// 设置窗口：读快照、展示键位、编辑 JSON、保存并应用。只通过 Tauri 命令与设备线程交互。
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const CONTROL_NAMES = ["键 1", "键 2", "键 3", "旋钮按下", "旋钮 → 右拧", "旋钮 ← 左拧"];
const KEY_NAMES = { 0x28: "Enter", 0x29: "Esc", 0x2A: "Backspace", 0x2B: "Tab", 0x2C: "Space", 0x39: "Caps Lock", 0x46: "Print Screen",
  0x4F: "→", 0x50: "←", 0x51: "↓", 0x52: "↑", 0x4A: "Home", 0x4D: "End", 0x4B: "Page Up", 0x4E: "Page Down", 0x4C: "Delete",
  0xE0: "Ctrl", 0xE1: "Shift", 0xE2: "Alt", 0xE3: "Super", 0xE4: "RCtrl", 0xE5: "RShift", 0xE6: "RAlt", 0xE7: "RSuper", 0x01: "Fn", 0x00: "无" };
const MOUSE_NAMES = ["左键", "右键", "中键", "鼠标键 4", "鼠标键 5", "滚轮↑", "滚轮↓"];

function keyName(entry) {
  if (entry.type === 7) return MOUSE_NAMES[entry.code] ?? `鼠标 0x${entry.code.toString(16)}`;
  if (entry.type && entry.type !== 2) return `类型${entry.type} 0x${entry.code.toString(16)}`;
  const c = entry.code;
  if (KEY_NAMES[c]) return KEY_NAMES[c];
  if (c >= 0x04 && c <= 0x1D) return String.fromCharCode(65 + c - 4);
  if (c >= 0x1E && c <= 0x27) return String((c - 0x1D) % 10);
  if (c >= 0x3A && c <= 0x45) return `F${c - 0x39}`;
  if (c >= 0x68 && c <= 0x73) return `F${c - 0x68 + 13}`;
  return `0x${c.toString(16).padStart(2, "0")}`;
}

function actionText(action, entries) {
  if (!action) return entries && entries.length ? entries.map(keyName).join(" + ") : "无";
  const [kind] = Object.keys(action);
  const v = action[kind]._0;
  switch (kind) {
    case "keyboard": return v.length ? v.map(keyName).join(" + ") : "无";
    case "application": return `切换到 ${v.name}`;
    case "macro": return `宏（${v.length} 步）`;
    case "library": return `动作库 ${v.slice(0, 8)}`;
    case "momentaryLayer": return `MO(${v})`;
    case "pointerMove": return `指针移动 (${v.dx}, ${v.dy})`;
    case "halo": return `光环「${v}」`;
    default: return kind;
  }
}

let current = null;

function renderControls(map) {
  const tbody = document.querySelector("#controls tbody");
  tbody.innerHTML = "";
  for (const c of [...map.controls].sort((a, b) => a.index - b.index)) {
    const tr = document.createElement("tr");
    const press = actionText(c.pressAction, c.press);
    const dbl = c.doublePressAction || c.doublePress ? actionText(c.doublePressAction, c.doublePress) : "—";
    const lng = c.longPressAction || c.longPress ? actionText(c.longPressAction, c.longPress) : "—";
    tr.innerHTML = `<td>${CONTROL_NAMES[c.index] ?? c.index}</td><td class="action">${press}</td><td class="action">${dbl}</td><td class="action">${lng}</td>`;
    tbody.appendChild(tr);
  }
  const halos = document.getElementById("halos");
  if (map.halos && map.halos.length) {
    halos.innerHTML = map.halos.map(h => `<div><span class="pill">${h.id}</span>${h.name}：${h.options.map(o => o.label).join(" · ")}</div>`).join("");
  }
}

async function loadKeymap() {
  try {
    current = await invoke("get_keymap");
    renderControls(current);
    document.getElementById("json").value = JSON.stringify(current, null, 2);
    document.getElementById("controls-msg").textContent = "";
  } catch (e) {
    document.getElementById("controls-msg").textContent = String(e);
  }
}

async function refreshSnapshot() {
  try {
    const s = await invoke("snapshot");
    document.getElementById("dot").className = "dot" + (s.connected ? " ok" : "");
    document.getElementById("conn").textContent = s.connected ? (s.online === false ? "接收器在，本体关机" : "已连接") : "未连接";
    if (s.battery) {
      const [pct, mv, charging] = s.battery;
      document.getElementById("battery").textContent = `电量 ${pct == null ? "?" : pct + "%"} · ${mv} mV${charging ? " · 充电中" : ""}`;
    }
    document.getElementById("stats").textContent = `心跳 ${s.heartbeats} / 回显 ${s.echoes} / 注入 ${s.injected}`;
    document.getElementById("path").textContent = s.configPath;
    if (s.lastError) log(`✗ ${s.lastError}`, "bad");
    if (!current && s.connected) loadKeymap();
  } catch (e) {
    document.getElementById("conn").textContent = String(e);
  }
}

const logEl = document.getElementById("log");
function log(text, cls) {
  const line = document.createElement("div");
  line.textContent = text;
  if (cls) line.className = cls;
  logEl.prepend(line);
  while (logEl.children.length > 60) logEl.lastChild.remove();
}

document.getElementById("save").onclick = async () => {
  const msg = document.getElementById("save-msg");
  try {
    const value = JSON.parse(document.getElementById("json").value);
    await invoke("save_keymap", { keymap: value });
    current = value;
    renderControls(current);
    msg.textContent = "已保存并应用";
    msg.className = "msg ok";
  } catch (e) {
    msg.textContent = String(e);
    msg.className = "msg bad";
  }
};
document.getElementById("reset").onclick = () => { if (current) document.getElementById("json").value = JSON.stringify(current, null, 2); };
document.getElementById("reload").onclick = async () => {
  const msg = document.getElementById("controls-msg");
  try {
    const keys = await invoke("read_device_keys");
    msg.textContent = "设备表：" + keys.map(([i, e]) => `${CONTROL_NAMES[i]}=${e.map(([t, c]) => keyName({ type: t, code: c })).join("+")}`).join("，");
  } catch (e) { msg.textContent = String(e); }
};

listen("halokey", ({ payload }) => {
  switch (payload.kind) {
    case "connected": log("接收器已连接"); refreshSnapshot(); loadKeymap(); break;
    case "disconnected": log("接收器断开"); refreshSnapshot(); break;
    case "key": log(`${payload.at.toFixed(3)}s ${payload.name} ${payload.pressed ? "按下" : "松开"}`, "k"); break;
    case "battery": refreshSnapshot(); break;
    case "error": log(`✗ ${payload.text}`, "bad"); break;
    case "haloShow": log(`光环「${payload.halo.name}」打开`); break;
    case "haloHide": log("光环关闭"); break;
    default: break;
  }
});

refreshSnapshot();
setInterval(refreshSnapshot, 3000);
