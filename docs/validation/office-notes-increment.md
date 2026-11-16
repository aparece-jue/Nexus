# 办公版 O1：多便签增量

日期：2026-11-16。起点：dev 787ac48。结论：验证未完整，O1 尚未完成。

复用现有 Store.save/read、不可变正文版本和 createEditor 状态机；read_note/save_note 增加可选 id，省略时仍读取旧 desktop-note，保持旧调用兼容。新增 list_notes 只表达列表读取，不另建正文或标题权威。标题由当前正文首个非空行派生，搜索目前仅标题。新建通过原 save_note 在 expected_revision=0 写入，不新增同义命令。

实际路径：新建/选择/保存 → invoke → 原 commands → spawn_blocking → Mutex<Store> → SQLite 与文件 → 当前版本列表及编辑状态。读取任一正文继续校验哈希。未变更 schema、迁移或数据目录，不覆盖旧小样。原工作区依旧只用于合成资料，不声明可投入真实工作。

新增界面支持多便签、新建、标题筛选、选择、保存和重读；未保存/保存中/读取中禁止切换。读取期间隐藏编辑器以免旧文档被继续输入。未执行归档、任务、独立置顶窗、计时及屏幕叠加。

## 实际验证

- 存储 cargo test --offline --locked：10 passed。新增真实临时 SQLite/文件场景：两便签创建、更新一条、旧版本拒绝、关闭重开、历史版本和另一条正文不变。
- 桌面适配 cargo test --offline --locked：2 passed；读取历史与重开不重置内容。
- 两处 cargo fmt、cargo clippy --offline --locked --all-targets -- -D warnings 通过；环境存在增量缓存硬链接回退复制提示。
- 前端 pnpm test：4 passed，其中2项新界面流程测试使用模拟 IPC，分别覆盖未保存切换保护/保存后重挂载和并发冲突保留草稿；不是原生端到端。
- pnpm build：TypeScript 与生产构建通过，原有编辑器 chunk 大于500kB告警保留。
- 原编辑状态 node --test editor.test.mjs：4 passed。
- 新界面测试初次缺 jsdom 环境声明导致失败，补齐后复验全部通过。
- 未执行原生窗口新建/保存/重启、真实 IME 和多窗口交互；未完成 Nexus 全业务闭环。

## 增量自评与后续

按 nexus-code-review 核对受影响存储、IPC、编辑状态及前端调用。消除了固定 ID 读取路径重复实现；复用现有锁及后台磁盘调用。对象级版本冲突仍按原规则处理，不吞掉错误。列表暂时读取全部当前正文用于派生标题，规模化时需要明确索引及分页；当前不作性能达标承诺。

后续依次补便签归档、任务/项目契约及真实持久化，再接计时与窗口。Worktrace commands/tasks.rs、domain/task.rs 的既有状态映射见 worktrace-contract-mapping.md，当前未迁入也未另造状态机。正式 epoch、统一权限/错误、备份恢复与生产数据迁移仍待实现。O1–O4验收要求保持，不把本增量称为可用版。
