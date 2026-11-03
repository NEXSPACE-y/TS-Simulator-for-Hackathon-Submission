// 起動処理：WASM（Rustの計算コア）を読み込んでから画面を出す

import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { ModeId, loadBridge } from './bridge';
import { App } from './ui/App';
import './ui/styles.css';

const root = document.getElementById('root');

async function start(): Promise<void> {
  if (!root) return;
  try {
    const bridge = await loadBridge(ModeId.Field, 1);
    createRoot(root).render(
      <StrictMode>
        <App bridge={bridge} />
      </StrictMode>,
    );
  } catch (e) {
    root.textContent = `読み込みに失敗しました / Failed to load: ${String(e)}`;
  }
}

void start();
