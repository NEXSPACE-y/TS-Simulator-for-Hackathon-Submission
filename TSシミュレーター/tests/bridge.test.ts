// WASMとのつなぎ目の確認（SPEC 11章）：WASMが読み込めること、操作を渡すと結果が返ること
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { beforeAll, describe, expect, it } from 'vitest';
import { Bridge, FIELD_NAMES, LangId, ModeId, dmsSigned, loadBridge } from '../src/bridge';

const wasmPath = fileURLToPath(new URL('../src/bridge/pkg/ts_sim_bg.wasm', import.meta.url));

let bridge: Bridge;

beforeAll(async () => {
  bridge = await loadBridge(ModeId.Field, 7, readFileSync(wasmPath));
});

describe('WASMとのつなぎ目', () => {
  it('WASMが読み込め、毎フレームの数値の並びがそろっている', () => {
    const f = bridge.current();
    expect(f.data.length).toBe(bridge.frameLength);
    for (const name of FIELD_NAMES) {
      expect(bridge.offsets[name]).toBeGreaterThanOrEqual(0);
    }
    // 現場モードは器械が載った状態で始まる
    expect(f.data[f.o.mounted]).toBe(1);
    expect(f.data[f.o.clampTight]).toBe(1);
  });

  it('整準ネジの操作を渡すと、電子気泡の値が変わって返る', () => {
    bridge.frame(1000);
    const before = bridge.current().data[bridge.offsets.tiltX];
    bridge.setHoldProvider(() => [
      ['turnScrew', 0, 1, 1],
      ['turnScrew', 1, -1, 1],
    ]);
    bridge.frame(1016);
    bridge.frame(1032);
    bridge.setHoldProvider(null);
    const f = bridge.current();
    const after = f.data[f.o.tiltX];
    // A・Bを内側へ回すと、気泡は左手親指の向き（＋X）へ動く
    expect(after).toBeGreaterThan(before);
    // 最初の操作でタイマーが動き出す
    expect(f.data[f.o.timerRunning]).toBe(1);
  });

  it('1回だけの操作を渡すと、画面の状態（JSON）が更新される', () => {
    const rev = bridge.getUi();
    expect(rev.mode).toBe('field');
    bridge.send('selectStep', 6);
    bridge.frame(1100);
    expect(bridge.getUi().currentStep).toBe(6);
    bridge.send('setClamp', 0);
    bridge.frame(1116);
    expect(bridge.getUi().clampTight).toBe(false);
    expect(bridge.current().data[bridge.offsets.clampTight]).toBe(0);
  });

  it('据付完了で判定結果が返る（定心桿が緩んだままなら不合格）', () => {
    bridge.send('completeSetup');
    bridge.frame(1200);
    const ui = bridge.getUi();
    expect(ui.phase).toBe('setupResult');
    expect(ui.judgement?.passed).toBe(false);
    expect(ui.judgement?.reasons.some((r) => r.includes('定心桿'))).toBe(true);
    // AI先輩の講評に渡す事実も、コアから受け取れる
    const facts = bridge.aiFacts('review');
    expect(facts?.find((f) => f.k === '結果')?.v).toBe('不合格');
  });

  it('度分秒の変換はコアの関数を使う', () => {
    const sec = Math.PI / (180 * 3600);
    expect(dmsSigned(12 * sec)).toBe('+0°00′12″');
    expect(dmsSigned(-8 * sec)).toBe('−0°00′08″');
  });

  it('最初からやり直すと教科書モードの状態になる', () => {
    bridge.restart(ModeId.Textbook, 3);
    const ui = bridge.getUi();
    expect(ui.mode).toBe('textbook');
    expect(ui.mounted).toBe(false);
    expect(ui.goal).not.toBeNull();
  });

  it('器械を載せる前は、脚頭の穴からのぞいた見え方が届き、載せるとレーザーに戻る', () => {
    bridge.restart(ModeId.Textbook, 3);
    bridge.frame(2000);
    let f = bridge.current();
    expect(f.data[f.o.peekValid]).toBe(1);
    expect(f.data[f.o.laserValid]).toBe(0);
    // のぞいた測点の位置は、脚頭の中心の反対向き（コアで計算した値がそのまま届く）
    expect(f.data[f.o.peek]).toBeCloseTo(-f.data[f.o.headCenter], 6);
    expect(f.data[f.o.peek + 1]).toBeCloseTo(-f.data[f.o.headCenter + 1], 6);
    expect(bridge.config.headHoleViewMm).toBeGreaterThan(bridge.config.textbookRoughCenterMm);
    bridge.send('mountInstrument');
    bridge.frame(2016);
    f = bridge.current();
    expect(f.data[f.o.peekValid]).toBe(0);
    expect(f.data[f.o.laserValid]).toBe(1);
  });

  it('言語を英語にすると、コアの文も英語で届き、やり直しても英語のまま', () => {
    bridge.restart(ModeId.Field, 5);
    expect(bridge.getUi().lang).toBe('ja');
    bridge.setLang(LangId.En);
    let ui = bridge.getUi();
    expect(ui.lang).toBe('en');
    expect(ui.steps[0]?.label).toBe('Set the tripod over the point');
    bridge.restart(ModeId.Textbook, 5);
    ui = bridge.getUi();
    expect(ui.lang).toBe('en');
    expect(ui.goal?.text).toMatch(/^Press in all three legs/);
    // AIに渡す事実は日本語のまま（AIには、選んだ言語で答えさせる）
    expect(bridge.aiFacts('status')?.some((f) => f.k === 'いまの手順')).toBe(true);
    bridge.setLang(LangId.Ja);
    expect(bridge.getUi().steps[0]?.label).toBe('三脚を測点の上におおよそ置き、脚頭をだいたい水平に');
  });
});
