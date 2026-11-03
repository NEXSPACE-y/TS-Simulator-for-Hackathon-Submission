// 第2部：観測（電子平板・視準・測量）の操作盤（SPEC 8章）
// 電子平板の画面は特定の製品を真似ない、汎用のデザイン。

import { useCallback, useRef, useState } from 'react';
import type { Frame } from '../bridge';
import { dmsCircleAt, dmsSignedAt, fixed, flag } from '../bridge/format';
import { AiExplain } from './AiExplain';
import { AiInstruction, InstructionResultBox } from './AiInstruction';
import { DirPad, HoldButton, Section, SpeedSelector } from './controls';
import { setText, useFrame, useSim, useUi } from './hooks';
import { useT } from './i18n';

export function ObservationPanel() {
  const ui = useUi();
  return (
    <>
      <InstructionResultBox />
      {ui.currentStep === 8 ? <PlaneTable /> : <Telescope />}
    </>
  );
}

function PlaneTable() {
  const ui = useUi();
  const t = useT();
  const { bridge } = useSim();
  const o = ui.observation;
  const [station, setStation] = useState<number>(o.station ?? 0);
  const [backsight, setBacksight] = useState<number>(o.backsight ?? -1);
  const [height, setHeight] = useState<string>(o.heightText ?? '');
  const measured = useRef<number>(Number.NaN);
  useFrame(
    useCallback((f: Frame) => {
      measured.current = f.data[f.o.instHeightM] ?? Number.NaN;
    }, []),
  );
  const heightValue = Number(height);
  const canSet = backsight >= 0 && station !== backsight && height.trim() !== '' && Number.isFinite(heightValue);
  return (
    <>
      <AiInstruction />
      <div className="tablet">
        <div className="tablet-title">{t.tablet}</div>
        <label className="field">
          <span>{t.station}</span>
          <select value={station} onChange={(e) => setStation(Number(e.target.value))}>
            {o.points.map((p, i) => (
              <option key={p.name} value={i}>
                {p.name}
              </option>
            ))}
          </select>
        </label>
        <label className="field">
          <span>{t.backsight}</span>
          <select value={backsight} onChange={(e) => setBacksight(Number(e.target.value))}>
            <option value={-1}>{t.choose}</option>
            {o.points.map((p, i) => (
              <option key={p.name} value={i}>
                {p.name}
              </option>
            ))}
          </select>
        </label>
        <label className="field">
          <span>{t.instHeight}</span>
          <input inputMode="decimal" value={height} placeholder={t.heightPlaceholder} onChange={(e) => setHeight(e.target.value)} />
          <button type="button" onClick={() => setHeight(fixed(measured.current, 3))}>
            {t.measureTape}
          </button>
        </label>
        <table className="points">
          <thead>
            <tr>
              <th>{t.pointName}</th>
              <th>X (m)</th>
              <th>Y (m)</th>
              <th>H (m)</th>
            </tr>
          </thead>
          <tbody>
            {o.points.map((p) => (
              <tr key={p.name}>
                <td>{p.name}</td>
                <td>{p.x}</td>
                <td>{p.y}</td>
                <td>{p.h}</td>
              </tr>
            ))}
          </tbody>
        </table>
        <p className="note">{t.tabletNote}</p>
        <button
          type="button"
          className="primary"
          disabled={!canSet}
          onClick={() => bridge.send('setStation', station, backsight, heightValue)}
        >
          {t.set}
        </button>
      </div>
    </>
  );
}

function Telescope() {
  const ui = useUi();
  const t = useT();
  const { bridge } = useSim();
  const ha = useRef<HTMLSpanElement>(null);
  const va = useRef<HTMLSpanElement>(null);
  const dh = useRef<HTMLSpanElement>(null);
  const dv = useRef<HTMLSpanElement>(null);
  const ok = useRef<HTMLSpanElement>(null);
  const reticle = useRef<HTMLSpanElement>(null);
  const focus = useRef<HTMLSpanElement>(null);
  useFrame(
    useCallback((f: Frame) => {
      setText(ha.current, dmsCircleAt(f, f.o.hrDms));
      setText(va.current, dmsCircleAt(f, f.o.vrDms));
      setText(dh.current, dmsSignedAt(f, f.o.sightDhDms));
      setText(dv.current, dmsSignedAt(f, f.o.sightDvDms));
      const good = flag(f, f.o.sightOk);
      setText(ok.current, good ? t.sightOk : t.sightAdjust);
      ok.current?.classList.toggle('is-ok', good);
      const rOk = flag(f, f.o.reticleOk);
      setText(reticle.current, rOk ? t.sharp : t.blurry);
      reticle.current?.classList.toggle('is-ok', rOk);
      const fOk = flag(f, f.o.focusOk);
      setText(focus.current, fOk ? t.inFocus : t.outOfFocus);
      focus.current?.classList.toggle('is-ok', fOk);
    }, [t]),
  );
  const o = ui.observation;
  const target = ui.currentStep === 9 ? t.targetBs : t.targetFs;
  return (
    <>
      <Section title={t.look}>
        <div className="seg" role="radiogroup" aria-label={t.look}>
          <button
            type="button"
            role="radio"
            aria-checked={o.peepSight}
            className={o.peepSight ? 'is-on' : ''}
            onClick={() => bridge.send('setPeepSight', 1)}
          >
            {t.peepSight}
          </button>
          <button
            type="button"
            role="radio"
            aria-checked={!o.peepSight}
            className={o.peepSight ? '' : 'is-on'}
            onClick={() => bridge.send('setPeepSight', 0)}
          >
            {t.telescope}
          </button>
        </div>
        <p className="note">{o.peepSight ? t.peepNote : t.teleNote}</p>
      </Section>
      <Section title={t.eyepieceTitle}>
        <div className="readout">
          <div>
            {t.crosshair} <span className="sight" ref={reticle} />
          </div>
        </div>
        <div className="pair-row">
          <HoldButton intent={{ kind: 'eyepiece', dir: -1 }} title={t.keyTitle('V')}>
            {t.turnLeft}
          </HoldButton>
          <HoldButton intent={{ kind: 'eyepiece', dir: 1 }} title={t.keyTitle('B')}>
            {t.turnRight}
          </HoldButton>
        </div>
      </Section>
      <Section title={t.focusTitle}>
        <div className="readout">
          <div>
            {t.focus} <span className="sight" ref={focus} />
          </div>
        </div>
        <div className="pair-row">
          <HoldButton intent={{ kind: 'focus', dir: 1 }} title={t.keyTitle('F')}>
            {t.near}
          </HoldButton>
          <HoldButton intent={{ kind: 'focus', dir: -1 }} title={t.keyTitle('G')}>
            {t.far}
          </HoldButton>
        </div>
        <p className="note">{t.focusNote}</p>
      </Section>
      <Section title={t.sightTitle(target)}>
        <div className="readout">
          <div>
            {t.ha} <span ref={ha} />
          </div>
          <div>
            {t.va} <span ref={va} />
          </div>
          <div>
            {t.offsetH} <span ref={dh} /> {t.offsetV} <span ref={dv} />
          </div>
          <div>
            <span className="sight" ref={ok} />
          </div>
        </div>
        <SpeedSelector labels={t.teleSpeedLabels} />
        <DirPad make={(dx, dy) => ({ kind: 'telescope', dh: dx, dv: dy })} labels={t.teleDir} />
        <p className="note">{t.teleNote2}</p>
        <button type="button" className="primary" onClick={() => bridge.send('observe')}>
          {t.measure}
        </button>
      </Section>
      {o.bs && (
        <Section title={t.bsTitle}>
          <dl className="result">
            <dt>{t.bearing}</dt>
            <dd>{o.bs.azimuthText}</dd>
            <dt>{t.hdObserved}</dt>
            <dd>{o.bs.horizontalText} m</dd>
            <dt>{t.hdCoord}</dt>
            <dd>{o.bs.coordDistanceText} m</dd>
            <dt>{t.distanceDiff}</dt>
            <dd>{o.bs.distanceDiffText}</dd>
          </dl>
        </Section>
      )}
      {o.fs && (
        <Section title={t.fsTitle(o.design.name)}>
          <table className="points">
            <thead>
              <tr>
                <th />
                <th>X (m)</th>
                <th>Y (m)</th>
                <th>H (m)</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>{t.observed}</td>
                <td>{o.fs.xText}</td>
                <td>{o.fs.yText}</td>
                <td>{o.fs.hText}</td>
              </tr>
              <tr>
                <td>{t.design}</td>
                <td>{o.design.x}</td>
                <td>{o.design.y}</td>
                <td>{o.design.h}</td>
              </tr>
              <tr className="diff">
                <td>{t.diff}</td>
                <td>{o.fs.dxText}</td>
                <td>{o.fs.dyText}</td>
                <td>{o.fs.dhText}</td>
              </tr>
            </tbody>
          </table>
          <p className="note">{t.fsNote(o.fs.azimuthText, o.fs.horizontalText)}</p>
          {/* 測り直したら説明も聞き直せるよう、結果が変わったら作り直す */}
          <AiExplain key={`${o.fs.xText}/${o.fs.yText}/${o.fs.hText}`} />
        </Section>
      )}
    </>
  );
}
