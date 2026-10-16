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

function cell(text, cls) {
  const td = document.createElement("td");
  td.textContent = text;
  if (cls) td.className = cls;
  return td;
}

// 配置文本来自用户编辑的 JSON，一律走 textContent，不拼 HTML。
function renderControls(map) {
  const tbody = document.querySelector("#controls tbody");
  tbody.replaceChildren();
  for (const c of [...map.controls].sort((a, b) => a.index - b.index)) {
    const tr = document.createElement("tr");
    const press = actionText(c.pressAction, c.press);
    const dbl = c.doublePressAction || c.doublePress ? actionText(c.doublePressAction, c.doublePress) : "—";
    const lng = c.longPressAction || c.longPress ? actionText(c.longPressAction, c.longPress) : "—";
    tr.append(cell(CONTROL_NAMES[c.index] ?? String(c.index)), cell(press, "action"), cell(dbl, "action"), cell(lng, "action"));
    tr.style.cursor = "pointer";
    tr.title = "点击编辑";
    tr.onclick = () => window.openEditor(map, c.index, markDirty);
    tbody.appendChild(tr);
  }
  window.renderHaloEditor(map, document.getElementById("halos"), markDirty);
}

// 表单改动同步进 JSON 文本，再由保存按钮统一校验、应用。
function markDirty() {
  if (!current) return;
  document.getElementById("json").value = JSON.stringify(current, null, 2);
  const tbody = document.querySelector("#controls tbody");
  for (const [i, tr] of [...tbody.children].entries()) {
    const c = [...current.controls].sort((a, b) => a.index - b.index)[i];
    if (!c) continue;
    tr.children[1].textContent = actionText(c.pressAction, c.press);
    tr.children[2].textContent = c.doublePressAction || c.doublePress ? actionText(c.doublePressAction, c.doublePress) : "—";
    tr.children[3].textContent = c.longPressAction || c.longPress ? actionText(c.longPressAction, c.longPress) : "—";
  }
  const msg = document.getElementById("save-msg");
  msg.textContent = "有未保存的改动";
  msg.className = "msg";
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
    document.getElementById("conn").textContent = !s.connected ? "未连接" : s.paused ? "空闲中，已交还控制" : s.online === false ? "接收器在，本体关机" : "已连接";
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


// ---- 设备信息与灯光 ----
const LIGHT_NAMES = ["键 1", "键 2", "键 3", "旋钮"];
let lighting = null;

function renderLighting(l) {
  lighting = l;
  const box = document.getElementById("lighting");
  box.style.display = "flex";
  document.getElementById("light-mode").value = String(l.mode);
  const b = document.getElementById("light-brightness");
  b.value = l.brightness;
  document.getElementById("light-brightness-v").textContent = l.brightness;
  const lights = document.getElementById("lights");
  lights.replaceChildren();
  l.lights.forEach((x, i) => {
    const row = document.createElement("div");
    const name = document.createElement("span");
    name.textContent = `${LIGHT_NAMES[i]}：`;
    const kind = document.createElement("select");
    for (const [v, t] of [[0, "关"], [1, "常亮"], [2, "呼吸"]]) {
      const o = document.createElement("option");
      o.value = String(v); o.textContent = t; kind.appendChild(o);
    }
    kind.value = String(x.kind);
    kind.onchange = () => { lighting.lights[i].kind = Number(kind.value); };
    const slider = document.createElement("input");
    slider.type = "range"; slider.min = 0; slider.max = 20; slider.value = x.alwaysOnBrightness;
    const val = document.createElement("span");
    val.textContent = ` 常亮 ${x.alwaysOnBrightness}`;
    slider.oninput = () => { lighting.lights[i].alwaysOnBrightness = Number(slider.value); val.textContent = ` 常亮 ${slider.value}`; };
    row.append(name, kind, slider, val);
    lights.appendChild(row);
  });
}

document.getElementById("light-mode").onchange = (e) => { if (lighting) lighting.mode = Number(e.target.value); };
document.getElementById("light-brightness").oninput = (e) => {
  if (lighting) lighting.brightness = Number(e.target.value);
  document.getElementById("light-brightness-v").textContent = e.target.value;
};

let loadInfo = async function () {
  const info = document.getElementById("info");
  info.textContent = "读取中…";
  try {
    const i = await invoke("device_info");
    const parts = [];
    if (i.firmware) parts.push(`本体固件 ${i.firmware}`);
    if (i.dongleFirmware) parts.push(`接收器固件 ${i.dongleFirmware}`);
    if (i.noiseReduction) parts.push(`降噪原始值 ${i.noiseReduction.join(" ")}`);
    info.textContent = parts.join(" · ") || "没有读到信息";
    if (i.errors.length) log(`设备信息：${i.errors.join("；")}`, "bad");
    if (i.lighting) renderLighting(i.lighting);
  } catch (e) { info.textContent = String(e); }
};
document.getElementById("light-apply").onclick = async () => {
  const msg = document.getElementById("light-msg");
  if (!lighting) return;
  msg.textContent = "写入中…"; msg.className = "msg";
  try {
    const after = await invoke("set_lighting", { lighting });
    renderLighting(after);
    msg.textContent = "已写入并读回确认"; msg.className = "msg ok";
  } catch (e) { msg.textContent = String(e); msg.className = "msg bad"; }
};


// ---- 写入设备表 & 登录启动 ----
document.getElementById("write-device").onclick = async () => {
  const msg = document.getElementById("controls-msg");
  if (!confirm("把基础层的键盘 / 鼠标条目写进设备自己的表？这会持久保存在设备里，应用不运行时设备按它直出。")) return;
  msg.textContent = "写入中…";
  try {
    const report = await invoke("write_device_keys");
    msg.textContent = report.join("；");
  } catch (e) { msg.textContent = String(e); }
};
const autostart = document.getElementById("autostart");
invoke("get_autostart").then(v => { autostart.checked = v; }).catch(() => {});
autostart.onchange = async () => {
  try { autostart.checked = await invoke("set_autostart", { enabled: autostart.checked }); }
  catch (e) { log(`✗ 登录启动：${e}`, "bad"); autostart.checked = !autostart.checked; }
};


// ---- 电源 / 麦克风 / 空闲 ----
const idleInput = document.getElementById("idle-minutes");
invoke("get_settings").then(v => { idleInput.value = v.idleMinutes; }).catch(() => {});
idleInput.onchange = async () => {
  const msg = document.getElementById("idle-msg");
  try { await invoke("set_idle_minutes", { minutes: Number(idleInput.value) || 0 }); msg.textContent = "已保存"; }
  catch (e) { msg.textContent = String(e); }
};

function showPower(p) {
  const box = document.getElementById("power");
  box.style.display = "flex";
  if (p.sleepDelay != null) document.getElementById("sleep-delay").value = String(p.sleepDelay);
  if (p.standbyDelay != null) document.getElementById("standby-delay").value = p.standbyDelay;
  if (p.micEnabled != null) document.getElementById("mic-enabled").checked = p.micEnabled;
  if (p.noiseReduction != null) document.getElementById("noise-reduction").checked = p.noiseReduction === 1;
  const msg = document.getElementById("power-msg");
  msg.textContent = (p.standby != null ? `设备${p.standby ? "待机中" : "活动中"}` : "") + (p.errors.length ? "；" + p.errors.join("；") : "");
}
const oldLoadInfo = loadInfo;
loadInfo = async function () {
  await oldLoadInfo();
  try { showPower(await invoke("power_info")); } catch (e) { document.getElementById("power-msg").textContent = String(e); }
};
document.getElementById("info-reload").onclick = () => loadInfo();
async function powerWrite(cmd, args, label) {
  const msg = document.getElementById("power-msg");
  msg.textContent = `${label}写入中…`; msg.className = "msg";
  try { await invoke(cmd, args); msg.textContent = `${label}已写入并读回确认`; msg.className = "msg ok"; }
  catch (e) { msg.textContent = `${label}：${e}`; msg.className = "msg bad"; }
}
document.getElementById("sleep-delay").onchange = (e) => powerWrite("set_sleep_delay", { seconds: Number(e.target.value) }, "休眠延时");
document.getElementById("standby-delay").onchange = (e) => powerWrite("set_standby_delay", { seconds: Number(e.target.value) }, "待机延时");
document.getElementById("mic-enabled").onchange = (e) => powerWrite("set_mic_enabled", { enabled: e.target.checked }, "麦克风");
document.getElementById("noise-reduction").onchange = (e) => powerWrite("set_noise_reduction", { level: e.target.checked ? 1 : 0 }, "降噪");
