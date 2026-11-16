// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
const port = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true, invoke: port.invoke }));
import { Tasks } from './Tasks';
afterEach(() => { cleanup(); port.invoke.mockReset(); });

test('clean details allow switching, but an edited draft blocks switching until cancelled', async () => {
  const base={project_id:null,terminal:false,allowed_targets:['Ready'],status:'Inbox',row_version:1,tags:[],archived:false,source_note_id:null,source_revision:null,parent_id:null};
  port.invoke.mockImplementation(async command=>command==='list_tasks' ? [{...base,id:'one',title:'First'},{...base,id:'two',title:'Second'}] : []);
  render(<Tasks source={null} onOpenSource={vi.fn()} />);
  fireEvent.click(await screen.findByRole('button',{name:'编辑 First'}));
  fireEvent.click(screen.getByRole('button',{name:'编辑 Second'}));
  expect((screen.getByLabelText('编辑任务标题') as HTMLInputElement).value).toBe('Second');
  fireEvent.change(screen.getByLabelText('编辑任务标题'),{target:{value:'Second draft'}});
  expect((screen.getByRole('button',{name:'编辑 First'}) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole('button',{name:'取消编辑'}));
  // A cancelled editor must not leave the list locked by its previous dirty state.
  await waitFor(()=>expect((screen.getByRole('button',{name:'编辑 First'}) as HTMLButtonElement).disabled).toBe(false));
});

test('collapsing a parent hides only its child and preserves the detail draft', async () => {
  const base={project_id:null,terminal:false,allowed_targets:['Ready'],status:'Inbox',row_version:1,tags:['工程'],archived:false,source_note_id:null,source_revision:null};
  port.invoke.mockImplementation(async command=>command==='list_tasks' ? [{...base,id:'p',title:'Parent',parent_id:null},{...base,id:'c',title:'Child',parent_id:'p'}] : []);
  render(<Tasks source={null} onOpenSource={vi.fn()} />);
  const collapse=await screen.findByRole('button',{name:'折叠 Parent'});
  fireEvent.click(collapse);
  expect(screen.queryByRole('button',{name:'编辑 Child'})).toBeNull();
  fireEvent.click(screen.getByRole('button',{name:'展开 Parent'}));
  fireEvent.click(screen.getByRole('button',{name:'编辑 Child'}));
  fireEvent.change(screen.getByLabelText('编辑任务标题'),{target:{value:'Child draft'}});
  expect((screen.getByRole('button',{name:'待办'}) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole('button',{name:'折叠 Parent'}));
  expect((screen.getByLabelText('编辑任务标题') as HTMLInputElement).value).toBe('Child draft');
});

test('tag and title search use the existing task query including archive scope', async () => {
  port.invoke.mockResolvedValue([]);
  render(<Tasks source={null} onOpenSource={vi.fn()} />);
  await waitFor(() => expect((screen.getByRole('button',{name:'搜索任务'}) as HTMLButtonElement).disabled).toBe(false));
  fireEvent.change(screen.getByLabelText('标签搜索'),{target:{value:'工程'}});
  fireEvent.change(screen.getByLabelText('任务标题搜索'),{target:{value:'电源'}});
  fireEvent.click(screen.getByLabelText('已归档任务'));
  fireEvent.click(screen.getByRole('button',{name:'搜索任务'}));
  await waitFor(() => expect(port.invoke).toHaveBeenCalledWith('list_tasks',{query:{tag:'工程',text:'电源',archived:true}}));
});

test('subtask creation carries parent project and multiple tags in one request', async () => {
  port.invoke.mockImplementation(async command => command === 'list_projects' ? [{id:'p',name:'Project',row_version:1}] : command === 'list_tasks' ? [{id:'parent',title:'Parent',project_id:'p',terminal:false,allowed_targets:[],status:'Inbox',row_version:1,tags:[],parent_id:null,archived:false,source_note_id:null,source_revision:null}] : []);
  render(<Tasks source={null} onOpenSource={vi.fn()} />);
  await screen.findByRole('option',{name:'Parent'});
  await waitFor(() => expect((screen.getByRole('button',{name:'刷新任务'}) as HTMLButtonElement).disabled).toBe(false));
  fireEvent.change(screen.getByLabelText('父任务'),{target:{value:'parent'}});
  fireEvent.change(screen.getByLabelText('任务标题'),{target:{value:'Child'}});
  fireEvent.change(screen.getByLabelText('任务标签'),{target:{value:'工程, Hardware'}});
  fireEvent.click(screen.getByRole('button',{name:'收集任务'}));
  await waitFor(() => expect(port.invoke).toHaveBeenCalledWith('create_task',{request:{id:expect.any(String),title:'Child',project_id:'p',parent_id:'parent',tags:['工程','Hardware'],source_note_id:null,source_revision:null}}));
});

test('capture includes project and exact saved source version without copying the body', async () => {
  port.invoke.mockImplementation(async command => command === 'list_projects' ? [{id:'p',name:'Project',row_version:1}] : []);
  render(<Tasks source={{id:'note',revision:7}} onOpenSource={vi.fn()} />);
  await screen.findByRole('option', {name:'Project'});
  await waitFor(() => expect((screen.getByRole('button',{name:'刷新任务'}) as HTMLButtonElement).disabled).toBe(false));
  fireEvent.change(screen.getByLabelText('任务标题'),{target:{value:'评估电源'}});
  fireEvent.change(screen.getByLabelText('所属项目'),{target:{value:'p'}});
  fireEvent.click(screen.getByLabelText('关联当前已保存便签'));
  fireEvent.click(screen.getByRole('button',{name:'收集任务'}));
  await waitFor(() => expect(port.invoke).toHaveBeenCalledWith('create_task',{request:{id:expect.any(String),title:'评估电源',project_id:'p',source_note_id:'note',source_revision:7,parent_id:null,tags:[]}}));
});

test('terminal task uses explicit reopen with its backend version', async () => {
  port.invoke.mockImplementation(async command => command === 'list_tasks' ? [{id:'t',title:'Completed',status:'Done',row_version:4,project_id:null,source_note_id:null,source_revision:null,allowed_targets:['Ready'],terminal:true}] : []);
  render(<Tasks source={null} onOpenSource={vi.fn()} />);
  fireEvent.click(await screen.findByRole('button',{name:'编辑 Completed'}));
  const button = await screen.findByRole('button',{name:'重新打开'});
  await waitFor(() => expect((button as HTMLButtonElement).disabled).toBe(false));
  fireEvent.click(button);
  await waitFor(() => expect(port.invoke).toHaveBeenCalledWith('transition_task',{id:'t',expectedRowVersion:4,target:'Ready',reopen:true}));
});
