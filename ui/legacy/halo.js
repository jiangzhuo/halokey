// 悬浮层只渲染，不接收焦点；所有输入来自设备线程转发的事件。
const { listen } = window.__TAURI__.event;
const ring = document.getElementById("ring");
let options = [];
function render(halo, selected) {
  ring.querySelectorAll(".opt").forEach(e => e.remove());
  document.getElementById("title").textContent = halo.name;
  options = halo.options;
  const n = options.length, R = 150, cx = 210, cy = 210;
  options.forEach((o, i) => {
    const a = -Math.PI / 2 + (i * 2 * Math.PI) / n;
    const el = document.createElement("div");
    el.className = "opt" + (i === selected ? " sel" : "");
    el.style.left = `${cx + R * Math.cos(a)}px`;
    el.style.top = `${cy + R * Math.sin(a)}px`;
    el.textContent = o.label;
    el.dataset.i = i;
    ring.appendChild(el);
  });
}
function select(i) {
  ring.querySelectorAll(".opt").forEach(e => e.classList.toggle("sel", Number(e.dataset.i) === i));
}
listen("halokey", ({ payload }) => {
  if (payload.kind === "haloShow") render(payload.halo, payload.selected);
  else if (payload.kind === "haloSelect") select(payload.selected);
});
