// 器械の液晶（3Dの表示パネルに貼る画像）。特定の製品の画面は真似ない、汎用の表示。

import * as THREE from 'three';

const W = 256;
const H = 160;

export interface LcdContent {
  mode: 'level' | 'angle' | 'off';
  /** 液晶◎の点（外側の円の半径を1とした位置。＋X＝右、＋Y＝奥＝画面の上） */
  dotX: number;
  dotY: number;
  innerRatio: number;
  line1: string;
  line2: string;
}

export class LcdScreen {
  readonly texture: THREE.CanvasTexture;
  private readonly ctx: CanvasRenderingContext2D;
  private lastKey = '';

  constructor() {
    const c = document.createElement('canvas');
    c.width = W;
    c.height = H;
    const ctx = c.getContext('2d');
    if (!ctx) throw new Error('2D canvas が使えません');
    this.ctx = ctx;
    this.texture = new THREE.CanvasTexture(c);
    this.texture.colorSpace = THREE.SRGBColorSpace;
  }

  /** 中身が変わったときだけ描き直す */
  draw(c: LcdContent): void {
    const key = `${c.mode}|${c.dotX.toFixed(3)}|${c.dotY.toFixed(3)}|${c.line1}|${c.line2}`;
    if (key === this.lastKey) return;
    this.lastKey = key;
    const g = this.ctx;
    g.fillStyle = c.mode === 'off' ? '#1d2320' : '#b9c7b4';
    g.fillRect(0, 0, W, H);
    if (c.mode === 'off') {
      this.texture.needsUpdate = true;
      return;
    }
    g.strokeStyle = '#1f2a22';
    g.fillStyle = '#1f2a22';
    if (c.mode === 'level') {
      const cx = 70;
      const cy = 80;
      const R = 58;
      g.lineWidth = 2;
      g.beginPath();
      g.arc(cx, cy, R, 0, Math.PI * 2);
      g.stroke();
      g.beginPath();
      g.arc(cx, cy, Math.max(2.5, R * c.innerRatio), 0, Math.PI * 2);
      g.stroke();
      g.lineWidth = 1;
      g.beginPath();
      g.moveTo(cx - R, cy);
      g.lineTo(cx + R, cy);
      g.moveTo(cx, cy - R);
      g.lineTo(cx, cy + R);
      g.stroke();
      if (Number.isFinite(c.dotX) && Number.isFinite(c.dotY)) {
        g.beginPath();
        g.arc(cx + c.dotX * R, cy - c.dotY * R, 4.5, 0, Math.PI * 2);
        g.fill();
      }
      g.font = 'bold 17px "Consolas", "Menlo", monospace';
      g.textBaseline = 'middle';
      g.fillText(c.line1, 136, 58);
      g.fillText(c.line2, 136, 102);
    } else {
      g.font = 'bold 22px "Consolas", "Menlo", monospace';
      g.textBaseline = 'middle';
      g.fillText(c.line1, 14, 54);
      g.fillText(c.line2, 14, 106);
    }
    this.texture.needsUpdate = true;
  }
}
