// WASM（Rustの計算コア）との窓口。src/ からWASMを呼ぶのはここだけ。
//
// 1フレームにつき1回 frame() を呼び、操作をまとめて渡して、数値の列をまとめて受け取る。
// 手順・結果などの変化の少ない状態は、uiRevision が変わったときだけ JSON で受け取る。

import init, {
  Sim,
  commandCodesJson,
  displayConfigJson,
  formatDmsSigned,
  frameLayoutJson,
  type InitInput,
} from './pkg/ts_sim.js';
import type { CommandName, DisplayConfig, UiState } from './types';

export type * from './types';

/** 毎フレームの数値の名前（Rust側 crates/wasm の visit() と同じ並び） */
export const FIELD_NAMES = [
  'feet',
  'hinges',
  'legLen',
  'planted',
  'headCenter',
  'headRot',
  'headTiltDeg',
  'headOffsetMm',
  'mounted',
  'clampTight',
  'slide',
  'screwTurns',
  'screwAngles',
  'instValid',
  'instPivot',
  'instRot',
  'laserValid',
  'laser',
  'centering',
  'centeringMm',
  'centeringDisplayMm',
  'centeringOk',
  'circValid',
  'circOffsetMm',
  'circNorm',
  'circIn',
  'tiltValid',
  'tiltX',
  'tiltY',
  'lcdDot',
  'tiltOk',
  'tiltXDms',
  'tiltYDms',
  'instHeightMm',
  'instHeightM',
  'elapsedMs',
  'elapsedParts',
  'timerRunning',
  'loops',
  'telHz',
  'telZenith',
  'telDir',
  'trunnionValid',
  'trunnion',
  'hrDms',
  'vrDms',
  'sightTarget',
  'sightDhSec',
  'sightDvSec',
  'sightOk',
  'bsTarget',
  'fsTarget',
  'uiRevision',
] as const;

export type FieldName = (typeof FIELD_NAMES)[number];
/** 名前 → 数値の列の中の位置 */
export type Offsets = Readonly<Record<FieldName, number>>;

/** 毎フレーム受け取る数値の列と、その読み方 */
export interface Frame {
  readonly data: Float64Array;
  readonly o: Offsets;
}

export type FrameListener = (f: Frame) => void;
export type UiListener = (u: UiState) => void;
/** 押している間の操作を、毎フレーム返す関数（[コード名, a, b, c] の列） */
export type HoldProvider = () => ReadonlyArray<readonly [CommandName, number, number, number]>;

/** 0＝教科書モード、1＝現場モード */
export const ModeId = { Textbook: 0, Field: 1 } as const;
export type ModeId = (typeof ModeId)[keyof typeof ModeId];

function parseLayout(): { offsets: Offsets; length: number } {
  const items = JSON.parse(frameLayoutJson()) as { name: string; offset: number; length: number }[];
  const map = new Map(items.map((i) => [i.name, i]));
  const offsets = {} as Record<FieldName, number>;
  for (const name of FIELD_NAMES) {
    const item = map.get(name);
    if (!item) throw new Error(`WASMの出力に ${name} がありません`);
    offsets[name] = item.offset;
  }
  if (items.length !== FIELD_NAMES.length) {
    throw new Error('WASMの出力の項目数が合いません');
  }
  const last = items[items.length - 1];
  return { offsets, length: last ? last.offset + last.length : 0 };
}

export class Bridge {
  private sim: Sim;
  private readonly codes: Readonly<Record<CommandName, number>>;
  private readonly stride: number;
  private queue: number[] = [];
  private holdProvider: HoldProvider | null = null;
  private frameListeners = new Set<FrameListener>();
  private uiListeners = new Set<UiListener>();
  private ui: UiState;
  private lastRevision = -1;
  private frameData: Float64Array;
  readonly offsets: Offsets;
  readonly frameLength: number;
  readonly config: DisplayConfig;
  private modeId: ModeId;

  constructor(mode: ModeId, seed: number) {
    const cc = JSON.parse(commandCodesJson()) as { stride: number; codes: Record<CommandName, number> };
    this.codes = cc.codes;
    this.stride = cc.stride;
    const layout = parseLayout();
    this.offsets = layout.offsets;
    this.frameLength = layout.length;
    this.config = JSON.parse(displayConfigJson()) as DisplayConfig;
    this.modeId = mode;
    this.sim = new Sim(mode, seed >>> 0);
    this.frameData = this.sim.frame(0, new Float64Array(0));
    this.ui = this.readUi();
  }

  get mode(): ModeId {
    return this.modeId;
  }

  /** 1回だけの操作（ボタンを押した等）。次のフレームでまとめて渡す */
  send(name: CommandName, a = 0, b = 0, c = 0): void {
    const code = this.codes[name];
    if (code === undefined) return;
    this.queue.push(code, a, b, c);
  }

  /** 押している間の操作を毎フレーム返す関数を登録する */
  setHoldProvider(p: HoldProvider | null): void {
    this.holdProvider = p;
  }

  /** 1フレーム分：操作を渡し、結果を受け取って、登録された表示に配る */
  frame(nowMs: number): Frame {
    const cmds = this.queue;
    this.queue = [];
    if (this.holdProvider) {
      for (const [name, a, b, c] of this.holdProvider()) {
        const code = this.codes[name];
        if (code !== undefined) cmds.push(code, a, b, c);
      }
    }
    if (cmds.length % this.stride !== 0) cmds.length -= cmds.length % this.stride;
    this.frameData = this.sim.frame(nowMs, Float64Array.from(cmds));
    const f: Frame = { data: this.frameData, o: this.offsets };
    const rev = this.frameData[this.offsets.uiRevision] ?? 0;
    if (rev !== this.lastRevision) {
      this.ui = this.readUi();
      for (const l of this.uiListeners) l(this.ui);
    }
    for (const l of this.frameListeners) l(f);
    return f;
  }

  /** 直近のフレーム */
  current(): Frame {
    return { data: this.frameData, o: this.offsets };
  }

  getUi(): UiState {
    return this.ui;
  }

  onFrame(l: FrameListener): () => void {
    this.frameListeners.add(l);
    return () => this.frameListeners.delete(l);
  }

  onUi(l: UiListener): () => void {
    this.uiListeners.add(l);
    return () => this.uiListeners.delete(l);
  }

  /** 最初からやり直す */
  restart(mode: ModeId, seed: number): void {
    this.sim.free();
    this.modeId = mode;
    this.sim = new Sim(mode, seed >>> 0);
    this.queue = [];
    this.lastRevision = -1;
    this.frameData = this.sim.frame(0, new Float64Array(0));
    this.ui = this.readUi();
    for (const l of this.uiListeners) l(this.ui);
  }

  private readUi(): UiState {
    this.lastRevision = this.sim.uiRevision();
    return JSON.parse(this.sim.uiStateJson()) as UiState;
  }
}

/** 度分秒の文字列（コアの関数） */
export function dmsSigned(rad: number): string {
  return formatDmsSigned(rad);
}

/**
 * WASMを読み込む。`source` を省略するとブラウザでは同じ場所の .wasm を取りに行く。
 * テスト（Node）ではファイルの中身を渡す。
 */
export async function loadBridge(mode: ModeId, seed: number, source?: InitInput): Promise<Bridge> {
  if (source !== undefined) {
    await init({ module_or_path: source });
  } else {
    await init();
  }
  return new Bridge(mode, seed);
}
