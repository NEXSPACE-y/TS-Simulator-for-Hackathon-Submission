// 診断モード（URL に ?debug を付けたときだけ表示）。
// スマホで操作が効かなくなったときに、タッチがどこまで届いているかを画面の隅に出して調べるためのもの。
// 表示は画面を直接書き換える（Reactの状態には入れない）。タッチは受け取らない（下の操作を邪魔しない）。

import { useEffect, useRef } from 'react';
import { useSim } from './hooks';

/** URL に ?debug が付いているか */
export function isDebugMode(): boolean {
  try {
    return new URLSearchParams(window.location.search).has('debug');
  } catch {
    return false;
  }
}

const MAX_LINES = 14;

/** 出来事が起きた場所を短く書く（ボタンなら文字、それ以外は要素の名前） */
function where(target: EventTarget | null): string {
  if (target === window) return 'window';
  if (target === document) return 'document';
  if (!(target instanceof Element)) return '?';
  const button = target.closest('button');
  if (button) return `ボタン「${(button.textContent ?? '').trim().slice(0, 8)}」`;
  const cls = typeof target.className === 'string' ? target.className.split(' ')[0] : '';
  return `${target.tagName.toLowerCase()}${cls ? `.${cls}` : ''}`;
}

export function DebugLog() {
  const { input } = useSim();
  const log = useRef<HTMLPreElement>(null);
  const status = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const lines: string[] = [];
    const t0 = performance.now();
    const push = (text: string) => {
      lines.unshift(`${((performance.now() - t0) / 1000).toFixed(2)}s ${text}`);
      if (lines.length > MAX_LINES) lines.length = MAX_LINES;
      if (log.current) log.current.textContent = lines.join('\n');
    };
    const onPointer = (e: Event) => {
      const p = e as PointerEvent;
      push(`${p.type} #${p.pointerId} ${p.pointerType} ${where(p.target)}`);
    };
    // 後から受け取って、ボタン側で長押しの動きを止めたか（defaultPrevented）も出す
    const onTouch = (e: Event) => {
      const t = e as TouchEvent;
      push(`${t.type} 指${t.touches.length}本 ${where(t.target)}${t.defaultPrevented ? '（長押し止め済）' : ''}`);
    };
    const onOther = (e: Event) => {
      const extra = e.type === 'visibilitychange' ? ` ${document.visibilityState}` : ` ${where(e.target)}`;
      push(`${e.type}${extra}`);
    };
    const pointerTypes = ['pointerdown', 'pointerup', 'pointercancel', 'gotpointercapture', 'lostpointercapture'];
    const touchTypes = ['touchstart', 'touchend', 'touchcancel'];
    const otherTypes = ['click', 'contextmenu', 'selectstart'];
    for (const t of pointerTypes) window.addEventListener(t, onPointer, true);
    for (const t of touchTypes) window.addEventListener(t, onTouch, { passive: true });
    for (const t of otherTypes) window.addEventListener(t, onOther, true);
    window.addEventListener('blur', onOther);
    document.addEventListener('visibilitychange', onOther);
    const timer = window.setInterval(() => {
      if (status.current) status.current.textContent = `診断モード　押している操作：${input.holdCount}`;
    }, 250);
    push('診断モードを開始');
    return () => {
      for (const t of pointerTypes) window.removeEventListener(t, onPointer, true);
      for (const t of touchTypes) window.removeEventListener(t, onTouch);
      for (const t of otherTypes) window.removeEventListener(t, onOther, true);
      window.removeEventListener('blur', onOther);
      document.removeEventListener('visibilitychange', onOther);
      window.clearInterval(timer);
    };
  }, [input]);

  return (
    <div className="debug-log" aria-hidden="true">
      <div ref={status} />
      <pre ref={log} />
    </div>
  );
}
