# Worktrace 到 Nexus 的最小契约映射

日期：2026-11-03｜状态：已按固定源码核对，迁入方案待实现

源码基线：7b9cd47ee7e1b5d41a8a1403c9cbb21d9a6feb62。实际读取 commands/projects.rs、commands/tasks.rs、envelope.rs、domain/task.rs 和 storage/schema_v1.rs；文档不代表接口已经迁入。

## 保留的契约

| 现有事实 | Nexus 处理 |
| --- | --- |
| project.id、task.id 为稳定 TEXT 主键，相关引用使用原 ID | 注册为 entity 时使用同一 ID，不再生成第二份项目或任务 ID |
| project/task 有 row_version；app_meta 有 data_epoch/revision | 继续区分对象并发版本、库身份及业务水位；内容版本另记 |
| WriteEnvelope 提供 expected_data_epoch/expected_row_version | 单对象写入复用；计时同时涉及任务和会话版本时保留专用字段，不硬塞单版本信封 |
| create_project/name、rename_project、archive_project 和 create_task/title/project_id | 优先保留现有命令名和字段；不新增同义 project.create 包装层 |
| 计时与计划采用 *_ms | 内部与 IPC 保留毫秒，展示时转换，取代早期统一秒的草案 |
| 任务十状态及显式 reopen；Scheduled 在旧版拒绝写入 | 先保留状态机与存储字符串；UI 可简化展示，不能增加 next 等同义存储状态；开放排程须单独实现 |
| 归档项目不删除任务及历史 | 保留可追溯引用，清除操作另行设计 |

## 新增但不重复的领域

- capture_item 表示尚未确定为任务的资料/想法；旧 task.Inbox 表示已创建任务。澄清新条目为任务时调用现有领域创建逻辑并在同事务建立来源与幂等结果，不能在 UI 先建任务再补链接。
- entity 是统一引用锚点，不替代 project/task 的领域字段；对象及锚点在同事务创建，实体类型变更不自动允许。
- knowledge_item、document_version、attachment 管理正文与附件事实，不能复用 task.description 作为第二份知识正文。
- entity_link 的两端外键与允许类型由存储/服务约束，视觉连线不默认写业务关系。

## 迁入限制

不原地修改 Worktrace 已发布 schema_v1；若后续支持旧数据导入，用显式迁移注册原 ID 并保留历史。Nexus 新工作区与旧库的兼容关系必须先测试，不能仅复制旧数据库后改 identifier。

小样 Store 缺少 epoch、审计和统一对象机制，不能直接宣布为生产知识服务；正式迁入时接 WriteEnvelope/统一权限边界，而不是维持并行版本系统。首版幂等新建扩展需落在真实事务内，旧 create 请求本身没有幂等键，不误报为现有能力。

## 本轮增量评审

核对发现 Store.save 没有在文件写入前拒绝空 ID、负版本及 i64::MAX 版本，存在无意义文件写入及加一溢出风险。修复在既有 save 入口完成，并增加拒绝后文件和库均无变化的验证，没有增加验证包装服务。仍是小样，不代替生产 UUID 校验。
