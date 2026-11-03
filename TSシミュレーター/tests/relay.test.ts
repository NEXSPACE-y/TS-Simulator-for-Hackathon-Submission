// AI先輩の中継（relay/api/ai.ts）の確認。本物の Claude API は呼ばず、fetch を差し替えて調べる。
// 確かめること：決まった形のデータだけを通すこと、呼べる出どころを絞ること、APIキーは環境変数から送ること、
// 指示の聞き取りで点名が違う文は出さないこと。
import { describe, expect, it } from 'vitest';
import { buildRequest, handle, instructTextOk, parseInput, type Deps } from '../relay/api/ai';

const PAGES = 'https://nexspace-y.github.io';
const facts = [
  { k: 'タイム', v: '2:34.5' },
  { k: '次の操作', v: '整準ネジA・Bを同時に内側へ' },
];

function deps(reply: unknown, opts: { status?: number; env?: Record<string, string> } = {}) {
  const calls: { url: string; init: RequestInit }[] = [];
  const d: Deps = {
    fetch: (async (url: string, init: RequestInit) => {
      calls.push({ url, init });
      return new Response(JSON.stringify(reply), { status: opts.status ?? 200 });
    }) as unknown as typeof fetch,
    env: opts.env ?? { ANTHROPIC_API_KEY: 'test-key' },
    now: () => 1000,
  };
  return { d, calls };
}

function post(body: unknown, origin = PAGES, ip = '1.1.1.1'): Request {
  return new Request('https://relay.example/api/ai', {
    method: 'POST',
    headers: { origin, 'content-type': 'application/json', 'x-forwarded-for': ip },
    body: JSON.stringify(body),
  });
}

const textReply = (text: string) => ({ stop_reason: 'end_turn', content: [{ type: 'text', text }] });

describe('中継が受け付ける形', () => {
  it('決まった形なら通す', () => {
    expect(parseInput({ kind: 'review', facts })).not.toBeNull();
    expect(parseInput({ kind: 'ask', facts, question: '求心ってなに？' })?.question).toBe('求心ってなに？');
  });

  it('知らない種類・余計な項目・長すぎる値・形の違う事実は通さない', () => {
    expect(parseInput({ kind: 'chat', facts })).toBeNull();
    expect(parseInput({ kind: 'review', facts, model: 'claude-opus-5-5' })).toBeNull();
    expect(parseInput({ kind: 'review', facts: [{ k: 'a', v: 'x'.repeat(241) }] })).toBeNull();
    expect(parseInput({ kind: 'review', facts: [{ k: 'a', v: 1 }] })).toBeNull();
    expect(parseInput({ kind: 'review', facts: [{ k: 'a', v: 'b', extra: 'c' }] })).toBeNull();
    expect(parseInput({ kind: 'review', facts: [] })).toBeNull();
    expect(parseInput({ kind: 'review', facts: Array.from({ length: 61 }, () => ({ k: 'a', v: 'b' })) })).toBeNull();
  });

  it('質問の文は「何でも質問」のときだけ、200字まで', () => {
    expect(parseInput({ kind: 'review', facts, question: 'x' })).toBeNull();
    expect(parseInput({ kind: 'ask', facts })).toBeNull();
    expect(parseInput({ kind: 'ask', facts, question: 'あ'.repeat(201) })).toBeNull();
  });

  it('指示の聞き取りには、機械点とBS点の点名が要る', () => {
    expect(parseInput({ kind: 'instruct', facts })).toBeNull();
    expect(
      parseInput({
        kind: 'instruct',
        facts: [
          { k: '機械点', v: 'T-1' },
          { k: 'BS点', v: 'T-2' },
        ],
      }),
    ).not.toBeNull();
  });

  it('AIへの指示文は中継の中で組み立て、事実と質問は本文に入れる', () => {
    const input = parseInput({ kind: 'ask', facts, question: 'なんで6と7を繰り返すの？' });
    if (!input) throw new Error('通るはず');
    const body = buildRequest(input) as { system: string; messages: { content: string }[]; max_tokens: number };
    expect(body.system).toContain('事実にない数値を作らない');
    expect(body.messages[0]?.content).toContain('- 次の操作：整準ネジA・Bを同時に内側へ');
    expect(body.messages[0]?.content).toContain('新人の質問：なんで6と7を繰り返すの？');
    expect(body.max_tokens).toBeGreaterThan(0);
  });
});

describe('中継の受け口', () => {
  it('Pages からの事前確認（OPTIONS）には許可を返し、ほかの出どころは断る', async () => {
    const { d } = deps(textReply('x'));
    const ok = await handle(new Request('https://relay.example/api/ai', { method: 'OPTIONS', headers: { origin: PAGES } }), d);
    expect(ok.status).toBe(204);
    expect(ok.headers.get('access-control-allow-origin')).toBe(PAGES);
    const ng = await handle(post({ kind: 'review', facts }, 'https://evil.example'), d);
    expect(ng.status).toBe(403);
    expect(ng.headers.get('access-control-allow-origin')).toBeNull();
  });

  it('APIキーが登録されていなければ、使えないと返す（Claude API は呼ばない）', async () => {
    const { d, calls } = deps(textReply('x'), { env: {} });
    const res = await handle(post({ kind: 'review', facts }, PAGES, '2.2.2.2'), d);
    expect(res.status).toBe(503);
    expect(calls.length).toBe(0);
  });

  it('APIキーは環境変数から付けて送り、返事の文章だけを返す', async () => {
    const { d, calls } = deps(textReply('いい据付だったね。'));
    const res = await handle(post({ kind: 'review', facts }, PAGES, '3.3.3.3'), d);
    const data = (await res.json()) as { ok: boolean; text: string };
    expect(data).toEqual({ ok: true, text: 'いい据付だったね。' });
    const headers = calls[0]?.init.headers as Record<string, string>;
    expect(headers['x-api-key']).toBe('test-key');
    expect(headers['anthropic-version']).toBe('2026-10-21');
    const sent = JSON.parse(String(calls[0]?.init.body)) as { model: string };
    expect(sent.model).toBe('claude-opus-5-5');
  });

  it('断られたとき・相手が落ちているときは、詳しい中身を出さずに返す', async () => {
    const refused = deps({ stop_reason: 'refusal', content: [] });
    const r1 = (await (await handle(post({ kind: 'review', facts }, PAGES, '4.4.4.4'), refused.d)).json()) as {
      error: string;
    };
    expect(r1.error).toBe('refused');
    const down = deps({ error: { message: 'secret detail' } }, { status: 500 });
    const res = await handle(post({ kind: 'review', facts }, PAGES, '5.5.5.5'), down.d);
    expect(res.status).toBe(502);
    expect(await res.text()).not.toContain('secret');
  });

  it('同じ出どころから1分に何度も来たら、しばらく断る', async () => {
    const { d } = deps(textReply('x'));
    let last = 0;
    for (let i = 0; i < 21; i++) last = (await handle(post({ kind: 'review', facts }, PAGES, '6.6.6.6'), d)).status;
    expect(last).toBe(429);
  });

  it('指示の聞き取りで、点名が違う・足りない文は出さず、決まった言い方にする', async () => {
    const f = [
      { k: '機械点', v: 'T-1' },
      { k: 'BS点', v: 'T-2' },
    ];
    expect(instructTextOk('T-1に据えたから、BSはT-2な。', f)).toBe(true);
    expect(instructTextOk('T-1に据えたから、BSはT-3な。', f)).toBe(false);
    const { d } = deps(textReply('BSはT-3な。'));
    const data = (await (await handle(post({ kind: 'instruct', facts: f }, PAGES, '7.7.7.7'), d)).json()) as {
      text: string;
    };
    expect(data.text).toContain('T-1');
    expect(data.text).toContain('T-2');
    expect(data.text).not.toContain('T-3');
  });
});
