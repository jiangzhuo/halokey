// 扇形环几何：N 个等分扇形，第 0 个以 12 点钟为中心，顺时针。SVG 坐标 y 向下。
const polar = (r: number, angle: number, cx: number, cy: number) => ({ x: cx + r * Math.sin(angle), y: cy - r * Math.cos(angle) });
const fmt = (v: number) => (Math.abs(v) < 1e-9 ? 0 : Math.round(v * 1000) / 1000);

/** 第 i 个扇形的中心点（半径 r 处），用于放标签。 */
export function wedgeCenter(n: number, i: number, r: number, cx = 0, cy = 0) {
  const a = (i * 2 * Math.PI) / Math.max(n, 1);
  const p = polar(r, a, cx, cy);
  return { x: fmt(p.x), y: fmt(p.y) };
}

/** 第 i 个扇形的 SVG path（外径 outer、内径 inner）；只有一个选项时画整环。 */
export function wedgePath(n: number, i: number, outer: number, inner: number, cx = 0, cy = 0): string {
  if (n <= 1) {
    // 整环：两段半圆拼起来（外圈顺时针，内圈逆时针），evenodd 填充留出中孔。
    const o1 = polar(outer, 0, cx, cy), o2 = polar(outer, Math.PI, cx, cy);
    const i1 = polar(inner, 0, cx, cy), i2 = polar(inner, Math.PI, cx, cy);
    return `M ${fmt(o1.x)} ${fmt(o1.y)} A ${outer} ${outer} 0 1 1 ${fmt(o2.x)} ${fmt(o2.y)} A ${outer} ${outer} 0 1 1 ${fmt(o1.x)} ${fmt(o1.y)} ` +
      `M ${fmt(i1.x)} ${fmt(i1.y)} A ${inner} ${inner} 0 1 0 ${fmt(i2.x)} ${fmt(i2.y)} A ${inner} ${inner} 0 1 0 ${fmt(i1.x)} ${fmt(i1.y)} Z`;
  }
  const step = (2 * Math.PI) / n;
  const start = i * step - step / 2, end = start + step;
  const large = step > Math.PI ? 1 : 0;
  const o1 = polar(outer, start, cx, cy), o2 = polar(outer, end, cx, cy);
  const i1 = polar(inner, end, cx, cy), i2 = polar(inner, start, cx, cy);
  return `M ${fmt(o1.x)} ${fmt(o1.y)} A ${outer} ${outer} 0 ${large} 1 ${fmt(o2.x)} ${fmt(o2.y)} L ${fmt(i1.x)} ${fmt(i1.y)} A ${inner} ${inner} 0 ${large} 0 ${fmt(i2.x)} ${fmt(i2.y)} Z`;
}
