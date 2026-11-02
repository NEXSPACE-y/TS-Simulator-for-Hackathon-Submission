# TS実習シミュレーター

トータルステーション（TS）の**据付から観測（BS・FS）まで**を、ブラウザの3Dで練習できるシミュレーターです。
Built with Claude: Japan 2026 の出展作品です。仕様は [SPEC.md](SPEC.md) にあります。

**Web版：https://nexspace-y.github.io/TS-Simulator-for-Hackathon-Submission/**

- 作品の核：求心と整準の「片方を直すと、もう片方がズレる」ループ（手順6⇄7）を、実機に近い感覚で再現すること
- 計算のコアは Rust で書き、WebAssembly（WASM）にして Web版とデスクトップ版で同じものを使います

## 使い方

1. 最初の画面で「教科書モード」か「現場モード」を選び、「はじめる」を押します。
2. 右の操作盤（ステップごとに中身が変わる）と、右上の拡大表示（求心・円形気泡・液晶）を見ながら据え付けます。
3. 求心ズレ 0.5 mm 以内、電子気泡 X・Y とも ±15″ 以内になったら「据付完了」。合格すると第2部（観測）へ進めます。

タイムは最初の操作で始まり、「据付完了」を押すと止まります。目安は 3分で合格、2分で熟練です。

### キーボード操作

| 操作 | キー |
|---|---|
| 整準ネジ A（左手前） | `A`（親指を左）／ `S`（親指を右） |
| 整準ネジ B（右手前） | `K` ／ `L` |
| 整準ネジ C（奥） | `Z` ／ `X` |
| A・B を同時に内側へ／外側へ | `S`＋`K` ／ `A`＋`L` |
| 脚を伸ばす／縮める | `1` `2` `3` ／ `Q` `W` `E` |
| 三脚・器械・望遠鏡を動かす | 矢印キー（手順によって動く物が変わる） |
| 定心桿を締める・緩める | `T` |
| 細かく動かす | `Shift` を押しながら |
| 測量（観測） | `Enter` |

## 開発

必要なもの：Rust（版は `rust-toolchain.toml` で固定）、[wasm-pack](https://crates.io/crates/wasm-pack)（`cargo install wasm-pack`）、Node.js（22以降）

```
npm install          依存関係のインストール
npm run build:wasm   WASMのビルド（wasm-pack）
npm run dev          開発サーバー（先にWASMをビルドする）
npm run build        Web版のビルド（WASMのビルドを含む）
cargo test           コアのテスト
npm test             Vitest（WASMとのつなぎ目。先にWASMをビルドする）
npm run tauri dev    デスクトップ版の開発起動
```

### 構成

```
crates/
  core/     状態・物理・判定・観測の計算・タイマー（純粋なRust）
  wasm/     coreをTypeScriptに公開する薄い層（wasm-bindgen）
src/
  bridge/   WASMの読み込みと呼び出し（TypeScriptからコアに触る窓口）
  render/   Three.jsのシーン、モデル、カメラ
  ui/       画面のパネル（React）
  main.tsx  起動処理
tests/      Vitest（WASMとのつなぎ目の確認）
src-tauri/  デスクトップ版（Tauri 2）
```

- 物理・判定・座標計算はすべて `crates/core` にあります。許容値などの数値は `crates/core/src/config.rs` の1か所にまとめています。
- 1フレームにつき「操作を渡す」「結果をまとめて受け取る」の1回だけ WASM を呼びます。

### 配布

- Web版：GitHub Pages（https://nexspace-y.github.io/TS-Simulator-for-Hackathon-Submission/ ）。`main` に push すると、`.github/workflows/pages.yml` でビルドして公開する。事前にリポジトリの Settings → Pages → Source を「GitHub Actions」にしておく
- デスクトップ版：GitHub Actions（`.github/workflows/desktop.yml`）で Windows / macOS 向けにビルドし、GitHub Releases の下書きに置く

## 素材と表記

- 外部の画像・3Dモデル・フォントなどの素材は使っていません。地面・測点・ターゲットなどの質感と、アプリのアイコンは、すべてプログラムで描いています。
- 画像生成など、Claude 以外の AI を使った素材はありません。
- 既知点・設計点の座標はすべて架空のものです。実在の現場の座標ではありません。
- TS本体・電子平板の画面は、特定メーカーの製品に似せていません。
