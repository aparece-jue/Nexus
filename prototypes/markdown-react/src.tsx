import { lazy, Suspense, useEffect, useMemo, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { isTauri, invoke } from '@tauri-apps/api/core';
import { createEditor } from '../markdown-editor/editor.mjs';
import './style.css';

const SourceEditor = lazy(() => import('./SourceEditor').then(module => ({ default: module.SourceEditor })));
const MarkdownPreview = lazy(() => import('./MarkdownPreview').then(module => ({ default: module.MarkdownPreview })));

const sample = '# Nexus 编辑组件验证\n\n- [ ] 中文输入与撤销\n\n| 器件 | 状态 |\n| --- | --- |\n| AD5422 | 待评估 |\n\n```rust\nlet revision = 1;\n```';
type Note = { revision: number; text: string };

function App() {
  const editor = useMemo(() => createEditor({
    read: async (_id: string, revision: number) => isTauri()
      ? (await invoke<Note>('read_note', { revision })).text : sample,
    save: async (_id: string, revision: number, text: string) => invoke<number>('save_note', {
      request: { expected_revision: revision, text },
    }),
  }), []);
  const [state, setState] = useState(editor.snapshot());
  const [initial, setInitial] = useState<string | null>(null);
  const [preview, setPreview] = useState(true);
  const [error, setError] = useState('');
  async function load() {
    try {
      const note = isTauri() ? await invoke<Note>('read_note') : { revision: 0, text: sample };
      await editor.load('desktop-note', note.revision);
      setState(editor.snapshot()); setInitial(editor.snapshot().text); setError('');
    } catch (failure) { setError(String(failure)); }
  }
  useEffect(() => { void load(); }, []);
  async function save() {
    if (!isTauri()) return;
    setError('');
    const pending = editor.save(); setState(editor.snapshot());
    await pending; setState(editor.snapshot());
  }
  return <main>
    <h1>Nexus Markdown 编辑验证</h1>
    <p>{isTauri() ? '桌面临时工作区：仅用于合成资料验证，退出后清理。' : '浏览器组件模式：不保存资料。'}</p>
    <button aria-pressed={preview} onClick={() => setPreview(!preview)}>渲染预览</button>{' '}
    <button disabled={!isTauri() || !state.dirty || state.saving} onClick={() => void save()}>保存新版本</button>{' '}
    <button disabled={state.dirty || state.saving} onClick={() => void load()}>重新读取</button>
    <p role="status">{error || state.error || (state.saving ? '保存中' : `版本 ${state.revision} · ${state.dirty ? '有未保存修改' : '已读取'}`)}</p>
    <Suspense fallback={<p role="status">正在加载编辑组件…</p>}><div className="panels">
      <section aria-label="编辑区">{initial !== null && <SourceEditor
        initial={initial} onChange={value => { editor.edit(value); setState(editor.snapshot()); }}
      />}</section>
      {preview && <section aria-label="Markdown 预览"><MarkdownPreview text={state.text} /></section>}
    </div></Suspense>
  </main>;
}
createRoot(document.getElementById('root')!).render(<App />);
