// 中央：3Dビュー。右上に拡大表示（求心／円形気泡／液晶、観測では望遠鏡の視野）、左下に視点切替（SPEC 4章）。

import { useCallback, useEffect, useRef, useState } from 'react';
import type { Frame } from '../bridge';
import { dmsSignedAt, fixed, flag } from '../bridge/format';
import { Engine, viewForStep, type Quality, type ViewName } from '../render/engine';
import { setAttr, setText, useFrame, useSim, useUi } from './hooks';

const VIEWS: { id: ViewName; label: string }[] = [
  { id: 'overview', label: '全体' },
  { id: 'instrument', label: '器械' },
  { id: 'feet', label: '足元' },
  { id: 'lcd', label: '液晶' },
];

export function Viewport(props: { quality: Quality }) {
  const { bridge } = useSim();
  const ui = useUi();
  const host = useRef<HTMLDivElement>(null);
  const scope = useRef<HTMLDivElement>(null);
  const engine = useRef<Engine | null>(null);
  const [view, setViewState] = useState<ViewName>('overview');
  const viewRef = useRef<ViewName>('overview');
  const [error, setError] = useState<string | null>(null);
  const changeView = useCallback((v: ViewName) => {
    viewRef.current = v;
    setViewState(v);
    engine.current?.setView(v);
  }, []);

  useEffect(() => {
    if (!host.current) return;
    try {
      const e = new Engine(host.current, bridge, props.quality);
      // 画質の切り替えで作り直したときも、今の視点と望遠鏡の視野を引き継ぐ
      e.setView(viewRef.current, true);
      e.setTelescopeElement(scope.current);
      engine.current = e;
    } catch (e) {
      setError(`3Dを表示できませんでした（WebGLが使えない可能性があります）：${String(e)}`);
    }
    return () => {
      engine.current?.dispose();
      engine.current = null;
    };
  }, [bridge, props.quality]);

  // ステップが変わったら、カメラの寄り先を変える
  useEffect(() => {
    changeView(viewForStep(ui.mode, ui.currentStep));
  }, [ui.mode, ui.currentStep, ui.phase, changeView]);

  const observing = ui.phase === 'observation' && ui.currentStep >= 9;
  useEffect(() => {
    engine.current?.setTelescopeElement(observing ? scope.current : null);
  }, [observing]);

  return (
    <div className="viewport">
      <div className="three-host" ref={host} />
      {error && <div className="view-error">{error}</div>}
      {ui.phase !== 'observation' && <ZoomPanels />}
      {observing && <TelescopeOverlay ref={scope} peep={ui.observation.peepSight} />}
      <div className="view-switch" role="group" aria-label="視点切替">
        {VIEWS.map((v) => (
          <button
            key={v.id}
            type="button"
            className={view === v.id ? 'is-on' : ''}
            onClick={() => changeView(v.id)}
          >
            {v.label}
          </button>
        ))}
      </div>
    </div>
  );
}

/** 求心の拡大表示の倍率 [px/mm]（表示の大きさだけ） */
const PX_PER_MM = 8;
const ZOOM_R = 56;

function ZoomPanels() {
  const { bridge } = useSim();
  const cfg = bridge.config;
  const laser = useRef<SVGCircleElement>(null);
  const centText = useRef<SVGTextElement>(null);
  const bubble = useRef<SVGCircleElement>(null);
  const dot = useRef<SVGCircleElement>(null);
  const xText = useRef<SVGTextElement>(null);
  const yText = useRef<SVGTextElement>(null);

  useFrame(
    useCallback((f: Frame) => {
      const d = f.data;
      const o = f.o;
      // 求心：レーザー点と測点（北＝上）
      if (flag(f, o.laserValid)) {
        // 枠の外に出るときは、向きを保ったまま枠の端に置く（表示の縮尺だけ）
        const rx = (d[o.centering] ?? 0) * PX_PER_MM;
        const ry = -(d[o.centering + 1] ?? 0) * PX_PER_MM;
        const k = Math.min(1, ZOOM_R / Math.max(Math.abs(rx), Math.abs(ry), 1e-9));
        const x = rx * k;
        const y = ry * k;
        setAttr(laser.current, 'cx', x.toFixed(2));
        setAttr(laser.current, 'cy', y.toFixed(2));
        setAttr(laser.current, 'visibility', 'visible');
        setText(centText.current, `${fixed(d[o.centeringDisplayMm], 1)} mm`);
      } else {
        setAttr(laser.current, 'visibility', 'hidden');
        setText(centText.current, '---');
      }
      // 円形気泡
      if (flag(f, o.circValid)) {
        setAttr(bubble.current, 'cx', ((d[o.circNorm] ?? 0) * 40).toFixed(2));
        setAttr(bubble.current, 'cy', (-(d[o.circNorm + 1] ?? 0) * 40).toFixed(2));
        setAttr(bubble.current, 'visibility', 'visible');
      } else {
        setAttr(bubble.current, 'visibility', 'hidden');
      }
      // 液晶◎
      if (flag(f, o.tiltValid)) {
        setAttr(dot.current, 'cx', ((d[o.lcdDot] ?? 0) * 50).toFixed(2));
        setAttr(dot.current, 'cy', (-(d[o.lcdDot + 1] ?? 0) * 50).toFixed(2));
        setAttr(dot.current, 'visibility', 'visible');
        setText(xText.current, `X: ${dmsSignedAt(f, o.tiltXDms)}`);
        setText(yText.current, `Y: ${dmsSignedAt(f, o.tiltYDms)}`);
      } else {
        setAttr(dot.current, 'visibility', 'hidden');
        setText(xText.current, 'X: ---');
        setText(yText.current, 'Y: ---');
      }
    }, []),
  );

  const tolR = cfg.centeringToleranceMm * PX_PER_MM;
  return (
    <div className="zooms">
      <figure className="zoom">
        <figcaption>求心（上＝奥）</figcaption>
        <svg viewBox="-64 -64 128 140" aria-label="レーザー求心の拡大">
          <rect x="-64" y="-64" width="128" height="128" className="z-ground" />
          {[1, 2, 3, 4, 5, 6, 7].map((mm) => (
            <circle key={mm} r={mm * PX_PER_MM} className={mm % 5 === 0 ? 'z-grid strong' : 'z-grid'} />
          ))}
          <circle r="12" className="z-mark" />
          <line x1="-12" y1="0" x2="12" y2="0" className="z-cross" />
          <line x1="0" y1="-12" x2="0" y2="12" className="z-cross" />
          <circle r={tolR} className="z-tol" />
          <circle ref={laser} r="2.6" className="z-laser" />
          <text ref={centText} x="0" y="74" className="z-text" />
        </svg>
      </figure>
      <figure className="zoom">
        <figcaption>円形気泡</figcaption>
        <svg viewBox="-64 -64 128 140" aria-label="円形気泡の拡大">
          <circle r="54" className="c-vial" />
          <circle r={cfg.circularInRatio * 40 + 10} className="c-ring" />
          <circle ref={bubble} r="10" className="c-bubble" />
          <text x="0" y="74" className="z-text small">中心の円に入れば合格</text>
        </svg>
      </figure>
      <figure className="zoom">
        <figcaption>液晶（電子気泡）</figcaption>
        <svg viewBox="-64 -64 128 156" aria-label="液晶の電子気泡の拡大">
          <rect x="-64" y="-64" width="128" height="156" rx="6" className="l-bg" />
          <circle r="50" className="l-line" />
          <circle r={Math.max(2, cfg.lcdInnerRatio * 50)} className="l-line" />
          <line x1="-50" y1="0" x2="50" y2="0" className="l-thin" />
          <line x1="0" y1="-50" x2="0" y2="50" className="l-thin" />
          <circle ref={dot} r="3.5" className="l-dot" />
          <text ref={xText} x="-56" y="68" className="l-text" />
          <text ref={yText} x="-56" y="86" className="l-text" />
        </svg>
      </figure>
    </div>
  );
}

type ScopeSize = 'small' | 'medium' | 'large';
const SCOPE_SIZES: { id: ScopeSize; label: string }[] = [
  { id: 'small', label: '小' },
  { id: 'medium', label: '中' },
  { id: 'large', label: '大' },
];
/** ターゲットのぼけの見た目の最大 [px]（表示の大きさだけ） */
const TARGET_BLUR_MAX_PX = 10;
/** 十字線のぼけの見た目の最大（視野の座標。直径100） */
const RETICLE_BLUR_MAX = 1.6;

function TelescopeOverlay(props: { ref: React.Ref<HTMLDivElement>; peep: boolean }) {
  const [size, setSize] = useState<ScopeSize>('medium');
  const blur = useRef<HTMLDivElement>(null);
  const lastBlur = useRef('');
  const reticle = useRef<SVGGElement>(null);
  const reticleBlur = useRef<SVGFEGaussianBlurElement>(null);
  const drag = useRef<{ id: number; x: number; y: number; left: number; top: number } | null>(null);

  // ぼけの量はコアが決める（0〜1）。ここでは見た目の大きさに置き換えるだけ
  useFrame(
    useCallback(
      (f: Frame) => {
        const tb = props.peep ? 0 : (f.data[f.o.targetBlur] ?? 0) * TARGET_BLUR_MAX_PX;
        const filter = tb > 0.05 ? `blur(${tb.toFixed(1)}px)` : 'none';
        const el = blur.current;
        if (el && lastBlur.current !== filter) {
          lastBlur.current = filter;
          el.style.backdropFilter = filter;
          el.style.setProperty('-webkit-backdrop-filter', filter);
        }
        const rb = (f.data[f.o.reticleBlur] ?? 0) * RETICLE_BLUR_MAX;
        setAttr(reticleBlur.current, 'stdDeviation', rb.toFixed(2));
        setAttr(reticle.current, 'filter', rb > 0.01 ? 'url(#scope-reticle-blur)' : 'none');
      },
      [props.peep],
    ),
  );

  // 視野の枠は、下の帯をつかんで動かせる（画面の中だけ）
  const onDown = (e: React.PointerEvent<HTMLDivElement>) => {
    const box = e.currentTarget.parentElement;
    const host = box?.parentElement;
    if (!box || !host) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    const r = box.getBoundingClientRect();
    const h = host.getBoundingClientRect();
    drag.current = { id: e.pointerId, x: e.clientX, y: e.clientY, left: r.left - h.left, top: r.top - h.top };
  };
  const onMove = (e: React.PointerEvent<HTMLDivElement>) => {
    const d = drag.current;
    const box = e.currentTarget.parentElement;
    const host = box?.parentElement;
    if (!d || d.id !== e.pointerId || !box || !host) return;
    const maxLeft = Math.max(0, host.clientWidth - box.offsetWidth);
    const maxTop = Math.max(0, host.clientHeight - box.offsetHeight);
    box.style.left = `${Math.min(Math.max(0, d.left + e.clientX - d.x), maxLeft)}px`;
    box.style.top = `${Math.min(Math.max(0, d.top + e.clientY - d.y), maxTop)}px`;
    box.style.right = 'auto';
  };
  const onUp = () => {
    drag.current = null;
  };
  const changeSize = (e: React.MouseEvent<HTMLButtonElement>, s: ScopeSize) => {
    // 大きさを変えたら、右上の元の位置に戻す
    const box = e.currentTarget.closest<HTMLElement>('.scope');
    if (box) {
      box.style.left = '';
      box.style.top = '';
      box.style.right = '';
    }
    setSize(s);
  };

  const label = props.peep ? '照準器の視野' : '望遠鏡の視野';
  return (
    <div className={`scope scope-${size} ${props.peep ? 'is-peep' : ''}`} ref={props.ref} aria-label={label}>
      <div className="scope-blur" ref={blur} />
      <svg viewBox="-50 -50 100 100" preserveAspectRatio="xMidYMid meet" className="scope-cross">
        <defs>
          <filter id="scope-reticle-blur" x="-20%" y="-20%" width="140%" height="140%">
            <feGaussianBlur ref={reticleBlur} stdDeviation="0" />
          </filter>
        </defs>
        <path
          className="scope-mask"
          fillRule="evenodd"
          d="M-51,-51 H51 V51 H-51 Z M0,-48 A48,48 0 1,0 0,48 A48,48 0 1,0 0,-48 Z"
        />
        <circle r="48" className="scope-ring" />
        {props.peep ? (
          <path className="peep-mark" d="M0,-5 L4.5,3 L-4.5,3 Z" />
        ) : (
          <g ref={reticle}>
            <line x1="-48" y1="0" x2="48" y2="0" />
            <line x1="0" y1="-48" x2="0" y2="48" />
            <line x1="-8" y1="-3" x2="-8" y2="3" />
            <line x1="8" y1="-3" x2="8" y2="3" />
            <line x1="-3" y1="-8" x2="3" y2="-8" />
            <line x1="-3" y1="8" x2="3" y2="8" />
          </g>
        )}
      </svg>
      <div
        className="scope-bar"
        onPointerDown={onDown}
        onPointerMove={onMove}
        onPointerUp={onUp}
        onPointerCancel={onUp}
        title="つかんで動かせます"
      >
        <span className="scope-label">{label}</span>
        <span className="scope-sizes" role="group" aria-label="視野の大きさ">
          {SCOPE_SIZES.map((s) => (
            <button
              key={s.id}
              type="button"
              className={size === s.id ? 'is-on' : ''}
              onPointerDown={(e) => e.stopPropagation()}
              onClick={(e) => changeSize(e, s.id)}
            >
              {s.label}
            </button>
          ))}
        </span>
      </div>
    </div>
  );
}
