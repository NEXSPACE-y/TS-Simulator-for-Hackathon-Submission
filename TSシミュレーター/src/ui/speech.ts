// 読み上げ（ブラウザの音声合成）。使えない環境では何もしない（文字の表示だけで使える）。
// 声は端末に入っているもの（料金なし）。画面の言語に合わせて、日本語か英語の声で読む。

import type { Lang } from './i18n';

function voiceLang(lang: Lang): string {
  return lang === 'en' ? 'en-US' : 'ja-JP';
}

function synth(): SpeechSynthesis | null {
  try {
    return typeof window !== 'undefined' && 'speechSynthesis' in window ? window.speechSynthesis : null;
  } catch {
    return null;
  }
}

/** 読み上げる。`queue` が false なら、読んでいる途中のものを止めてから読む */
export function speak(text: string, lang: Lang, queue = false): void {
  const s = synth();
  if (!s || text.trim() === '') return;
  try {
    if (!queue) s.cancel();
    const u = new SpeechSynthesisUtterance(text);
    u.lang = voiceLang(lang);
    s.speak(u);
  } catch {
    // 読み上げが無くても練習はできる
  }
}

/** 読み上げを止める */
export function stopSpeech(): void {
  try {
    synth()?.cancel();
  } catch {
    // 何もしない
  }
}

/**
 * iPhone などでは、ボタンを押した流れの中で一度読み上げておかないと、あとから読み上げられない。
 * 「はじめる」を押したときに、空の読み上げを1回しておく。
 */
export function unlockSpeech(lang: Lang): void {
  const s = synth();
  if (!s) return;
  try {
    const u = new SpeechSynthesisUtterance('');
    u.lang = voiceLang(lang);
    s.speak(u);
  } catch {
    // 何もしない
  }
}
