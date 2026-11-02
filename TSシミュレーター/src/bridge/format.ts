// 毎フレームの数値を、画面に出す文字列に整える（並べ替え・桁そろえだけ。計算はしない）。
// 度分秒への変換（丸め・繰り上がり）はコアで済んでいて、ここでは部品を並べるだけ。

import type { Frame } from './index';

const pad2 = (n: number): string => (n < 10 ? `0${n}` : `${n}`);

/** 符号つきの度分秒。例：+0°00′12″ / −0°00′08″ */
export function dmsSignedAt(f: Frame, offset: number): string {
  const d = f.data;
  const sign = (d[offset] ?? 1) < 0 ? '−' : '+';
  return `${sign}${d[offset + 1]}°${pad2(d[offset + 2] ?? 0)}′${pad2(d[offset + 3] ?? 0)}″`;
}

/** 0〜360°の度分秒。例：123°45′06″ */
export function dmsCircleAt(f: Frame, offset: number): string {
  const d = f.data;
  return `${d[offset + 1]}°${pad2(d[offset + 2] ?? 0)}′${pad2(d[offset + 3] ?? 0)}″`;
}

/** タイム。例：2:34.5 */
export function timeAt(f: Frame): string {
  const i = f.o.elapsedParts;
  const d = f.data;
  return `${d[i]}:${pad2(d[i + 1] ?? 0)}.${d[i + 2]}`;
}

/** 数値を小数 digits 桁で。値がないときは --- */
export function fixed(v: number | undefined, digits: number): string {
  return v === undefined || !Number.isFinite(v) ? '---' : v.toFixed(digits);
}

export function flag(f: Frame, offset: number): boolean {
  return (f.data[offset] ?? 0) > 0.5;
}
