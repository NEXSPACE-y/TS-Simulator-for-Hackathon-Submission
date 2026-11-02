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
  const [view, setView] = useState<ViewName>('overview');
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!host.current) return;
    try {
      engine.current = new Engine(host.current, bridge, props.quality);
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
    const v = viewForStep(ui.mode, ui.currentStep);
    setView(v);
    engine.current?.setView(v);
  }, [ui.mode, ui.currentStep, ui.phase]);

  const observing = ui.phase === 'observation' && ui.currentStep >= 9;
  useEffect(() => {
    engine.current?.setTelescopeElement(observing ? scope.current : null);
  }, [observing]);

  return (
    <div className="viewport">
      <div className="three-host" ref={host} />
      {error && <div className="view-error">{error}</div>}
      {ui.phase !== 'observation' && <ZoomPanels />}
      {observing && <TelescopeOverlay ref={scope} />}
      <div className="view-switch" role="group" aria-label="視点切替">
        {VIEWS.map((v) => (
          <button
            key={v.id}
            type="button"
            className={view === v.id ? 'is-on' : ''}
            onClick={() => {
              setView(v.id);
              engine.current?.setView(v.id);
            }}
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

function TelescopeOverlay(props: { ref: React.Ref<HTMLDivElement> }) {
  return (
    <div className="scope" ref={props.ref} aria-label="望遠鏡の視野">
      <svg viewBox="-50 -50 100 100" preserveAspectRatio="xMidYMid meet" className="scope-cross">
        <circle r="49" className="scope-ring" />
        <line x1="-49" y1="0" x2="49" y2="0" />
        <line x1="0" y1="-49" x2="0" y2="49" />
        <line x1="-8" y1="-3" x2="-8" y2="3" />
        <line x1="8" y1="-3" x2="8" y2="3" />
        <line x1="-3" y1="-8" x2="3" y2="-8" />
        <line x1="-3" y1="8" x2="3" y2="8" />
      </svg>
      <span className="scope-label">望遠鏡の視野</span>
    </div>
  );
}
