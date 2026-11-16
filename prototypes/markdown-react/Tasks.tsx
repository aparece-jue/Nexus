import { useEffect, useState } from 'react';
import { invoke, isTauri } from '@tauri-apps/api/core';

type Project = { id: string; name: string; row_version: number };
type Task = { id: string; title: string; project_id: string | null; status: string; row_version: number;
  source_note_id: string | null; source_revision: number | null; allowed_targets: string[]; terminal: boolean;
  parent_id: string | null; tags: string[]; archived: boolean };
const splitTags = (value: string) => value.split(/[,，]/).map(tag => tag.trim()).filter(Boolean);
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
  const [parentId, setParentId] = useState('');
  const [tags, setTags] = useState('');
  const [filter, setFilter] = useState({tag:'',text:'',archived:false});
  const [allTasks, setAllTasks] = useState<Task[]>([]);
  const [editing, setEditing] = useState<Task | null>(null);
  const [linkSource, setLinkSource] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  async function refresh() {
    const [nextProjects,nextTasks,candidates] = await Promise.all([invoke<Project[]>('list_projects'),invoke<Task[]>('list_tasks',{query:filter}),invoke<Task[]>('list_tasks')]);
    setProjects(nextProjects); setTasks(nextTasks); setAllTasks(candidates);
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
        parent_id: parentId || null, tags:splitTags(tags),
        source_note_id: linkSource ? source?.id ?? null : null, source_revision: linkSource ? source?.revision ?? null : null } });
      setTitle('');
    }); }}>
      <input disabled={busy} aria-label="任务标题" value={title} onChange={e => setTitle(e.target.value)} />
      <select disabled={busy} aria-label="所属项目" value={projectId} onChange={e => setProjectId(e.target.value)}>
        <option value="">无项目</option>{projects.map(p => <option key={p.id} value={p.id}>{p.name}</option>)}
      </select>
      <select disabled={busy} aria-label="父任务" value={parentId} onChange={e => {
        setParentId(e.target.value); const parent = allTasks.find(t => t.id===e.target.value); if (parent) setProjectId(parent.project_id ?? '');
      }}><option value="">顶层任务</option>{allTasks.filter(t => !t.terminal).map(t => <option key={t.id} value={t.id}>{t.title}</option>)}</select>
      <input disabled={busy} aria-label="任务标签" placeholder="标签，用逗号分隔" value={tags} onChange={e => setTags(e.target.value)} />
      <label><input type="checkbox" checked={linkSource} disabled={busy || !source} onChange={e => setLinkSource(e.target.checked)} />关联当前已保存便签</label>
      <button disabled={!isTauri() || busy || !title.trim() || (linkSource && !source)}>收集任务</button>
    </form>
    <form onSubmit={e => { e.preventDefault(); void run(async () => {}); }}>
      <input disabled={busy} aria-label="标签搜索" placeholder="精确标签，如 工程" value={filter.tag} onChange={e => setFilter({...filter,tag:e.target.value})} />
      <input disabled={busy} aria-label="任务标题搜索" value={filter.text} onChange={e => setFilter({...filter,text:e.target.value})} />
      <label><input disabled={busy} type="checkbox" checked={filter.archived} onChange={e => setFilter({...filter,archived:e.target.checked})} />已归档任务</label>
      <button disabled={!isTauri() || busy}>搜索任务</button>
    </form>
    {editing && <TaskEditor key={editing.id} task={editing} projects={projects} candidates={allTasks} busy={busy}
      onCancel={() => setEditing(null)} onSave={(fields,archived) => void run(async () => {
        await invoke('update_task',{id:editing.id,expectedRowVersion:editing.row_version,fields,archived}); setEditing(null);
      })} />}
    <button disabled={!isTauri() || busy} onClick={() => void run(async () => {})}>刷新任务</button>
    {error && <p role="alert">{error}</p>}
    <ul>{tasks.map(task => <li key={task.id}>
      <strong>{task.title}</strong> · {labels[task.status] ?? task.status} · {projects.find(p => p.id === task.project_id)?.name ?? '无项目'}
      {task.parent_id && <span> · 子任务（父任务：{allTasks.find(t=>t.id===task.parent_id)?.title ?? task.parent_id}）</span>}
      {(task.tags ?? []).map(tag => <span key={tag}> #{tag}</span>)}
      <button disabled={busy || editing !== null} onClick={() => setEditing(task)}>编辑 {task.title}</button>
      {task.source_note_id && task.source_revision !== null && <button disabled={busy} onClick={() => onOpenSource(task.source_note_id!,task.source_revision!)}>查看来源版本 {task.source_revision}</button>}
      {!task.archived && task.allowed_targets.map(target => <button key={target} disabled={busy} onClick={() => void run(() => invoke('transition_task', {
        id:task.id, expectedRowVersion:task.row_version, target, reopen:task.terminal,
      }))}>{task.terminal ? '重新打开' : labels[target] ?? target}</button>)}
    </li>)}</ul>
  </section>;
}

function TaskEditor({ task, projects, candidates, busy, onSave, onCancel }: {
  task: Task; projects: Project[]; candidates: Task[]; busy:boolean;
  onSave:(fields:{title:string;project_id:string|null;parent_id:string|null;tags:string[]},archived:boolean)=>void; onCancel:()=>void;
}) {
  const [title,setTitle] = useState(task.title);
  const [project,setProject] = useState(task.project_id ?? '');
  const [parent,setParent] = useState(task.parent_id ?? '');
  const [tags,setTags] = useState((task.tags ?? []).join(', '));
  const [archived,setArchived] = useState(task.archived ?? false);
  return <form aria-label="编辑任务" onSubmit={e=>{e.preventDefault();onSave({title,project_id:project||null,parent_id:parent||null,tags:splitTags(tags)},archived);}}>
    <fieldset disabled={busy}><legend>编辑任务与标签</legend>
      <input aria-label="编辑任务标题" value={title} onChange={e=>setTitle(e.target.value)} />
      <select aria-label="编辑任务项目" value={project} onChange={e=>setProject(e.target.value)}><option value="">无项目</option>{projects.map(p=><option key={p.id} value={p.id}>{p.name}</option>)}</select>
      <select aria-label="编辑父任务" value={parent} onChange={e=>setParent(e.target.value)}><option value="">顶层任务</option>
        {parent && !candidates.some(t=>t.id===parent) && <option value={parent}>{parent}</option>}
        {candidates.filter(t=>t.id!==task.id).map(t=><option key={t.id} value={t.id}>{t.title}</option>)}</select>
      <input aria-label="编辑任务标签" value={tags} onChange={e=>setTags(e.target.value)} />
      <label><input type="checkbox" checked={archived} onChange={e=>setArchived(e.target.checked)} />归档此任务</label>
      <button disabled={!title.trim()}>保存任务</button><button type="button" onClick={onCancel}>取消编辑</button>
    </fieldset>
  </form>;
}
