import { useEffect, useRef, useState } from 'react';
import { invoke, isTauri } from '@tauri-apps/api/core';
type Session = { id:string; task_id:string; state:string; row_version:number; confirmed_ms:number; pending_intervals:number };
type View = { session:Session|null; live_ms:number };
function duration(ms:number) { const seconds=Math.floor(ms/1000);return [Math.floor(seconds/3600),Math.floor(seconds/60)%60,seconds%60].map(value=>value.toString().padStart(2,'0')).join(':'); }

export function TimerPanel({ selected, disabled, onChanged }: {
  selected:{id:string;title:string;row_version:number;status:string}|null;disabled:boolean;onChanged:()=>void;
}) {
  const [view,setView]=useState<View>({session:null,live_ms:0});
  const [busy,setBusy]=useState(false);
  const [error,setError]=useState('');
  const gate=useRef(false);
  const sequence=useRef(0);
  useEffect(()=>{
    if(!isTauri())return;
    let active=true;
    async function read() {
      if(gate.current)return;
      gate.current=true;const request=++sequence.current;
      try { const next=await invoke<View>('timer_snapshot');if(active&&request===sequence.current)setView(next); }
      catch(failure) {if(active&&request===sequence.current)setError(String(failure));}
      finally {gate.current=false;}
    }
    void read();const interval=setInterval(()=>void read(),1000);
    return ()=>{active=false;sequence.current++;clearInterval(interval);};
  },[]);
  async function action(operation:string) {
    if(gate.current)return;
    gate.current=true;setBusy(true);setError('');++sequence.current;
    try {
      const next=await invoke<View>('timer_action',{operation,id:operation==='start'?crypto.randomUUID():view.session?.id,
        expectedVersion:operation==='start'?selected?.row_version:view.session?.row_version,taskId:operation==='start'?selected?.id:null});
      setView(next);onChanged();
    } catch(failure){setError(String(failure));}
    finally{gate.current=false;setBusy(false);}
  }
  const session=view.session;
  return <section className="timer-panel" aria-label="任务计时">
    <h3>任务计时</h3>
    {session ? <>
      <p>{selected?.id===session.task_id?selected.title:'当前计时任务'} · {({running:'计时中',paused:'已暂停',recovering:'中断待处理'} as Record<string,string>)[session.state] ?? session.state}</p>
      <strong>{duration(session.confirmed_ms+(view.live_ms??0))}</strong>
      <p>已确认 {duration(session.confirmed_ms)} · 暂计 {duration(view.live_ms??0)}</p>
      {session.state==='running'&&<button disabled={disabled||busy} onClick={()=>void action('pause')}>暂停计时</button>}
      {session.state==='paused'&&<button disabled={disabled||busy} onClick={()=>void action('resume')}>继续计时</button>}
      {session.state!=='recovering'&&<button disabled={disabled||busy} onClick={()=>void action('finish')}>结束计时</button>}
      {session.state==='recovering'&&<><p>开放区间未计入确认工时。结束会话会保留待确认记录。</p><button disabled={disabled||busy} onClick={()=>void action('finish_recovery')}>结束中断会话</button></>}
    </> : <><p>{selected ? selected.title : '选择待办或进行中的叶任务开始计时。'}</p><button disabled={!isTauri()||disabled||busy||!selected||!['Ready','Doing'].includes(selected.status)} onClick={()=>void action('start')}>开始计时</button></>}
    {error&&<p role="alert">{error}<button onClick={()=>setError('')}>关闭计时提示</button></p>}
  </section>;
}
