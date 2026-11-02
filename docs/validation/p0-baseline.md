# P0 基础核验记录

日期：2026-11-02｜状态：已开始，尚未完成复验

## 本轮实际检查

- Nexus 位于 research，检查时 HEAD 为 73867fa，跟踪 origin/research；存在用户未跟踪文件 docs/architecture/framework-baseline.md，已只读查看，未修改或提交。
- 可执行工具：Node v24.21.0、pnpm 12.3.4、rustc 1.98.1、cargo 1.98.1；以上为本次命令实际输出，不是历史记录。
- 用户 Cargo registry 目录存在，Cargo git 缓存目录不存在；目录存在不能证明所有锁定依赖都已缓存。
- Worktrace 当前工作目录仅显示 .git、LICENSE、README.md；源码需从之前确认的远程跟踪 dev 分支固定提交提取，不能在 main 目录直接运行应用测试。
- PowerShell 启动出现 InitializeDefaultDrives/FileSystem 警告，但本轮版本与文件读取命令仍返回实际结果；后续需按具体命令退出结果判断，不能忽略失败。

## 尚待执行

1. 重新读取 Worktrace dev 提交、许可、锁文件和构建要求，固定隔离复验基线。
2. 检查 Windows MSVC/WebView2 与依赖缓存是否满足实际构建要求；尚未确认可成功编译。
3. 隔离目录运行依赖完整性检查及关键测试，必要下载单独说明，不修改旧项目。
4. 确定对象、任务状态、毫秒单位、正文版本、IPC 与备份契约，完成小样和复用裁定。

本记录仅证明环境清点已开始，不表示 Worktrace 测试、Nexus 初始化或闭环模拟已经通过。

## 固定副本与首次离线复验

固定 Worktrace 提交为 `7b9cd47ee7e1b5d41a8a1403c9cbb21d9a6feb62`。通过 git archive 提取跟踪文件到 Nexus 的 `.local-validation/worktrace-7b9cd47`，原仓库未切分支、未修改。隔离目录加入忽略规则，不上传副本或生成产物。

实际尝试：`cargo test --offline --locked --manifest-path .local-validation/worktrace-7b9cd47/src-tauri/Cargo.toml --test domain_invariants --test timer_clock --test recovery_end_to_end --test stats_end_to_end --test backup_restore`。

结果：退出码 1，依赖解析报告本机 crates.io 缓存没有 rusqlite；编译和测试均未开始。不能将此结果记为业务测试失败或通过。后续需按原锁文件补齐依赖，不能使用 cargo update 改变版本来规避。

已读取固定版本 Cargo.toml 和仓库 LICENSE：根许可证文本为 Apache-2.0；传递依赖及迁入文件声明仍需逐项核对。本轮不将许可证文本读取当作完整许可审计。

后续检查发现 MSVC BuildTools 路径为 `C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools`。已按 `cargo fetch --locked` 补齐依赖，未修改锁定版本；执行期间测试等待 package cache 锁，不能据此声称开始编译或通过。

静态复核：stats.rs 统一 Today/JSON/Markdown 统计，按半开区间裁剪，区分确认/暂计/待确认和人工/机器/等待；db.rs 对磁盘库要求 WAL，将事务所有权留给服务层。这些是复用依据，不是本次测试结论。

迁入注意项：tauri.conf.json 的 CSP 为 null，bundle.targets 为 all；Nexus 必须另行收紧内容安全策略并确定安装形态，不能照搬旧配置。尚未验证本机 WebView2、前端依赖、安装及桌面启动。

## 本轮关键回归结果

`cargo fetch --locked` 最终退出 0；随后上述离线锁定测试完成。Windows/MSVC 构建成功，日志出现生成 worktrace_lib 导入库的 linker stdout 警告，未导致构建失败。

| 测试目标 | 通过 | 失败 |
| --- | --- | --- |
| backup_restore | 26 | 0 |
| domain_invariants | 18 | 0 |
| recovery_end_to_end | 2 | 0 |
| stats_end_to_end | 3 | 0 |
| timer_clock | 12 | 0 |
| 合计 | 61 | 0 |

完整本机日志：`.local-validation/worktrace-tests.log`（忽略目录，未上传；不得当作长期归档路径）。以上为本次实际执行结果，不沿用旧机验收。归档源码为固定提交，测试调用真实库与相应测试夹具，未启动 Worktrace 用户应用或接触个人数据库。

结论：计时、恢复、统计与数据库备份的关键回归在本机可编译且通过，可继续复用适配评估。尚未执行全量 Rust 测试、前端测试/构建、系统事件实测、编辑器与文件版本小样；P0 尚未结束，Nexus 最小闭环尚未实现。

## 前端复验与开发原则补充

用户明确要求优先复用接口、避免重复定义与多层嵌套、重视可读性及维护性；已写入根 AGENTS.md，后续实现执行此约定。

隔离目录首先执行 pnpm install --frozen-lockfile --offline --ignore-scripts，因缺少镜像元数据失败；未将其计为业务失败。随后按冻结锁文件联网补齐，pnpm 报告 supply-chain policy 检查通过，235 包安装完成；没有升级锁文件，禁用安装脚本。

本次实际执行 pnpm test：21 文件、229 项通过，退出 0；pnpm build：TypeScript 检查及 Vite 生产构建通过，退出 0。主 JS chunk 836.79 kB（gzip 263.41 kB），产生大于 500 kB 提示；后续按实际页面拆包，不提高警告阈值掩盖问题。完整本地日志位于忽略目录 worktrace-web-tests.log、worktrace-web-build.log。

已读 src/ipc.ts 与 commands/tasks.rs：现有 IPC 有统一错误规范化、版本响应过滤和事件订阅，任务命令复用 catalog 服务。迁入时先沿用现有命令、类型与版本机制；新增知识关联等语义才补必要接口，不再包一层纯转发服务。

当前累计证据为 Rust 选定 61 项、前端 229 项及前端生产构建通过；不是 Rust 全量回归或桌面安装验收。编辑器、正文/附件版本一致性与完整许可证核对仍待完成。
