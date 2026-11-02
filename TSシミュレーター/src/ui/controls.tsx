// 操作盤の部品（押している間だけ動くボタンなど）

import { useCallback, useEffect, useId, useRef, useState, type ReactNode } from 'react';
import type { Frame } from '../bridge';
import { fixed } from '../bridge/format';
import type { HoldIntent, SpeedLevel } from './input';
import { setText, useFrame, useSim, useUi } from './hooks';

/** 押している指・マウス（どの指で押したかを覚えておき、その指を離したときだけ止める） */
interface HoldOwner {
  pointerId: number | null;
  touchId: number | null;
  count: number;
}

/**
 * 押している間だけ操作を送るボタンの仕組み（HoldButton・PairButton で共通）。
 * - スマホ：押し始め（touchstart）で、ブラウザの長押しの動き（文字の選択・メニューなど）を止める
 * - 指を捕まえる処理（ポインターキャプチャ）は使わない。iPhone でタッチが効かなくなる原因になりうるため
 * - ボタンの外で離しても、タッチが取り消されても、画面を離れても、必ず止める
 */
function useHold(intents: HoldIntent[], disabled: boolean | undefined) {
  const { input } = useSim();
  const id = useId();
  const ref = useRef<HTMLButtonElement>(null);
  const [active, setActive] = useState(false);
  const latest = useRef({ intents, disabled });
  latest.current = { intents, disabled };
  const owner = useRef<HoldOwner | null>(null);

  const release = useCallback(() => {
    const o = owner.current;
    if (!o) return;
    owner.current = null;
    for (let i = 0; i < o.count; i++) input.release(`${id}:${i}`);
    setActive(false);
  }, [input, id]);

  // タッチでは pointerdown と touchstart の両方が来る。どちらか先に来たほうで押し始める
  const begin = useCallback(
    (pointerId: number | null, touchId: number | null) => {
      const o = owner.current;
      if (o) {
        if (o.pointerId === null) o.pointerId = pointerId;
        if (o.touchId === null) o.touchId = touchId;
        return;
      }
      const { intents: list, disabled: off } = latest.current;
      if (off) return;
      list.forEach((it, i) => input.press(`${id}:${i}`, it));
      owner.current = { pointerId, touchId, count: list.length };
      setActive(true);
    },
    [input, id],
  );

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const onTouchStart = (e: TouchEvent) => {
      e.preventDefault();
      begin(null, e.changedTouches[0]?.identifier ?? null);
    };
    el.addEventListener('touchstart', onTouchStart, { passive: false });
    return () => el.removeEventListener('touchstart', onTouchStart);
  }, [begin]);

  useEffect(() => {
    const onPointerEnd = (e: PointerEvent) => {
      if (owner.current?.pointerId === e.pointerId) release();
    };
    const onTouchEnd = (e: TouchEvent) => {
      const t = owner.current?.touchId;
      if (t === null || t === undefined) return;
      for (const ch of Array.from(e.changedTouches)) {
        if (ch.identifier === t) {
          release();
          return;
        }
      }
    };
    const onHidden = () => {
      if (document.visibilityState !== 'visible') release();
    };
    window.addEventListener('pointerup', onPointerEnd, true);
    window.addEventListener('pointercancel', onPointerEnd, true);
    window.addEventListener('touchend', onTouchEnd, true);
    window.addEventListener('touchcancel', onTouchEnd, true);
    window.addEventListener('blur', release);
    document.addEventListener('visibilitychange', onHidden);
    return () => {
      window.removeEventListener('pointerup', onPointerEnd, true);
      window.removeEventListener('pointercancel', onPointerEnd, true);
      window.removeEventListener('touchend', onTouchEnd, true);
      window.removeEventListener('touchcancel', onTouchEnd, true);
      window.removeEventListener('blur', release);
      document.removeEventListener('visibilitychange', onHidden);
    };
  }, [release]);

  // 押したまま消えたり使えなくなったりしたボタンが、操作を送り続けないようにする
  useEffect(() => release, [release]);
  useEffect(() => {
    if (disabled) release();
  }, [disabled, release]);

  const onPointerDown = (e: React.PointerEvent) => begin(e.pointerId, null);
  return { ref, active, onPointerDown };
}

/** 押している間だけ操作が続くボタン */
export function HoldButton(props: {
  intent: HoldIntent;
  children: ReactNode;
  disabled?: boolean;
  className?: string;
  title?: string;
}) {
  const hold = useHold([props.intent], props.disabled);
  return (
    <button
      ref={hold.ref}
      type="button"
      className={`hold ${props.className ?? ''} ${hold.active ? 'is-active' : ''}`}
      disabled={props.disabled}
      title={props.title}
      onPointerDown={hold.onPointerDown}
      onContextMenu={(e) => e.preventDefault()}
    >
      {props.children}
    </button>
  );
}

/** 2本を同時に回すボタン（左手親指の法則の練習用） */
export function PairButton(props: { intents: HoldIntent[]; children: ReactNode; disabled?: boolean }) {
  const hold = useHold(props.intents, props.disabled);
  return (
    <button
      ref={hold.ref}
      type="button"
      className={`hold pair ${hold.active ? 'is-active' : ''}`}
      disabled={props.disabled}
      onPointerDown={hold.onPointerDown}
      onContextMenu={(e) => e.preventDefault()}
    >
      {props.children}
    </button>
  );
}

/** 速さ（粗・中・微） */
export function SpeedSelector(props: { labels?: [string, string, string] }) {
  const { input } = useSim();
  const [speed, setSpeed] = useState<SpeedLevel>(input.speed);
  const labels = props.labels ?? ['粗', '中', '微'];
  return (
    <div className="speed" role="radiogroup" aria-label="操作の速さ">
      <span className="speed-label">速さ</span>
      {([0, 1, 2] as const).map((s) => (
        <button
          key={s}
          type="button"
          role="radio"
          aria-checked={speed === s}
          className={speed === s ? 'is-on' : ''}
          onClick={() => {
            input.setSpeed(s);
            setSpeed(s);
          }}
        >
          {labels[s]}
        </button>
      ))}
      <span className="hint">Shiftキーを押している間は「微」</span>
    </div>
  );
}

/** 上下左右（奥・手前・左・右）のボタン */
export function DirPad(props: {
  make: (dx: number, dy: number) => HoldIntent;
  disabled?: boolean;
  labels?: { up: string; down: string; left: string; right: string };
}) {
  const l = props.labels ?? { up: '奥へ', down: '手前へ', left: '左へ', right: '右へ' };
  return (
    <div className="dirpad">
      <span />
      <HoldButton intent={props.make(0, 1)} disabled={props.disabled}>
        ▲<small>{l.up}</small>
      </HoldButton>
      <span />
      <HoldButton intent={props.make(-1, 0)} disabled={props.disabled}>
        ◀<small>{l.left}</small>
      </HoldButton>
      <span className="dirpad-center">矢印キー</span>
      <HoldButton intent={props.make(1, 0)} disabled={props.disabled}>
        ▶<small>{l.right}</small>
      </HoldButton>
      <span />
      <HoldButton intent={props.make(0, -1)} disabled={props.disabled}>
        ▼<small>{l.down}</small>
      </HoldButton>
      <span />
    </div>
  );
}

export const LEG_NAMES = ['脚1（奥）', '脚2（左手前）', '脚3（右手前）'] as const;
export const SCREW_NAMES = ['A（左手前）', 'B（右手前）', 'C（奥）'] as const;

/** 脚を踏み込むボタン */
export function PressLegButtons(props: { disabled?: boolean }) {
  const { bridge } = useSim();
  const ui = useUi();
  return (
    <div className="button-row">
      {LEG_NAMES.map((name, i) => (
        <button
          key={name}
          type="button"
          disabled={props.disabled || ui.planted[i]}
          className={ui.planted[i] ? 'is-done' : ''}
          onClick={() => bridge.send('pressLeg', i)}
        >
          {ui.planted[i] ? `${name} 踏み込み済み` : `${name}を踏み込む`}
        </button>
      ))}
    </div>
  );
}

/** 脚の伸縮 */
export function LegLengthControls() {
  const refs = useRef<(HTMLSpanElement | null)[]>([]);
  useFrame(
    useCallback((f: Frame) => {
      for (let i = 0; i < 3; i++) setText(refs.current[i] ?? null, `${fixed(f.data[f.o.legLen + i], 0)} mm`);
    }, []),
  );
  const keys = [
    ['1', 'Q'],
    ['2', 'W'],
    ['3', 'E'],
  ];
  return (
    <div className="leg-controls">
      {LEG_NAMES.map((name, i) => (
        <div className="leg-row" key={name}>
          <span className="leg-name">{name}</span>
          <span className="leg-len" ref={(el) => {
            refs.current[i] = el;
          }} />
          <HoldButton intent={{ kind: 'legLength', leg: i, extend: true }}>
            伸ばす<kbd>{keys[i]?.[0]}</kbd>
          </HoldButton>
          <HoldButton intent={{ kind: 'legLength', leg: i, extend: false }}>
            縮める<kbd>{keys[i]?.[1]}</kbd>
          </HoldButton>
        </div>
      ))}
    </div>
  );
}

/** 整準ネジ3本（親指の向きで回す） */
export function ScrewControls(props: { disabled?: boolean }) {
  const keys = [
    ['A', 'S'],
    ['K', 'L'],
    ['Z', 'X'],
  ];
  return (
    <div className="screws">
      <ScrewDiagram />
      {SCREW_NAMES.map((name, i) => (
        <div className="screw-row" key={name}>
          <span className="screw-name">{name}</span>
          <HoldButton intent={{ kind: 'screw', screw: i, thumb: -1 }} disabled={props.disabled}>
            ◀ 親指を左<kbd>{keys[i]?.[0]}</kbd>
          </HoldButton>
          <HoldButton intent={{ kind: 'screw', screw: i, thumb: 1 }} disabled={props.disabled}>
            親指を右 ▶<kbd>{keys[i]?.[1]}</kbd>
          </HoldButton>
        </div>
      ))}
      <div className="pair-row">
        <PairButton
          disabled={props.disabled}
          intents={[
            { kind: 'screw', screw: 0, thumb: 1 },
            { kind: 'screw', screw: 1, thumb: -1 },
          ]}
        >
          A・B を内側へ<small>S＋K</small>
        </PairButton>
        <PairButton
          disabled={props.disabled}
          intents={[
            { kind: 'screw', screw: 0, thumb: -1 },
            { kind: 'screw', screw: 1, thumb: 1 },
          ]}
        >
          A・B を外側へ<small>A＋L</small>
        </PairButton>
      </div>
      <p className="note">
        左手親指の法則：2本を同時に内側または外側へ回すと、気泡は<strong>左手の親指を動かす向き</strong>に動きます。
      </p>
    </div>
  );
}

/** 整準ネジの並び（上から見た図） */
function ScrewDiagram() {
  return (
    <svg className="screw-diagram" viewBox="0 0 120 92" aria-label="整準ネジの並び（上から見た図）">
      <circle cx="60" cy="44" r="34" className="sd-base" />
      <circle cx="31" cy="61" r="9" className="sd-screw" />
      <circle cx="89" cy="61" r="9" className="sd-screw" />
      <circle cx="60" cy="10" r="9" className="sd-screw" />
      <text x="31" y="65" className="sd-label">A</text>
      <text x="89" y="65" className="sd-label">B</text>
      <text x="60" y="14" className="sd-label">C</text>
      <text x="60" y="90" className="sd-you">手前（あなた）</text>
    </svg>
  );
}

/** 定心桿 */
export function ClampControl() {
  const { bridge } = useSim();
  const ui = useUi();
  return (
    <div className="clamp">
      <span className={`clamp-state ${ui.clampTight ? 'tight' : 'loose'}`}>
        定心桿：{ui.clampTight ? '締めている' : '緩めている'}
      </span>
      <button type="button" disabled={!ui.mounted} onClick={() => bridge.send('setClamp', ui.clampTight ? 0 : 1)}>
        {ui.clampTight ? '緩める' : '締める'}
        <kbd>T</kbd>
      </button>
    </div>
  );
}

/** 見出しつきのまとまり */
export function Section(props: { title: string; children: ReactNode }) {
  return (
    <section className="op-section">
      <h3>{props.title}</h3>
      {props.children}
    </section>
  );
}
