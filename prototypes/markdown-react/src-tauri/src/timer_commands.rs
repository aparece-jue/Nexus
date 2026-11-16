use crate::Workspace;
use nexus_knowledge_store_probe::{tasks::TaskQuery, timer::TimerSnapshot};
use serde::Serialize;
use std::{
    sync::Arc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

pub struct TimerClock {
    wall: i64,
    monotonic: Instant,
}
impl TimerClock {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            wall: wall_ms()?,
            monotonic: Instant::now(),
        })
    }
    fn sample(&mut self) -> Result<i64, String> {
        let now = wall_ms()?;
        let elapsed =
            i64::try_from(self.monotonic.elapsed().as_millis()).map_err(|e| e.to_string())?;
        let verdict = validate_sample(self.wall, now, elapsed);
        self.wall = now;
        self.monotonic = Instant::now();
        verdict.map(|()| now)
    }
}
fn wall_ms() -> Result<i64, String> {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis(),
    )
    .map_err(|e| e.to_string())
}
fn validate_sample(previous: i64, now: i64, elapsed: i64) -> Result<(), String> {
    let delta = now.checked_sub(previous).ok_or("时钟跨度异常")?;
    if delta < 0 || elapsed > 30_000 || delta.abs_diff(elapsed) > 2_000 {
        return Err("时钟变化或采样中断，当前开放区间需确认".into());
    }
    Ok(())
}
#[derive(Serialize)]
pub struct TimerView {
    session: Option<TimerSnapshot>,
    live_ms: i64,
}

fn snapshot(store: &nexus_knowledge_store_probe::Store, now: i64) -> Result<TimerView, String> {
    let session = store.active_timer().map_err(|e| e.to_string())?;
    let live_ms = session
        .as_ref()
        .and_then(|s| s.open_since_ms)
        .map(|start| now.saturating_sub(start).max(0))
        .unwrap_or(0);
    Ok(TimerView { session, live_ms })
}

#[tauri::command]
pub async fn timer_snapshot(
    workspace: tauri::State<'_, Arc<Workspace>>,
) -> Result<TimerView, String> {
    let workspace = workspace.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut store = workspace.store.lock().map_err(|_| "存储状态不可用")?;
        let mut clock = workspace.clock.lock().map_err(|_| "计时时钟不可用")?;
        let now = match clock.sample() {
            Ok(now) => now,
            Err(error) => {
                store
                    .recover_interrupted_timers()
                    .map_err(|e| e.to_string())?;
                return Err(error);
            }
        };
        snapshot(&store, now)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn timer_action(
    workspace: tauri::State<'_, Arc<Workspace>>,
    operation: String,
    id: String,
    expected_version: i64,
    task_id: Option<String>,
) -> Result<TimerView, String> {
    let workspace = workspace.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut store = workspace.store.lock().map_err(|_| "存储状态不可用")?;
        let mut clock = workspace.clock.lock().map_err(|_| "计时时钟不可用")?;
        if operation == "finish_recovery" {
            store
                .finish_recovery(&id, expected_version)
                .map_err(|e| e.to_string())?;
            return snapshot(&store, wall_ms()?);
        }
        if operation == "start" {
            if store.active_timer().map_err(|e| e.to_string())?.is_some() {
                return Err("请先结束当前计时会话".into());
            }
            let task_id = task_id.as_deref().ok_or("请选择任务")?;
            let task = store
                .list_tasks(TaskQuery::default())
                .map_err(|e| e.to_string())?
                .into_iter()
                .find(|t| t.id == task_id)
                .ok_or("任务不存在或已归档")?;
            if task.row_version != expected_version {
                return Err("任务版本已变化，请刷新".into());
            }
        } else {
            let session = store
                .active_timer()
                .map_err(|e| e.to_string())?
                .ok_or("没有活动计时")?;
            if session.id != id || session.row_version != expected_version {
                return Err("计时版本已变化，请刷新".into());
            }
            if !matches!(
                (session.state.as_str(), operation.as_str()),
                ("running", "pause" | "finish") | ("paused", "resume" | "finish")
            ) {
                return Err("不允许此计时操作".into());
            }
        }
        let now = match clock.sample() {
            Ok(now) => now,
            Err(error) => {
                store
                    .recover_interrupted_timers()
                    .map_err(|e| e.to_string())?;
                return Err(error);
            }
        };
        if operation == "start" {
            store
                .start_timer(&id, task_id.as_deref().ok_or("请选择任务")?, now)
                .map_err(|e| e.to_string())?;
        } else {
            store
                .change_timer(&id, expected_version, &operation, now)
                .map_err(|e| e.to_string())?;
        }
        snapshot(&store, now)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clock_rejects_rollback_drift_and_sampling_gap() {
        assert!(validate_sample(1000, 2000, 1000).is_ok());
        assert!(validate_sample(1000, 900, 1000).is_err());
        assert!(validate_sample(1000, 9000, 1000).is_err());
        assert!(validate_sample(1000, 32001, 31001).is_err());
    }
}
