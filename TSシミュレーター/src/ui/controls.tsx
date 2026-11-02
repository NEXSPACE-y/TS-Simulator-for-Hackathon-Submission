// 操作盤の部品（押している間だけ動くボタンなど）

import { useCallback, useId, useRef, useState, type ReactNode } from 'react';
import type { Frame } from '../bridge';
import { fixed } from '../bridge/format';
import type { HoldIntent, SpeedLevel } from './input';
import { setText, useFrame, useSim, useUi } from './hooks';

/** 押している間だけ操作が続くボタン */
export function HoldButton(props: {
  intent: HoldIntent;
  children: ReactNode;
  disabled?: boolean;
  className?: string;
  title?: string;
}) {
  const { input } = useSim();
  const id = useId();
  const [active, setActive] = useState(false);
  const start = (e: React.PointerEvent) => {
    if (props.disabled) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    input.press(id, props.intent);
    setActive(true);
  };
  const stop = () => {
    input.release(id);
    setActive(false);
  };
  return (
    <button
      type="button"
      className={`hold ${props.className ?? ''} ${active ? 'is-active' : ''}`}
      disabled={props.disabled}
      title={props.title}
      onPointerDown={start}
      onPointerUp={stop}
      onPointerCancel={stop}
      onLostPointerCapture={stop}
      onContextMenu={(e) => e.preventDefault()}
    >
      {props.children}
    </button>
  );
}

/** 2本を同時に回すボタン（左手親指の法則の練習用） */
export function PairButton(props: { intents: HoldIntent[]; children: ReactNode; disabled?: boolean }) {
  const { input } = useSim();
  const id = useId();
  const [active, setActive] = useState(false);
  const start = (e: React.PointerEvent) => {
    if (props.disabled) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    props.intents.forEach((it, i) => input.press(`${id}:${i}`, it));
    setActive(true);
  };
  const stop = () => {
    props.intents.forEach((_, i) => input.release(`${id}:${i}`));
    setActive(false);
  };
  return (
    <button
      type="button"
      className={`hold pair ${active ? 'is-active' : ''}`}
      disabled={props.disabled}
      onPointerDown={start}
      onPointerUp={stop}
      onPointerCancel={stop}
      onLostPointerCapture={stop}
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
