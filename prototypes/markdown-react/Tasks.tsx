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
  const [detailDirty,setDetailDirty] = useState(false);
  const [linkSource, setLinkSource] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [projectView, setProjectView] = useState('');
  const [inboxOnly,setInboxOnly] = useState(false);
  const [closed, setClosed] = useState<Set<string>>(new Set());
  async function refresh(query = filter) {
    const [nextProjects,nextTasks,candidates] = await Promise.all([invoke<Project[]>('list_projects'),invoke<Task[]>('list_tasks',{query}),invoke<Task[]>('list_tasks')]);
    setProjects(nextProjects); setTasks(nextTasks); setAllTasks(candidates);
  }
  async function run(action: () => Promise<unknown>, query = filter) {
    setBusy(true); setError('');
    try { await action(); await refresh(query); }
    catch (failure) { setError(String(failure)); }
    finally { setBusy(false); }
  }
  useEffect(() => { if (isTauri()) void run(async () => {}); }, []);
  function chooseTag(tag: string) {
    setProjectView(''); setInboxOnly(false);
    const query = { ...filter,tag };
    setFilter(query); void run(async () => {},query);
  }
  const visibleTasks = tasks.filter(t => (!projectView || t.project_id===projectView) && (!inboxOnly || t.status==='Inbox'));
  const visibleIds = new Set(visibleTasks.map(t=>t.id));
  const children = new Map<string,Task[]>();
  for (const task of visibleTasks) {
    const key = task.parent_id && visibleIds.has(task.parent_id) ? task.parent_id : '';
    children.set(key,[...(children.get(key) ?? []),task]);
  }
  function renderTasks(parent = '', depth = 0): React.ReactNode {
    return (children.get(parent) ?? []).map(task => <li key={task.id}>
      <div className={`task-row ${editing?.id===task.id ? 'selected' : ''}`} style={{paddingLeft:14+depth*20}}>
        {(children.get(task.id)?.length ?? 0)>0 ? <button className="disclosure" aria-label={`${closed.has(task.id)?'展开':'折叠'} ${task.title}`} aria-expanded={!closed.has(task.id)} onClick={()=>setClosed(previous=>{const next=new Set(previous);if(next.has(task.id))next.delete(task.id);else next.add(task.id);return next;})}>{closed.has(task.id)?'▸':'▾'}</button> : <span className="disclosure-space" />}
        <span className={`task-state ${task.terminal?'terminal':''}`} aria-label={labels[task.status] ?? task.status}>{task.terminal?'✓':'○'}</span>
        <button className="task-select" aria-label={`编辑 ${task.title}`} aria-pressed={editing?.id===task.id} disabled={busy || editing!==null} onClick={()=>setEditing(task)}>{task.title}</button>
        <span className="task-meta">{labels[task.status] ?? task.status}</span>
        <span className="task-tags">{(task.tags ?? []).map(tag=><span key={tag}>#{tag} </span>)}</span>
      </div>
      {!closed.has(task.id) && <ul className="task-tree">{renderTasks(task.id,depth+1)}</ul>}
    </li>);
  }
  return <section className="task-layout" aria-label="项目与任务">
    <aside className="workspace-sidebar">
    <h2 className="brand">Nexus<span>个人工作空间</span></h2>
    <button className={!inboxOnly&&!projectView&&!filter.tag&&!filter.archived?'nav-item active':'nav-item'} disabled={busy} onClick={()=>{setInboxOnly(false);setProjectView('');const query={tag:'',text:'',archived:false};setFilter(query);void run(async()=>{},query);}}>所有任务 <span>{allTasks.length}</span></button>
    <button className={`nav-item ${inboxOnly?'active':''}`} disabled={busy} onClick={()=>{setInboxOnly(true);setProjectView('');const query={tag:'',text:'',archived:false};setFilter(query);void run(async()=>{},query);}}>收集箱 <span>{allTasks.filter(t=>t.status==='Inbox').length}</span></button>
    <button className={`nav-item ${filter.archived?'active':''}`} disabled={busy} onClick={()=>{setInboxOnly(false);setProjectView('');const query={tag:'',text:'',archived:true};setFilter(query);void run(async()=>{},query);}}>已归档</button>
    <h3>项目</h3>
    {projects.map(p=><button key={p.id} className={`nav-item ${projectView===p.id?'active':''}`} disabled={busy} onClick={()=>{setInboxOnly(false);setProjectView(p.id);setProjectId(p.id);setParentId('');}}>{p.name}</button>)}
    <form onSubmit={event => { event.preventDefault(); void run(async () => {
      await invoke('create_project', { id: crypto.randomUUID(), name: projectName }); setProjectName('');
    }); }}>
      <input disabled={busy} aria-label="项目名称" placeholder="项目名称" value={projectName} onChange={e => setProjectName(e.target.value)} />
      <button disabled={!isTauri() || busy || !projectName.trim()}>新建项目</button>
    </form>
    <h3>标签</h3>
    {[...new Set(allTasks.flatMap(t=>t.tags ?? []))].sort().map(tag=><button key={tag} disabled={busy} className={`nav-item ${filter.tag===tag?'active':''}`} onClick={()=>chooseTag(tag)}># {tag}</button>)}
    <p className="sidebar-hint">标签筛选与搜索共用任务数据。</p>
    </aside>
    <div className="task-list-pane">
    <header className="pane-heading"><h1>{projectView ? projects.find(p=>p.id===projectView)?.name : inboxOnly ? '收集箱' : filter.tag ? `# ${filter.tag}` : filter.archived ? '已归档任务' : '所有任务'}</h1><span>{visibleTasks.length} 项</span></header>
    <div className="task-tools">
    <form onSubmit={event => { event.preventDefault(); void run(async () => {
      await invoke('create_task', { request: { id: crypto.randomUUID(), title, project_id: projectId || null,
        parent_id: parentId || null, tags:splitTags(tags),
        source_note_id: linkSource ? source?.id ?? null : null, source_revision: linkSource ? source?.revision ?? null : null } });
      setTitle('');
    }); }}>
      <input disabled={busy} aria-label="任务标题" placeholder="＋ 添加任务" value={title} onChange={e => setTitle(e.target.value)} />
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
    <form className="task-search" onSubmit={e => { e.preventDefault(); void run(async () => {}); }}>
      <input disabled={busy} aria-label="标签搜索" placeholder="精确标签，如 工程" value={filter.tag} onChange={e => setFilter({...filter,tag:e.target.value})} />
      <input disabled={busy} aria-label="任务标题搜索" placeholder="搜索任务标题" value={filter.text} onChange={e => setFilter({...filter,text:e.target.value})} />
      <label><input disabled={busy} type="checkbox" checked={filter.archived} onChange={e => setFilter({...filter,archived:e.target.checked})} />已归档任务</label>
      <button disabled={!isTauri() || busy}>搜索任务</button>
    </form>
    </div>
    <div className="list-feedback">
      <button disabled={!isTauri() || busy} onClick={() => void run(async () => {})}>刷新任务</button>
      <span>{busy ? '正在更新…' : '本地工作区'}</span>
    </div>
    {error && <p role="alert" className="error-message">{error}</p>}
    <ul className="task-tree">{renderTasks()}</ul>
    {!busy && !visibleTasks.length && <p className="empty-state">没有符合条件的任务。可添加任务，或调整筛选。</p>}
    </div>
    <aside className="task-detail-pane" aria-label="任务详情">
    {editing && <TaskEditor key={editing.id} task={editing} projects={projects} candidates={allTasks} busy={busy} onDirty={setDetailDirty}
      onCancel={() => setEditing(null)} onSave={(fields,archived) => void run(async () => {
        await invoke('update_task',{id:editing.id,expectedRowVersion:editing.row_version,fields,archived}); setEditing(null);
      })} />}
    {!editing && <div className="empty-state"><h2>任务详情</h2><p>选择一个任务，查看项目、标签和父子关系。</p></div>}
    {editing && <div className="detail-actions">
      <h3>状态与来源</h3>
      {(() => { const task=editing; return <>
      {task.source_note_id && task.source_revision !== null && <button disabled={busy} onClick={() => onOpenSource(task.source_note_id!,task.source_revision!)}>查看来源版本 {task.source_revision}</button>}
      {!task.archived && task.allowed_targets.map(target => <button key={target} disabled={busy || detailDirty} onClick={() => void run(async () => { await invoke('transition_task', {
        id:task.id, expectedRowVersion:task.row_version, target, reopen:task.terminal,
      }); setEditing(null); })}>{task.terminal ? '重新打开' : labels[target] ?? target}</button>)}
      </>; })()}
      <p className="sidebar-hint">修改后保存或取消，再选择其他任务。</p>
    </div>}
    </aside>
  </section>;
}

function TaskEditor({ task, projects, candidates, busy, onSave, onCancel, onDirty }: {
  task: Task; projects: Project[]; candidates: Task[]; busy:boolean;
  onSave:(fields:{title:string;project_id:string|null;parent_id:string|null;tags:string[]},archived:boolean)=>void; onCancel:()=>void;
  onDirty:(dirty:boolean)=>void;
}) {
  const [title,setTitle] = useState(task.title);
  const [project,setProject] = useState(task.project_id ?? '');
  const [parent,setParent] = useState(task.parent_id ?? '');
  const [tags,setTags] = useState((task.tags ?? []).join(', '));
  const [archived,setArchived] = useState(task.archived ?? false);
  useEffect(()=>{ onDirty(title!==task.title || project!==(task.project_id??'') || parent!==(task.parent_id??'') || tags!==(task.tags??[]).join(', ') || archived!==(task.archived??false)); },[title,project,parent,tags,archived,task,onDirty]);
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
