// 教科書モードのナビ（音声つき）。いつ・何を案内するかはコアが決める（手順に入った・目標を満たした・
// 次の操作が変わった・手順外の操作）。手順に入ったときと目標を満たしたときはAIに言い方を整えてもらい、
// それ以外と、AIが使えないときは、コアの文をそのまま読む。3Dビューの左上に字幕として出す。

import { useEffect, useRef, useState } from 'react';
import type { AiFact } from '../bridge';
import { askSenpai } from '../ai/client';
import { useSim, useUi } from './hooks';
import { useT, type Lang } from './i18n';
import { speak, stopSpeech } from './speech';

const VOICE_KEY = 'ts-sim-navi-voice';
/**
 * ナビの言い方を待つ時間の上限 [ms]（これを過ぎたらコアの文を読む）。
 * 英語はAIの返事が日本語の倍ほどかかる（3〜6秒）ので、長めに待つ（仮）
 */
const NAVI_AI_TIMEOUT_MS: Record<Lang, number> = { ja: 6_000, en: 10_000 };

function loadVoice(): boolean {
  try {
    return window.localStorage.getItem(VOICE_KEY) !== 'off';
  } catch {
    return true;
  }
}

function saveVoice(on: boolean): void {
  try {
    window.localStorage.setItem(VOICE_KEY, on ? 'on' : 'off');
  } catch {
    // 保存できなくても動作には関係ない
  }
}

interface Cue {
  id: number;
  lang: Lang;
  speech: string;
  ai: boolean;
  facts: AiFact[] | null;
}

/** AIに言い方を整えてもらう（待ちすぎたら・使えなければ、コアの文） */
async function resolveText(cue: Cue): Promise<string> {
  if (!cue.ai || !cue.facts) return cue.speech;
  const ctl = new AbortController();
  const t = window.setTimeout(() => ctl.abort(), NAVI_AI_TIMEOUT_MS[cue.lang]);
  try {
    const r = await askSenpai('navi', cue.facts, undefined, ctl.signal, cue.lang);
    return r.ok ? r.text : cue.speech;
  } finally {
    window.clearTimeout(t);
  }
}

export function NaviCaption() {
  const ui = useUi();
  const t = useT();
  const { bridge } = useSim();
  const [text, setText] = useState('');
  const [voice, setVoice] = useState(loadVoice);
  const voiceRef = useRef(voice);
  voiceRef.current = voice;
  // ナビは出た順に読む（AIを待つ間に次のナビが来ても、順番を入れ替えない）
  const chain = useRef<Promise<void>>(Promise.resolve());
  const alive = useRef(true);
  // すでに扱ったナビ（同じものを2回読まない）
  const handled = useRef(0);
  const id = ui.navi?.id ?? 0;
  const active = ui.mode === 'textbook' && ui.phase === 'setup';

  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
      stopSpeech();
    };
  }, []);

  useEffect(() => {
    if (!active || id === 0 || !ui.navi || handled.current === id) return;
    handled.current = id;
    // 事実は、きっかけが出た時点のものを、すぐに受け取っておく
    const cue: Cue = {
      id,
      lang: ui.lang,
      speech: ui.navi.speech,
      ai: ui.navi.ai,
      facts: ui.navi.ai ? bridge.aiFacts('navi') : null,
    };
    chain.current = chain.current.then(async () => {
      const said = await resolveText(cue);
      if (!alive.current) return;
      setText(said);
      if (voiceRef.current) speak(said, cue.lang, true);
    });
    // id が変わったときだけ動かす
  }, [id]);

  useEffect(() => {
    if (!active) stopSpeech();
  }, [active]);

  if (!active || !text) return null;
  return (
    <div className="navi-caption" role="status" aria-live="polite">
      <div className="navi-head">
        <span className="senpai-label">{t.navi}</span>
        <span className="navi-actions">
          <button type="button" className="ghost" onClick={() => speak(text, ui.lang)} disabled={!voice}>
            {t.replay}
          </button>
          <button
            type="button"
            className={`ghost ${voice ? 'is-on' : ''}`}
            aria-pressed={voice}
            onClick={() => {
              const on = !voice;
              setVoice(on);
              saveVoice(on);
              if (!on) stopSpeech();
            }}
          >
            {t.voice(voice)}
          </button>
        </span>
      </div>
      <p className="senpai-text">{text}</p>
    </div>
  );
}
