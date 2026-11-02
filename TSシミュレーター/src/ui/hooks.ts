// React から bridge を使うための小さな道具。
// 毎フレーム変わる数値は React の状態に入れず、ref で表示を直接書き換える（SPEC 2章）。

import { createContext, useContext, useEffect, useSyncExternalStore } from 'react';
import type { Bridge, Frame, UiState } from '../bridge';
import type { InputManager } from './input';

export interface SimContextValue {
  bridge: Bridge;
  input: InputManager;
}

export const SimContext = createContext<SimContextValue | null>(null);

export function useSim(): SimContextValue {
  const v = useContext(SimContext);
  if (!v) throw new Error('SimContext がありません');
  return v;
}

/** 手順・結果など、変化の少ない状態（変わったときだけ再描画） */
export function useUi(): UiState {
  const { bridge } = useSim();
  return useSyncExternalStore(
    (cb) => bridge.onUi(cb),
    () => bridge.getUi(),
  );
}

/** 毎フレーム呼ばれる関数を登録する（中で ref の表示を書き換える） */
export function useFrame(cb: (f: Frame) => void): void {
  const { bridge } = useSim();
  useEffect(() => {
    cb(bridge.current());
    return bridge.onFrame(cb);
  }, [bridge, cb]);
}

/** 文字が変わったときだけ書き換える */
export function setText(el: HTMLElement | SVGElement | null, text: string): void {
  if (el && el.textContent !== text) el.textContent = text;
}

/** SVGの属性を、変わったときだけ書き換える */
export function setAttr(el: Element | null, name: string, value: string): void {
  if (el && el.getAttribute(name) !== value) el.setAttribute(name, value);
}
