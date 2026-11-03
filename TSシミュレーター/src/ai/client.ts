// AI先輩（Claude）を呼ぶ窓口。呼ぶ先は中継（relay/）だけで、APIキーはブラウザ側には無い。
// 渡すのは、コア（Rust）が出した「事実」だけ。中継に届かない・止まっているときは「使えません」を返し、
// シミュレーター本体はそのまま動く。

import type { AiFact } from '../bridge';
import type { Lang } from '../ui/i18n';
import { AI_RELAY_URL, AI_TIMEOUT_MS } from './config';

/** review＝据付の講評、ask＝何でも質問、explain＝FS座標の計算の説明、instruct＝現場の指示、advice＝操作中のひとこと、
 *  navi＝教科書モードのナビ */
export type AiKind = 'review' | 'ask' | 'explain' | 'instruct' | 'advice' | 'navi';

export type AiResult = { ok: true; text: string } | { ok: false; message: string };

const UNAVAILABLE: Readonly<Record<Lang, string>> = {
  ja: 'AI先輩はいま使えません（ネットにつながっていないか、中継が止まっています）。シミュレーターはそのまま使えます。',
  en: 'AI Senpai is not available right now (you may be offline, or the relay is down). The simulator still works.',
};

const MESSAGES: Readonly<Record<Lang, Readonly<Record<string, string>>>> = {
  ja: {
    refused: 'その内容には答えられませんでした。聞き方を変えてみてください。',
    too_many: 'AI先輩への質問が続いたので、少し時間をおいてからもう一度どうぞ。',
    bad_request: 'AI先輩に送る内容の形が合いませんでした（質問は200字までです）。',
  },
  en: {
    refused: 'AI Senpai could not answer that. Try asking another way.',
    too_many: 'Many questions in a row. Please wait a moment and try again.',
    bad_request: 'The request to AI Senpai was not in the expected form (questions are up to 200 characters).',
  },
};

export async function askSenpai(
  kind: AiKind,
  facts: readonly AiFact[],
  question?: string,
  signal?: AbortSignal,
  lang: Lang = 'ja',
): Promise<AiResult> {
  const unavailable = UNAVAILABLE[lang];
  if (!AI_RELAY_URL) return { ok: false, message: unavailable };
  const ctl = new AbortController();
  const timer = window.setTimeout(() => ctl.abort(), AI_TIMEOUT_MS);
  const onAbort = () => ctl.abort();
  signal?.addEventListener('abort', onAbort);
  try {
    const res = await fetch(AI_RELAY_URL, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      // 言語は英語のときだけ送る（日本語は前からの形のまま）
      body: JSON.stringify({
        kind,
        facts,
        ...(question === undefined ? {} : { question }),
        ...(lang === 'en' ? { lang } : {}),
      }),
      signal: ctl.signal,
    });
    const data = (await res.json().catch(() => null)) as { ok?: boolean; text?: unknown; error?: unknown } | null;
    if (data?.ok && typeof data.text === 'string' && data.text !== '') return { ok: true, text: data.text };
    const code = typeof data?.error === 'string' ? data.error : '';
    return { ok: false, message: MESSAGES[lang][code] ?? unavailable };
  } catch {
    return { ok: false, message: unavailable };
  } finally {
    window.clearTimeout(timer);
    signal?.removeEventListener('abort', onAbort);
  }
}
