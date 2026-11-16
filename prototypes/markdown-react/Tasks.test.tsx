// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
const port = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true, invoke: port.invoke }));
import { Tasks } from './Tasks';
afterEach(() => { cleanup(); port.invoke.mockReset(); });

test('capture includes project and exact saved source version without copying the body', async () => {
  port.invoke.mockImplementation(async command => command === 'list_projects' ? [{id:'p',name:'Project',row_version:1}] : []);
  render(<Tasks source={{id:'note',revision:7}} onOpenSource={vi.fn()} />);
  await screen.findByRole('option', {name:'Project'});
  await waitFor(() => expect((screen.getByRole('button',{name:'刷新任务'}) as HTMLButtonElement).disabled).toBe(false));
  fireEvent.change(screen.getByLabelText('任务标题'),{target:{value:'评估电源'}});
  fireEvent.change(screen.getByLabelText('所属项目'),{target:{value:'p'}});
  fireEvent.click(screen.getByLabelText('关联当前已保存便签'));
  fireEvent.click(screen.getByRole('button',{name:'收集任务'}));
  await waitFor(() => expect(port.invoke).toHaveBeenCalledWith('create_task',{request:{id:expect.any(String),title:'评估电源',project_id:'p',source_note_id:'note',source_revision:7}}));
});

test('terminal task uses explicit reopen with its backend version', async () => {
  port.invoke.mockImplementation(async command => command === 'list_tasks' ? [{id:'t',title:'Completed',status:'Done',row_version:4,project_id:null,source_note_id:null,source_revision:null,allowed_targets:['Ready'],terminal:true}] : []);
  render(<Tasks source={null} onOpenSource={vi.fn()} />);
  const button = await screen.findByRole('button',{name:'重新打开'});
  await waitFor(() => expect((button as HTMLButtonElement).disabled).toBe(false));
  fireEvent.click(button);
  await waitFor(() => expect(port.invoke).toHaveBeenCalledWith('transition_task',{id:'t',expectedRowVersion:4,target:'Ready',reopen:true}));
});
