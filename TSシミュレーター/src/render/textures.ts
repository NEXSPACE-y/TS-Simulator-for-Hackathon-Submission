// 質感はキャンバスで描いた画像（テクスチャ）で表す（SPEC 9.2）。外部の画像素材は使わない。

import * as THREE from 'three';

function canvas(w: number, h: number): [HTMLCanvasElement, CanvasRenderingContext2D] {
  const c = document.createElement('canvas');
  c.width = w;
  c.height = h;
  const ctx = c.getContext('2d');
  if (!ctx) throw new Error('2D canvas が使えません');
  return [c, ctx];
}

/** 再現できる乱数（見た目が毎回同じになるように） */
function rand(seed: number): () => number {
  let s = seed >>> 0 || 1;
  return () => {
    s ^= s << 13;
    s ^= s >>> 17;
    s ^= s << 5;
    return ((s >>> 0) % 100000) / 100000;
  };
}

function finish(c: HTMLCanvasElement, repeat = 1): THREE.CanvasTexture {
  const t = new THREE.CanvasTexture(c);
  t.colorSpace = THREE.SRGBColorSpace;
  if (repeat !== 1) {
    t.wrapS = THREE.RepeatWrapping;
    t.wrapT = THREE.RepeatWrapping;
    t.repeat.set(repeat, repeat);
  }
  t.anisotropy = 4;
  return t;
}

/** 地面（土と砂利）。中央付近の陰影は焼き込みで表す */
export function groundTexture(): THREE.CanvasTexture {
  const [c, g] = canvas(512, 512);
  const r = rand(7);
  g.fillStyle = '#7a6a55';
  g.fillRect(0, 0, 512, 512);
  for (let i = 0; i < 260; i++) {
    const x = r() * 512;
    const y = r() * 512;
    const rad = 10 + r() * 40;
    const shade = r() < 0.5 ? 'rgba(60,48,36,0.10)' : 'rgba(150,135,110,0.10)';
    g.fillStyle = shade;
    g.beginPath();
    g.arc(x, y, rad, 0, Math.PI * 2);
    g.fill();
  }
  for (let i = 0; i < 5000; i++) {
    const x = r() * 512;
    const y = r() * 512;
    const v = 90 + Math.floor(r() * 110);
    g.fillStyle = `rgba(${v},${v - 6},${v - 14},${0.35 + r() * 0.5})`;
    const s = 1 + r() * 2.6;
    g.fillRect(x, y, s, s);
  }
  return finish(c, 80);
}

/** 測点の金属標（十字と中心の点） */
export function markTexture(): THREE.CanvasTexture {
  const [c, g] = canvas(256, 256);
  const grd = g.createRadialGradient(110, 100, 10, 128, 128, 128);
  grd.addColorStop(0, '#d8c07e');
  grd.addColorStop(1, '#8f7434');
  g.fillStyle = grd;
  g.beginPath();
  g.arc(128, 128, 126, 0, Math.PI * 2);
  g.fill();
  g.strokeStyle = 'rgba(60,45,15,0.8)';
  g.lineWidth = 4;
  g.beginPath();
  g.arc(128, 128, 110, 0, Math.PI * 2);
  g.stroke();
  g.lineWidth = 3;
  g.beginPath();
  g.moveTo(128, 30);
  g.lineTo(128, 226);
  g.moveTo(30, 128);
  g.lineTo(226, 128);
  g.stroke();
  g.fillStyle = '#2a1f08';
  g.beginPath();
  g.arc(128, 128, 6, 0, Math.PI * 2);
  g.fill();
  return finish(c);
}

/** つまみのギザギザ */
export function knurlTexture(): THREE.CanvasTexture {
  const [c, g] = canvas(128, 16);
  g.fillStyle = '#30343a';
  g.fillRect(0, 0, 128, 16);
  g.fillStyle = '#5a6068';
  for (let x = 0; x < 128; x += 8) g.fillRect(x, 0, 3, 16);
  return finish(c);
}

/** ポールの紅白 */
export function poleTexture(): THREE.CanvasTexture {
  const [c, g] = canvas(16, 256);
  for (let i = 0; i < 8; i++) {
    g.fillStyle = i % 2 === 0 ? '#c8352e' : '#f2f2ee';
    g.fillRect(0, i * 32, 16, 32);
  }
  return finish(c);
}

/** BSのターゲット板（白黒の四分割と円） */
export function targetTexture(): THREE.CanvasTexture {
  const [c, g] = canvas(256, 256);
  g.fillStyle = '#f4f4f0';
  g.fillRect(0, 0, 256, 256);
  g.fillStyle = '#111';
  g.fillRect(0, 0, 128, 128);
  g.fillRect(128, 128, 128, 128);
  g.strokeStyle = '#d13b2c';
  g.lineWidth = 6;
  g.beginPath();
  g.arc(128, 128, 70, 0, Math.PI * 2);
  g.stroke();
  g.strokeStyle = '#d13b2c';
  g.lineWidth = 2;
  g.beginPath();
  g.moveTo(128, 0);
  g.lineTo(128, 256);
  g.moveTo(0, 128);
  g.lineTo(256, 128);
  g.stroke();
  return finish(c);
}

/** FSのプリズムの枠（黄と黒） */
export function prismFrameTexture(): THREE.CanvasTexture {
  const [c, g] = canvas(256, 256);
  g.fillStyle = '#e9b81c';
  g.fillRect(0, 0, 256, 256);
  g.fillStyle = '#151515';
  g.beginPath();
  g.moveTo(128, 6);
  g.lineTo(250, 128);
  g.lineTo(128, 250);
  g.lineTo(6, 128);
  g.closePath();
  g.fill();
  g.fillStyle = '#e9b81c';
  g.beginPath();
  g.arc(128, 128, 72, 0, Math.PI * 2);
  g.fill();
  return finish(c);
}
