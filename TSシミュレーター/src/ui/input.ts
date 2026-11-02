// 押している間の操作（ボタン・キーボード）をまとめて、毎フレーム bridge に渡す。
// どれだけ動くか（速さ×時間）の計算はコアが行う。ここは「何を押しているか」を伝えるだけ。

import type { Bridge, CommandName } from '../bridge';

export type SpeedLevel = 0 | 1 | 2;

export type HoldIntent =
  | { kind: 'moveLegs'; dx: number; dy: number }
  | { kind: 'legLength'; leg: number; extend: boolean }
  | { kind: 'screw'; screw: number; thumb: 1 | -1 }
  | { kind: 'slide'; dx: number; dy: number }
  | { kind: 'telescope'; dh: number; dv: number }
  | { kind: 'focus'; dir: 1 | -1 }
  | { kind: 'eyepiece'; dir: 1 | -1 };

export interface InputContext {
  phase: 'setup' | 'setupResult' | 'observation';
  mode: 'textbook' | 'field';
  step: number;
  clampTight: boolean;
  peepSight: boolean;
}

type Cmd = readonly [CommandName, number, number, number];

/** キー → 押している間の操作 */
const KEY_HOLDS: Record<string, HoldIntent> = {
  KeyA: { kind: 'screw', screw: 0, thumb: -1 },
  KeyS: { kind: 'screw', screw: 0, thumb: 1 },
  KeyK: { kind: 'screw', screw: 1, thumb: -1 },
  KeyL: { kind: 'screw', screw: 1, thumb: 1 },
  KeyZ: { kind: 'screw', screw: 2, thumb: -1 },
  KeyX: { kind: 'screw', screw: 2, thumb: 1 },
  Digit1: { kind: 'legLength', leg: 0, extend: true },
  Digit2: { kind: 'legLength', leg: 1, extend: true },
  Digit3: { kind: 'legLength', leg: 2, extend: true },
  KeyQ: { kind: 'legLength', leg: 0, extend: false },
  KeyW: { kind: 'legLength', leg: 1, extend: false },
  KeyE: { kind: 'legLength', leg: 2, extend: false },
};

/** 観測のときだけ使うキー → 押している間の操作（合焦つまみ・接眼レンズ） */
const OBSERVATION_KEY_HOLDS: Record<string, HoldIntent> = {
  KeyF: { kind: 'focus', dir: 1 },
  KeyG: { kind: 'focus', dir: -1 },
  KeyV: { kind: 'eyepiece', dir: -1 },
  KeyB: { kind: 'eyepiece', dir: 1 },
};

const ARROWS: Record<string, [number, number]> = {
  ArrowUp: [0, 1],
  ArrowDown: [0, -1],
  ArrowLeft: [-1, 0],
  ArrowRight: [1, 0],
};

export class InputManager {
  private holds = new Map<string, HoldIntent>();
  private speedLevel: SpeedLevel = 1;
  private fine = false;
  private detach: (() => void) | null = null;

  constructor(
    private readonly bridge: Bridge,
    private readonly context: () => InputContext,
  ) {}

  get speed(): SpeedLevel {
    return this.speedLevel;
  }

  setSpeed(s: SpeedLevel): void {
    this.speedLevel = s;
  }

  press(id: string, intent: HoldIntent): void {
    this.holds.set(id, intent);
  }

  release(id: string): void {
    this.holds.delete(id);
  }

  releaseAll(): void {
    this.holds.clear();
    this.fine = false;
  }

  /** bridge に毎フレームの操作を渡し始め、キーボードの操作も受け付ける */
  attach(target: Window): void {
    this.detach?.();
    this.bridge.setHoldProvider(this.provide);
    const isTyping = (e: KeyboardEvent) => {
      const el = e.target as HTMLElement | null;
      const tag = el?.tagName;
      return tag === 'INPUT' || tag === 'SELECT' || tag === 'TEXTAREA';
    };
    const down = (e: KeyboardEvent) => {
      if (isTyping(e) || e.ctrlKey || e.metaKey || e.altKey) return;
      if (e.key === 'Shift') this.fine = true;
      const ctx = this.context();
      const arrow = ARROWS[e.code];
      if (arrow) {
        e.preventDefault();
        this.holds.set(`key:${e.code}`, this.arrowIntent(ctx, arrow));
        return;
      }
      const hold = KEY_HOLDS[e.code] ?? (ctx.phase === 'observation' ? OBSERVATION_KEY_HOLDS[e.code] : undefined);
      if (hold) {
        e.preventDefault();
        this.holds.set(`key:${e.code}`, hold);
        return;
      }
      if (e.code === 'Enter' && ctx.phase === 'observation' && ctx.step >= 9) {
        // 選ばれているボタン（「測量」など）の Enter は打ち消して、測量を1回だけ送る
        e.preventDefault();
        if (!e.repeat) this.bridge.send('observe');
        return;
      }
      if (e.repeat) return;
      if (e.code === 'KeyT' && ctx.phase === 'setup') {
        this.bridge.send('setClamp', ctx.clampTight ? 0 : 1);
      } else if (e.code === 'KeyP' && ctx.phase === 'observation' && ctx.step >= 9) {
        this.bridge.send('setPeepSight', ctx.peepSight ? 0 : 1);
      }
    };
    const up = (e: KeyboardEvent) => {
      if (e.key === 'Shift') this.fine = false;
      this.holds.delete(`key:${e.code}`);
    };
    const blur = () => this.releaseAll();
    target.addEventListener('keydown', down);
    target.addEventListener('keyup', up);
    target.addEventListener('blur', blur);
    this.detach = () => {
      target.removeEventListener('keydown', down);
      target.removeEventListener('keyup', up);
      target.removeEventListener('blur', blur);
    };
  }

  dispose(): void {
    this.detach?.();
    this.detach = null;
    this.bridge.setHoldProvider(null);
    this.holds.clear();
  }

  /** 矢印キーの意味は、そのときの手順で変わる */
  private arrowIntent(ctx: InputContext, [x, y]: [number, number]): HoldIntent {
    if (ctx.phase === 'observation') return { kind: 'telescope', dh: x, dv: y };
    const slideStep = ctx.mode === 'field' ? ctx.step === 6 : ctx.step === 6 || ctx.step === 7;
    if (slideStep) return { kind: 'slide', dx: x, dy: y };
    return { kind: 'moveLegs', dx: x, dy: y };
  }

  private readonly provide = (): Cmd[] => {
    if (this.holds.size === 0) return [];
    const speed = this.fine ? 2 : this.speedLevel;
    const out: Cmd[] = [];
    let mx = 0, my = 0, sx = 0, sy = 0, th = 0, tv = 0;
    for (const h of this.holds.values()) {
      switch (h.kind) {
        case 'moveLegs':
          mx += h.dx;
          my += h.dy;
          break;
        case 'slide':
          sx += h.dx;
          sy += h.dy;
          break;
        case 'telescope':
          th += h.dh;
          tv += h.dv;
          break;
        case 'legLength':
          out.push(['changeLegLength', h.leg, h.extend ? 1 : 0, speed]);
          break;
        case 'screw':
          out.push(['turnScrew', h.screw, h.thumb, speed]);
          break;
        case 'focus':
          out.push(['turnFocus', h.dir, speed, 0]);
          break;
        case 'eyepiece':
          out.push(['turnEyepiece', h.dir, speed, 0]);
          break;
      }
    }
    if (mx !== 0 || my !== 0) out.push(['moveFreeLegs', mx, my, speed]);
    if (sx !== 0 || sy !== 0) out.push(['slideInstrument', sx, sy, speed]);
    if (th !== 0 || tv !== 0) out.push(['rotateTelescope', th, tv, speed]);
    return out;
  };
}
