import { lazy, Suspense, useEffect, useMemo, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { isTauri, invoke } from '@tauri-apps/api/core';
import { createEditor } from '../markdown-editor/editor.mjs';
import './style.css';
import { Tasks } from './Tasks';

const SourceEditor = lazy(() => import('./SourceEditor').then(module => ({ default: module.SourceEditor })));
const MarkdownPreview = lazy(() => import('./MarkdownPreview').then(module => ({ default: module.MarkdownPreview })));

const sample = '# Nexus 编辑组件验证\n\n- [ ] 中文输入与撤销\n\n| 器件 | 状态 |\n| --- | --- |\n| AD5422 | 待评估 |\n\n```rust\nlet revision = 1;\n```';
type Note = { revision: number; text: string; archived?: boolean };
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
  const [showArchived, setShowArchived] = useState(false);
  const [selectedArchived, setSelectedArchived] = useState(false);
  const [sourcePreview, setSourcePreview] = useState<{ id: string; revision: number; text: string } | null>(null);
  async function openSource(id: string, revision: number) {
    try {
      const note = await invoke<Note>('read_note', { id, revision });
      setSourcePreview({ id, revision, text: note.text });
    } catch (failure) { setError(String(failure)); }
  }
  async function refreshNotes(archived = showArchived) {
    if (isTauri()) setNotes(await invoke<NoteSummary[]>('list_notes', { archived }));
  }
  async function load(id = 'desktop-note', archived = false) {
    setLoading(true);
    try {
      const note = isTauri() ? await invoke<Note>('read_note', { id }) : { revision: 0, text: sample };
      await editor.load(id, note.revision);
      setState(editor.snapshot()); setInitial(editor.snapshot().text); setError('');
      setSelectedArchived(note.archived ?? archived);
      await refreshNotes();
    } catch (failure) { setError(String(failure)); }
    finally { setLoading(false); }
  }
  async function changeArchive(note: NoteSummary, archived: boolean) {
    if (loading || state.dirty || state.saving) return;
    setLoading(true); setError('');
    try {
      await invoke<number>('set_note_archived', { id: note.id, expectedRevision: note.revision, archived });
      if (state.id === note.id) await load(note.id, archived);
      await refreshNotes();
    } catch (failure) { setError(String(failure)); }
    finally { setLoading(false); }
  }
  async function toggleArchiveList() {
    setLoading(true); setError('');
    try { await refreshNotes(!showArchived); setShowArchived(!showArchived); }
    catch (failure) { setError(String(failure)); }
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
    <Tasks source={!state.dirty && !state.saving && !loading && !selectedArchived && state.id ? { id: state.id, revision: state.revision } : null}
      onOpenSource={(id, revision) => { void openSource(id, revision); }} />
    {sourcePreview && <section role="dialog" aria-label="任务来源版本">
      <h2>来源版本 {sourcePreview.revision}</h2><pre>{sourcePreview.text}</pre>
      <button onClick={() => setSourcePreview(null)}>关闭来源</button>
    </section>}
    <p>{isTauri() ? '独立验证工作区：退出后保留，未加密。请勿存入真实工作资料。' : '浏览器组件模式：不保存资料。'}</p>
    <button disabled={!isTauri() || loading || state.dirty || state.saving} onClick={() => void createNote()}>新建便签</button>{' '}
    <button aria-pressed={preview} onClick={() => setPreview(!preview)}>渲染预览</button>{' '}
    <button disabled={!isTauri() || selectedArchived || loading || !state.dirty || state.saving} onClick={() => void save()}>保存新版本</button>{' '}
    <button disabled={loading || state.dirty || state.saving} onClick={() => void load(state.id ?? 'desktop-note', selectedArchived)}>重新读取</button>{' '}
    <button disabled={loading || state.dirty || state.saving} onClick={() => void toggleArchiveList()}>{showArchived ? '返回便签' : '查看归档'}</button>
    <p>编辑后先保存，再切换便签。标题取正文首个非空行。</p>
    <nav aria-label="便签列表">
      <input aria-label="按标题查找便签" placeholder="按标题查找便签" value={query} onChange={event => setQuery(event.target.value)} />
      {notes.filter(note => note.title.toLocaleLowerCase().includes(query.toLocaleLowerCase())).map(note => <span key={note.id}>
        <button aria-current={state.id === note.id ? 'page' : undefined}
          disabled={loading || state.dirty || state.saving} onClick={() => void load(note.id, showArchived)}
        >{note.title || '未命名便签'}</button>
        <button aria-label={`${showArchived ? '恢复' : '归档'} ${note.title}`} disabled={loading || state.dirty || state.saving}
          onClick={() => void changeArchive(note, !showArchived)}>{showArchived ? '恢复' : '归档'}</button>
      </span>)}
    </nav>
    <p role="status">{error || state.error || (state.saving ? '保存中' : `版本 ${state.revision} · ${state.dirty ? '有未保存修改' : '已读取'}`)}</p>
    {loading ? <p role="status">正在读取便签…</p> : <Suspense fallback={<p role="status">正在加载编辑组件…</p>}><div className="panels">
      <section aria-label="编辑区">{selectedArchived ? <p>此便签已归档，请在归档列表恢复后编辑。</p> : initial !== null && <SourceEditor
        initial={initial} onChange={value => { editor.edit(value); setState(editor.snapshot()); }}
      />}</section>
      {preview && <section aria-label="Markdown 预览"><MarkdownPreview text={state.text} /></section>}
    </div></Suspense>}
  </main>;
}
createRoot(document.getElementById('root')!).render(<App />);
