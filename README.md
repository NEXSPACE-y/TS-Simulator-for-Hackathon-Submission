# TS実習シミュレーター

トータルステーション（TS）の**据付から観測（BS・FS）まで**を、ブラウザの3Dで練習できるシミュレーターです。
Built with Claude: Japan 2026 の出展作品です。

**Web版：https://nexspace-y.github.io/TS-Simulator-for-Hackathon-Submission/**
（インストールは要りません。URL を開くだけで動きます）

- 作品の核：求心と整準の「片方を直すと、もう片方がズレる」ループを、実機に近い感覚で再現すること
- 計算のコアは Rust で書き、WebAssembly（WASM）にしてブラウザで動かします
- AI先輩（Claude）が、講評・質問への答え・計算の解説・現場の指示・声のナビをします。合否と数値はコアが決め、AIは言葉にするだけです
- 画面は日本語と英語に対応しています

## 中身

| 場所 | 内容 |
|---|---|
| [TSシミュレーター/README.md](TSシミュレーター/README.md) | 使い方・キーボード操作・AI先輩・開発のしかた |
| [TSシミュレーター/SPEC.md](TSシミュレーター/SPEC.md) | 仕様書 |
| [TSシミュレーター/SPEC追記案_AI機能.md](TSシミュレーター/SPEC追記案_AI機能.md) | AI先輩の仕様（オーナー確認済み） |
| [TSシミュレーター/](TSシミュレーター/) | アプリのソース一式（Rust のコア、TypeScript の表示、AI先輩の中継） |
