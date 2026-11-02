// 第2部：観測（電子平板・視準・測量）の操作盤（SPEC 8章）
// 電子平板の画面は特定の製品を真似ない、汎用のデザイン。

import { useCallback, useRef, useState } from 'react';
import type { Frame } from '../bridge';
import { dmsCircleAt, fixed, flag } from '../bridge/format';
import { DirPad, Section, SpeedSelector } from './controls';
import { setText, useFrame, useSim, useUi } from './hooks';

export function ObservationPanel() {
  const ui = useUi();
  if (ui.currentStep === 8) return <PlaneTable />;
  return <Telescope />;
}

function PlaneTable() {
  const ui = useUi();
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
    <div className="tablet">
      <div className="tablet-title">電子平板</div>
      <label className="field">
        <span>機械点</span>
        <select value={station} onChange={(e) => setStation(Number(e.target.value))}>
          {o.points.map((p, i) => (
            <option key={p.name} value={i}>
              {p.name}
            </option>
          ))}
        </select>
      </label>
      <label className="field">
        <span>BS点</span>
        <select value={backsight} onChange={(e) => setBacksight(Number(e.target.value))}>
          <option value={-1}>選んでください</option>
          {o.points.map((p, i) => (
            <option key={p.name} value={i}>
              {p.name}
            </option>
          ))}
        </select>
      </label>
      <label className="field">
        <span>器械高 (m)</span>
        <input inputMode="decimal" value={height} placeholder="例 1.500" onChange={(e) => setHeight(e.target.value)} />
        <button type="button" onClick={() => setHeight(fixed(measured.current, 3))}>
          巻尺で測る
        </button>
      </label>
      <table className="points">
        <thead>
          <tr>
            <th>点名</th>
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
      <p className="note">据えた測点を機械点に、ターゲットが立っている点をBS点に選びます（3Dのターゲットを確認）。</p>
      <button
        type="button"
        className="primary"
        disabled={!canSet}
        onClick={() => bridge.send('setStation', station, backsight, heightValue)}
      >
        セット
      </button>
    </div>
  );
}

function Telescope() {
  const ui = useUi();
  const { bridge } = useSim();
  const ha = useRef<HTMLSpanElement>(null);
  const va = useRef<HTMLSpanElement>(null);
  const dh = useRef<HTMLSpanElement>(null);
  const dv = useRef<HTMLSpanElement>(null);
  const ok = useRef<HTMLSpanElement>(null);
  useFrame(
    useCallback((f: Frame) => {
      setText(ha.current, dmsCircleAt(f, f.o.hrDms));
      setText(va.current, dmsCircleAt(f, f.o.vrDms));
      setText(dh.current, `${fixed(f.data[f.o.sightDhSec], 0)}″`);
      setText(dv.current, `${fixed(f.data[f.o.sightDvSec], 0)}″`);
      const good = flag(f, f.o.sightOk);
      setText(ok.current, good ? '視準OK' : '視準を合わせる');
      ok.current?.classList.toggle('is-ok', good);
    }, []),
  );
  const o = ui.observation;
  const target = ui.currentStep === 9 ? 'BS（ターゲット板）' : 'FS（プリズム）';
  return (
    <>
      <Section title={`望遠鏡：${target}を視準`}>
        <div className="readout">
          <div>
            水平角 HA <span ref={ha} />
          </div>
          <div>
            鉛直角 VA <span ref={va} />
          </div>
          <div>
            十字線とのずれ 水平 <span ref={dh} /> ／ 鉛直 <span ref={dv} />
          </div>
          <div>
            <span className="sight" ref={ok} />
          </div>
        </div>
        <SpeedSelector labels={['粗動', '中', '微動']} />
        <DirPad
          make={(dx, dy) => ({ kind: 'telescope', dh: dx, dv: dy })}
          labels={{ up: '上へ', down: '下へ', left: '左へ', right: '右へ' }}
        />
        <p className="note">右上の視野で、十字線をターゲットの中心に合わせます。Enterキーでも測量できます。</p>
        <button type="button" className="primary" onClick={() => bridge.send('observe')}>
          測量
        </button>
      </Section>
      {o.bs && (
        <Section title="BSの観測（方向角の基準）">
          <dl className="result">
            <dt>方向角</dt>
            <dd>{o.bs.azimuthText}</dd>
            <dt>水平距離（観測）</dt>
            <dd>{o.bs.horizontalText} m</dd>
            <dt>水平距離（座標から）</dt>
            <dd>{o.bs.coordDistanceText} m</dd>
            <dt>較差</dt>
            <dd>{o.bs.distanceDiffText}</dd>
          </dl>
        </Section>
      )}
      {o.fs && (
        <Section title={`FSの観測結果（設計点 ${o.design.name}）`}>
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
                <td>観測</td>
                <td>{o.fs.xText}</td>
                <td>{o.fs.yText}</td>
                <td>{o.fs.hText}</td>
              </tr>
              <tr>
                <td>設計</td>
                <td>{o.design.x}</td>
                <td>{o.design.y}</td>
                <td>{o.design.h}</td>
              </tr>
              <tr className="diff">
                <td>差</td>
                <td>{o.fs.dxText}</td>
                <td>{o.fs.dyText}</td>
                <td>{o.fs.dhText}</td>
              </tr>
            </tbody>
          </table>
          <p className="note">
            方向角 {o.fs.azimuthText}・水平距離 {o.fs.horizontalText} m
          </p>
        </Section>
      )}
    </>
  );
}
