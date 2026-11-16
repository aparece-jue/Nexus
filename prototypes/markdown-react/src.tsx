import { lazy, Suspense, useEffect, useMemo, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { isTauri, invoke } from '@tauri-apps/api/core';
import { createEditor } from '../markdown-editor/editor.mjs';
import './style.css';

const SourceEditor = lazy(() => import('./SourceEditor').then(module => ({ default: module.SourceEditor })));
const MarkdownPreview = lazy(() => import('./MarkdownPreview').then(module => ({ default: module.MarkdownPreview })));

const sample = '# Nexus 编辑组件验证\n\n- [ ] 中文输入与撤销\n\n| 器件 | 状态 |\n| --- | --- |\n| AD5422 | 待评估 |\n\n```rust\nlet revision = 1;\n```';
type Note = { revision: number; text: string };
type NoteSummary = { id: string; revision: number; title: string };

export function App() {
  const editor = useMemo(() => createEditor({
    read: async (id: string, revision: number) => isTauri()
      ? (await invoke<Note>('read_note', { id, revision })).text : sample,
    save: async (id: string, revision: number, text: string) => invoke<number>('save_note', {
      request: { id, expected_revision: revision, text },
    }),
  }), []);
  const [state, setState] = useState(editor.snapshot());
  const [initial, setInitial] = useState<string | null>(null);
  const [preview, setPreview] = useState(true);
  const [error, setError] = useState('');
  const [notes, setNotes] = useState<NoteSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [query, setQuery] = useState('');
  async function refreshNotes() {
    if (isTauri()) setNotes(await invoke<NoteSummary[]>('list_notes'));
  }
  async function load(id = 'desktop-note') {
    setLoading(true);
    try {
      const note = isTauri() ? await invoke<Note>('read_note', { id }) : { revision: 0, text: sample };
      await editor.load(id, note.revision);
      setState(editor.snapshot()); setInitial(editor.snapshot().text); setError('');
      await refreshNotes();
    } catch (failure) { setError(String(failure)); }
    finally { setLoading(false); }
  }
  useEffect(() => { void load(); }, []);
  async function save() {
    if (!isTauri()) return;
    setError('');
    const pending = editor.save(); setState(editor.snapshot());
    await pending; setState(editor.snapshot());
    try { await refreshNotes(); } catch (failure) { setError(`列表刷新失败：${String(failure)}`); }
  }
  async function createNote() {
    if (!isTauri() || loading || state.dirty || state.saving) return;
    setLoading(true);
    const id = crypto.randomUUID();
    try {
      await invoke<number>('save_note', { request: { id, expected_revision: 0, text: '# 新便签\n\n' } });
      await load(id);
    } catch (failure) { setError(String(failure)); }
    finally { setLoading(false); }
  }
  return <main>
    <h1>Nexus · 便签工作区</h1>
    <p>{isTauri() ? '独立验证工作区：退出后保留，未加密。请勿存入真实工作资料。' : '浏览器组件模式：不保存资料。'}</p>
    <button disabled={!isTauri() || loading || state.dirty || state.saving} onClick={() => void createNote()}>新建便签</button>{' '}
    <button aria-pressed={preview} onClick={() => setPreview(!preview)}>渲染预览</button>{' '}
    <button disabled={!isTauri() || loading || !state.dirty || state.saving} onClick={() => void save()}>保存新版本</button>{' '}
    <button disabled={loading || state.dirty || state.saving} onClick={() => void load(state.id ?? 'desktop-note')}>重新读取</button>
    <p>编辑后先保存，再切换便签。标题取正文首个非空行。</p>
    <nav aria-label="便签列表">
      <input aria-label="按标题查找便签" placeholder="按标题查找便签" value={query} onChange={event => setQuery(event.target.value)} />
      {notes.filter(note => note.title.toLocaleLowerCase().includes(query.toLocaleLowerCase())).map(note => <button
        key={note.id} aria-current={state.id === note.id ? 'page' : undefined}
        disabled={loading || state.dirty || state.saving} onClick={() => void load(note.id)}
      >{note.title || '未命名便签'}</button>)}
    </nav>
    <p role="status">{error || state.error || (state.saving ? '保存中' : `版本 ${state.revision} · ${state.dirty ? '有未保存修改' : '已读取'}`)}</p>
    {loading ? <p role="status">正在读取便签…</p> : <Suspense fallback={<p role="status">正在加载编辑组件…</p>}><div className="panels">
      <section aria-label="编辑区">{initial !== null && <SourceEditor
        initial={initial} onChange={value => { editor.edit(value); setState(editor.snapshot()); }}
      />}</section>
      {preview && <section aria-label="Markdown 预览"><MarkdownPreview text={state.text} /></section>}
    </div></Suspense>}
  </main>;
}
createRoot(document.getElementById('root')!).render(<App />);
