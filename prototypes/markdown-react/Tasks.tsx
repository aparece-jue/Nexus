import { useEffect, useState } from 'react';
import { invoke, isTauri } from '@tauri-apps/api/core';

type Project = { id: string; name: string; row_version: number };
type Task = { id: string; title: string; project_id: string | null; status: string; row_version: number;
  source_note_id: string | null; source_revision: number | null; allowed_targets: string[]; terminal: boolean };
const labels: Record<string,string> = { Inbox:'收集箱', Clarifying:'澄清中', Ready:'待办', Doing:'进行中', Blocked:'阻塞', Waiting:'等待', Review:'待检查', Done:'已完成', Cancelled:'已取消' };

export function Tasks({ source, onOpenSource }: {
  source: { id: string; revision: number } | null;
  onOpenSource: (id: string, revision: number) => void;
}) {
  const [projects, setProjects] = useState<Project[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [title, setTitle] = useState('');
  const [projectName, setProjectName] = useState('');
  const [projectId, setProjectId] = useState('');
  const [linkSource, setLinkSource] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  async function refresh() {
    const [nextProjects,nextTasks] = await Promise.all([invoke<Project[]>('list_projects'),invoke<Task[]>('list_tasks')]);
    setProjects(nextProjects); setTasks(nextTasks);
  }
  async function run(action: () => Promise<unknown>) {
    setBusy(true); setError('');
    try { await action(); await refresh(); }
    catch (failure) { setError(String(failure)); }
    finally { setBusy(false); }
  }
  useEffect(() => { if (isTauri()) void run(async () => {}); }, []);
  return <section aria-label="项目与任务">
    <h2>项目与任务</h2>
    <p>先收集，再澄清为待办；状态变更由本地服务校验。</p>
    <form onSubmit={event => { event.preventDefault(); void run(async () => {
      await invoke('create_project', { id: crypto.randomUUID(), name: projectName }); setProjectName('');
    }); }}>
      <input disabled={busy} aria-label="项目名称" value={projectName} onChange={e => setProjectName(e.target.value)} />
      <button disabled={!isTauri() || busy || !projectName.trim()}>新建项目</button>
    </form>
    <form onSubmit={event => { event.preventDefault(); void run(async () => {
      await invoke('create_task', { request: { id: crypto.randomUUID(), title, project_id: projectId || null,
        source_note_id: linkSource ? source?.id ?? null : null, source_revision: linkSource ? source?.revision ?? null : null } });
      setTitle('');
    }); }}>
      <input disabled={busy} aria-label="任务标题" value={title} onChange={e => setTitle(e.target.value)} />
      <select disabled={busy} aria-label="所属项目" value={projectId} onChange={e => setProjectId(e.target.value)}>
        <option value="">无项目</option>{projects.map(p => <option key={p.id} value={p.id}>{p.name}</option>)}
      </select>
      <label><input type="checkbox" checked={linkSource} disabled={busy || !source} onChange={e => setLinkSource(e.target.checked)} />关联当前已保存便签</label>
      <button disabled={!isTauri() || busy || !title.trim() || (linkSource && !source)}>收集任务</button>
    </form>
    <button disabled={!isTauri() || busy} onClick={() => void run(async () => {})}>刷新任务</button>
    {error && <p role="alert">{error}</p>}
    <ul>{tasks.map(task => <li key={task.id}>
      <strong>{task.title}</strong> · {labels[task.status] ?? task.status} · {projects.find(p => p.id === task.project_id)?.name ?? '无项目'}
      {task.source_note_id && task.source_revision !== null && <button disabled={busy} onClick={() => onOpenSource(task.source_note_id!,task.source_revision!)}>查看来源版本 {task.source_revision}</button>}
      {task.allowed_targets.map(target => <button key={target} disabled={busy} onClick={() => void run(() => invoke('transition_task', {
        id:task.id, expectedRowVersion:task.row_version, target, reopen:task.terminal,
      }))}>{task.terminal ? '重新打开' : labels[target] ?? target}</button>)}
    </li>)}</ul>
  </section>;
}
