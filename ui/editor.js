// 表单式动作编辑器：点击控件表格的单元格编辑该手势的动作；光环菜单可增删选项。
// 所有改动先写进 current（app.js 里的配置对象），点保存时整体校验并应用。
(() => {
  const KEYS = [];
  for (let c = 0x04; c <= 0x1D; c++) KEYS.push([c, String.fromCharCode(65 + c - 4)]);
  for (let c = 0x1E; c <= 0x27; c++) KEYS.push([c, String((c - 0x1D) % 10)]);
  KEYS.push([0x28, "Enter"], [0x29, "Esc"], [0x2A, "Backspace"], [0x2B, "Tab"], [0x2C, "Space"], [0x39, "Caps Lock"]);
  for (let c = 0x3A; c <= 0x45; c++) KEYS.push([c, `F${c - 0x39}`]);
  KEYS.push([0x46, "Print Screen"], [0x49, "Insert"], [0x4A, "Home"], [0x4B, "Page Up"], [0x4C, "Delete"], [0x4D, "End"], [0x4E, "Page Down"],
    [0x4F, "→"], [0x50, "←"], [0x51, "↓"], [0x52, "↑"]);
  for (let c = 0x68; c <= 0x73; c++) KEYS.push([c, `F${c - 0x68 + 13}`]);
  KEYS.push([0x01, "Fn"]);
  const MODS = [[0xE0, "Ctrl"], [0xE1, "Shift"], [0xE2, "Alt"], [0xE3, "Super"]];
  const MOUSE = [[0, "左键"], [1, "右键"], [2, "中键"], [3, "鼠标键 4"], [4, "鼠标键 5"], [5, "滚轮↑"], [6, "滚轮↓"]];
  const GESTURES = [["press", "单击", "pressAction", "press"], ["double", "双击", "doublePressAction", "doublePress"], ["long", "长按", "longPressAction", "longPress"]];

  function el(tag, props = {}, children = []) {
    const e = document.createElement(tag);
    for (const [k, v] of Object.entries(props)) {
      if (k === "class") e.className = v;
      else if (k === "text") e.textContent = v;
      else if (k.startsWith("on")) e[k] = v;
      else e.setAttribute(k, v);
    }
    for (const c of children) e.append(c);
    return e;
  }
  function option(value, text, selected) {
    const o = el("option", { value: String(value), text });
    if (selected) o.selected = true;
    return o;
  }

  // 把现有动作拆成编辑器状态。
  function decompose(action, entries) {
    if (!action) {
      const list = entries || [];
      if (list.length === 1 && list[0].type === 7) return { type: "mouse", code: list[0].code };
      if (list.length === 0 || (list.length === 1 && list[0].code === 0)) return { type: "none" };
      return { type: "keyboard", mods: list.filter(e => e.code >= 0xE0 && e.code <= 0xE7).map(e => e.code & 0xFB), key: (list.find(e => e.code < 0xE0) || {}).code ?? 0 };
    }
    const [kind] = Object.keys(action);
    const v = action[kind]._0;
    switch (kind) {
      case "keyboard": return decompose(null, v);
      case "application": return { type: "application", cls: v.bundleIdentifier, path: v.path, name: v.name };
      case "pointerMove": return { type: "pointerMove", dx: v.dx, dy: v.dy };
      case "halo": return { type: "halo", id: v };
      case "momentaryLayer": return { type: "layer", layer: v };
      default: return { type: "raw", raw: action };
    }
  }

  // 把编辑器状态变回动作；返回 { action, entries }。
  function compose(st) {
    switch (st.type) {
      case "none": return { action: null, entries: [{ type: 2, code: 0 }] };
      case "keyboard": {
        const entries = [...st.mods.map(m => ({ type: 2, code: m })), ...(st.key ? [{ type: 2, code: st.key }] : [])];
        return { action: null, entries: entries.length ? entries : [{ type: 2, code: 0 }] };
      }
      case "mouse": return { action: null, entries: [{ type: 7, code: st.code }] };
      case "application": return { action: { application: { _0: { bundleIdentifier: st.cls || "", path: st.path || "", name: st.name || st.cls || "应用" } } } };
      case "pointerMove": return { action: { pointerMove: { _0: { dx: Number(st.dx) || 0, dy: Number(st.dy) || 0 } } } };
      case "halo": return { action: { halo: { _0: st.id } } };
      case "layer": return { action: { momentaryLayer: { _0: Number(st.layer) } } };
      default: return { action: st.raw };
    }
  }

  // 渲染动作选择器；onChange(state) 在任何字段变化时回调。
  function actionPicker(state, halos, onChange, { allowLayer = false } = {}) {
    const box = el("div", { class: "picker" });
    const types = [["none", "无"], ["keyboard", "键盘"], ["mouse", "鼠标"], ["application", "切换应用"], ["pointerMove", "指针移动"], ["halo", "光环菜单"]];
    if (allowLayer) types.push(["layer", "MO 切层"]);
    if (state.type === "raw") types.push(["raw", "（复杂动作，请在 JSON 里改）"]);
    const typeSel = el("select");
    for (const [v, t] of types) typeSel.append(option(v, t, v === state.type));
    const fields = el("div", { class: "fields" });
    const render = () => {
      fields.replaceChildren();
      switch (state.type) {
        case "keyboard": {
          for (const [code, name] of MODS) {
            const cb = el("input", { type: "checkbox" });
            cb.checked = state.mods.includes(code);
            cb.onchange = () => { state.mods = cb.checked ? [...state.mods, code] : state.mods.filter(m => m !== code); onChange(state); };
            fields.append(el("label", {}, [cb, ` ${name} `]));
          }
          const keySel = el("select");
          keySel.append(option(0, "（仅修饰键）", !state.key));
          for (const [code, name] of KEYS) keySel.append(option(code, name, code === state.key));
          keySel.onchange = () => { state.key = Number(keySel.value); onChange(state); };
          fields.append(keySel);
          break;
        }
        case "mouse": {
          const sel = el("select");
          for (const [code, name] of MOUSE) sel.append(option(code, name, code === state.code));
          sel.onchange = () => { state.code = Number(sel.value); onChange(state); };
          fields.append(sel);
          break;
        }
        case "application": {
          const cls = el("input", { placeholder: "WM_CLASS，如 google-chrome", value: state.cls || "" });
          const name = el("input", { placeholder: "显示名", value: state.name || "" });
          const path = el("input", { placeholder: "可执行文件路径（可选）", value: state.path || "" });
          for (const [i, k] of [[cls, "cls"], [name, "name"], [path, "path"]]) i.oninput = () => { state[k] = i.value; onChange(state); };
          fields.append(cls, name, path);
          break;
        }
        case "pointerMove": {
          const dx = el("input", { type: "number", value: state.dx ?? 20, style: "width:70px" });
          const dy = el("input", { type: "number", value: state.dy ?? 0, style: "width:70px" });
          dx.oninput = () => { state.dx = dx.value; onChange(state); };
          dy.oninput = () => { state.dy = dy.value; onChange(state); };
          fields.append("dx ", dx, " dy ", dy, " 像素");
          break;
        }
        case "halo": {
          const sel = el("select");
          if (!halos.length) sel.append(option("", "（先在下方添加光环菜单）", true));
          for (const h of halos) sel.append(option(h.id, `${h.name} (${h.id})`, h.id === state.id));
          if (!state.id && halos.length) state.id = halos[0].id;
          sel.onchange = () => { state.id = sel.value; onChange(state); };
          fields.append(sel);
          break;
        }
        case "layer": {
          const sel = el("select");
          for (const l of [1, 2, 3]) sel.append(option(l, `Layer ${l}`, l === Number(state.layer)));
          if (!state.layer) state.layer = 1;
          sel.onchange = () => { state.layer = Number(sel.value); onChange(state); };
          fields.append(sel);
          break;
        }
        default: break;
      }
    };
    typeSel.onchange = () => {
      const t = typeSel.value;
      const fresh = { none: { type: "none" }, keyboard: { type: "keyboard", mods: [], key: 0x28 }, mouse: { type: "mouse", code: 5 },
        application: { type: "application", cls: "", name: "", path: "" }, pointerMove: { type: "pointerMove", dx: 20, dy: 0 },
        halo: { type: "halo", id: halos[0]?.id || "" }, layer: { type: "layer", layer: 1 } }[t] || { type: t };
      Object.keys(state).forEach(k => delete state[k]);
      Object.assign(state, fresh);
      render();
      onChange(state);
    };
    render();
    box.append(typeSel, fields);
    return box;
  }

  // ---- 控件编辑面板 ----
  const panel = el("section", { class: "wide", id: "editor" });
  const title = el("h2", { text: "编辑动作" });
  const body = el("div");
  panel.append(title, body);
  document.querySelector("main").insertBefore(panel, document.getElementById("json").closest("section"));
  panel.style.display = "none";

  window.openEditor = (map, index, onDirty) => {
    const control = map.controls.find(c => c.index === index);
    if (!control) return;
    panel.style.display = "";
    title.textContent = `编辑：${["键 1", "键 2", "键 3", "旋钮按下", "旋钮 → 右拧", "旋钮 ← 左拧"][index]}`;
    body.replaceChildren();
    const gestures = index < 4 ? GESTURES : GESTURES.slice(0, 1);
    for (const [id, label, actionKey, entriesKey] of gestures) {
      const row = el("div", { class: "row" });
      const enabled = id === "press" || control[actionKey] != null || control[entriesKey] != null;
      const state = enabled ? decompose(control[actionKey], control[entriesKey]) : { type: "none" };
      const apply = (st) => {
        if (id !== "press" && st.type === "none") { delete control[actionKey]; delete control[entriesKey]; onDirty(); return; }
        const { action, entries } = compose(st);
        if (action) { control[actionKey] = action; if (id === "press") control.press = control.press || []; else delete control[entriesKey]; }
        else { delete control[actionKey]; control[entriesKey] = entries; }
        onDirty();
      };
      row.append(el("strong", { text: label, style: "width:48px" }), actionPicker(state, map.halos || [], apply, { allowLayer: index < 4 && id !== "double" }));
      if (id === "press") {
        const beh = el("select");
        for (const [v, t] of [["hold", "跟随按住"], ["tap", "短按一次"], ["burst", "连按"]]) beh.append(option(v, t, (control.pressBehavior || "hold") === v));
        beh.onchange = () => { control.pressBehavior = beh.value; onDirty(); };
        row.append(" 输出 ", beh);
      }
      body.append(row);
    }
    const close = el("button", { text: "收起", onclick: () => { panel.style.display = "none"; } });
    body.append(el("div", { class: "row" }, [close, el("span", { class: "msg", text: "改动会进入下方 JSON，点「保存并应用」生效。" })]));
  };

  // ---- 光环菜单编辑 ----
  window.renderHaloEditor = (map, container, onDirty) => {
    container.replaceChildren();
    map.halos = map.halos || [];
    for (const h of map.halos) {
      const card = el("div", { class: "halo-card" });
      const id = el("input", { value: h.id, placeholder: "id", style: "width:90px" });
      const name = el("input", { value: h.name, placeholder: "名称", style: "width:120px" });
      id.oninput = () => { h.id = id.value; onDirty(); };
      name.oninput = () => { h.name = name.value; onDirty(); };
      const del = el("button", { text: "删除菜单", onclick: () => { map.halos = map.halos.filter(x => x !== h); onDirty(); window.renderHaloEditor(map, container, onDirty); } });
      card.append(el("div", { class: "row" }, [el("span", { class: "pill", text: "光环" }), id, name, del]));
      h.options.forEach((o, i) => {
        const row = el("div", { class: "row" });
        const label = el("input", { value: o.label, placeholder: "选项文字", style: "width:110px" });
        label.oninput = () => { o.label = label.value; onDirty(); };
        const state = decompose(o.action, null);
        const picker = actionPicker(state, map.halos, (st) => { const { action, entries } = compose(st); o.action = action || { keyboard: { _0: entries } }; onDirty(); });
        const rm = el("button", { text: "×", onclick: () => { h.options.splice(i, 1); onDirty(); window.renderHaloEditor(map, container, onDirty); } });
        row.append(el("span", { text: `${i + 1}.`, style: "width:22px" }), label, picker, rm);
        card.append(row);
      });
      if (h.options.length < 8) {
        card.append(el("button", { text: "+ 选项", onclick: () => { h.options.push({ label: "新选项", action: { keyboard: { _0: [{ type: 2, code: 0x28 }] } } }); onDirty(); window.renderHaloEditor(map, container, onDirty); } }));
      }
      container.append(card);
    }
    container.append(el("button", { text: "+ 新建光环菜单", onclick: () => { map.halos.push({ id: `halo${map.halos.length + 1}`, name: "新菜单", options: [{ label: "回车", action: { keyboard: { _0: [{ type: 2, code: 0x28 }] } } }] }); onDirty(); window.renderHaloEditor(map, container, onDirty); } }));
  };
})();
