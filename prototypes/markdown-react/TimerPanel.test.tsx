// @vitest-environment jsdom
import { cleanup,fireEvent,render,screen,waitFor } from '@testing-library/react';
import { afterEach,test,expect,vi } from 'vitest';
const port=vi.hoisted(()=>({invoke:vi.fn()}));
vi.mock('@tauri-apps/api/core',()=>({isTauri:()=>true,invoke:port.invoke}));
import { TimerPanel } from './TimerPanel';
afterEach(()=>{cleanup();port.invoke.mockReset();});
test('start sends task version but never a client timestamp',async()=>{
  port.invoke.mockResolvedValue({session:null,live_ms:0});
  render(<TimerPanel selected={{id:'t',title:'Task',row_version:4,status:'Ready'}} disabled={false} onChanged={vi.fn()} />);
  await waitFor(()=>expect(port.invoke).toHaveBeenCalledWith('timer_snapshot'));
  fireEvent.click(screen.getByRole('button',{name:'开始计时'}));
  await waitFor(()=>expect(port.invoke).toHaveBeenCalledWith('timer_action',{operation:'start',id:expect.any(String),expectedVersion:4,taskId:'t'}));
});
test('recovering session exposes conservative finish and excludes unknown live time',async()=>{
  port.invoke.mockResolvedValue({session:{id:'s',task_id:'t',state:'recovering',row_version:3,confirmed_ms:1000,pending_intervals:1},live_ms:0});
  render(<TimerPanel selected={null} disabled={false} onChanged={vi.fn()} />);
  fireEvent.click(await screen.findByRole('button',{name:'结束中断会话'}));
  await waitFor(()=>expect(port.invoke).toHaveBeenCalledWith('timer_action',{operation:'finish_recovery',id:'s',expectedVersion:3,taskId:null}));
  expect(screen.queryByRole('button',{name:'继续计时'})).toBeNull();
});
