# P0 知识存储小样

日期：2026-11-02。代码位于 dev 分支 prototypes/knowledge-store；当前通过隔离 worktree 开发，research 主目录不切换。

复用 rusqlite 0.40.2 bundled 与缓存中的 sha2/tempfile，未创建生产 IPC 或通用存储框架。小样流程：内容哈希→不可变文件写入与 sync→SQLite 事务内版本检查和引用提交→读取时校验哈希。临时库和文件不触及个人数据。

实际执行 cargo test --offline：3 passed/0 failed；cargo fmt；cargo clippy --offline --all-targets -- -D warnings：通过。仅此小样，不等于 Nexus 应用已完成。

覆盖：两个版本关闭后复制到新目录并重读；旧版本写入拒绝后原正文不变；内容已写但没有引用时重开不产生新版本；附件缺失及正文哈希不匹配返回错误。

增量评审：小样仅一个 Store 和明确 save/read 入口，直接使用库接口，无纯转发包装；正式迁入时复用既有业务版本边界，不保留第二套生产接口。

限制：中断用“写文件但不提交引用”模拟，未强杀或断电；未注入真实 SQLite 提交 I/O 失败；单写入假设，尚未验证并发、目录同步耐久性、孤立文件清理、在线备份、完整清单和迁移。复制恢复仅在关闭数据库后执行，禁止当作运行中备份实现。编辑器 UI 与 FTS5 仍待验证。小样错误暂为标准错误对象，生产实现沿用统一错误契约。

## 后续用户决策

无需立即阻塞小样；在对应能力实施前确认：

- 日常知识编辑是否采用“Markdown 为正文，器件参数等结构化字段放属性面板”；建议采用，复杂自由排版后置。
- MVP 优先的一种真实工程模板及示例资料；在工程设计阶段前提供，建议选本人高频、输入参数明确的场景。
- 首发支持平台及分发范围；当前建议 Windows 个人使用，跨平台与公开发行另设验收。
- 是否有必须加密的本地工作资料；若有，在存入真实资料前确定数据库、附件、备份与密钥保护范围。
- 在模型接入前选择本地模型或云接口及允许发送的数据范围，当前无需决定。

技术细节如函数拆分、接口复用、短事务、缓存失效和测试组织由开发者按现行规范处理；涉及数据格式不兼容、破坏性迁移或新外部副作用时明确说明影响。

## 2026-11-03 增量验证

本轮仅补测试，不增加生产接口或包装层。cargo fmt、cargo test --offline --locked（6 passed/0 failed）及 cargo clippy --offline --locked --all-targets -- -D warnings 均通过。

新增验证：通过 SQLite trigger 拒绝版本 INSERT，确认同事务 revision 回滚、旧内容可读且移除注入后可以重试；通过 pending 文件位置的同名目录拒绝文件创建，确认未创建数据库引用。它们分别覆盖语句失败与文件创建失败，不等于真实磁盘满、提交阶段 I/O 失败或断电测试。

当前 bundled SQLite 可创建 FTS5 trigram 索引，MATCH 能找到“冗余电源”和“AD5422”；两字词“电源”MATCH 无结果，instr 子串查询可返回。正式检索需明确短词回退与分页/性能上限，不能宣称 FTS5 默认具备完整中文分词。该验证暂未把搜索索引接入 Store，也未证明索引更新和重建流程完成。

增量自评结论：本轮测试覆盖在其声明范围内可接受；P0 仍缺编辑器交互、生产迁移/在线备份与完整许可核对，不宣布整个知识模块完成。

## 编辑状态小样

dev 的 prototypes/markdown-editor 使用无新增依赖的 JavaScript 状态小样，read/save 端口语义对应现有 Store 边界，尚未创建正式 IPC。node --test 执行 4 项通过：冲突保留文本与原版本；保存中输入保持 dirty 且阻止重复保存；干净保存可重载且迟到读取不覆盖输入；切换对象后旧读取不能覆盖新选择。

增量评审修复了并发 load 的迟到响应问题，引入局部请求序号；没有新增 Manager 或纯转发层。上述使用测试替身，仅证明编辑状态行为，不证明实际 UI、Markdown 渲染、Rust 连接或落盘。下一步接入真实存储并补界面验证，本轮不宣称编辑器已完成。

## 编辑状态到真实存储的集成

新增 Rust 测试专用进程入口 src/main.rs，仅将 JSON read/save 请求映射到既有 Store，不重写业务存储；不是生产 API，也不作为未来 Tauri 之外的长期调用层。新增 serde_json 使用现有离线缓存，更新小样锁文件。

storage.integration.test.mjs 使用 Node 子进程调用真实 Rust 二进制、临时 SQLite 与文件，每次调用重新打开存储。实际验证中文 Markdown 保存为新版本、重新读取、旧版保留、外部更新后冲突草稿保留且数据库外部新版本不被覆盖。临时目录在用例结束清理，测试不访问个人工作区。

本轮 cargo build --offline、cargo test --offline --locked（6 passed）、node --test 两文件（5 passed，含 1 项真实跨语言集成）、cargo clippy --offline --locked --all-targets -- -D warnings 全部通过；格式化完成。跨语言测试必须先构建二进制，不能只运行 Node 后误用陈旧产物。

增量自评：业务边界复用 Store，测试连接仅承担协议转换，新增依赖限 JSON 解析。结论在上述模拟范围可接受；没有可见编辑组件、Markdown 渲染与正式 Tauri IPC 的验证，不宣称 UI 或 P0 整体完成。

## 本机编辑预览

新增 preview.html/preview.mjs：原生 textarea 编辑 Markdown 原文、保存新版本、重载当前版本，展示保存/冲突状态；文本使用 textContent/value 呈现，不执行 Markdown 中的 HTML。复用既有 createEditor，无新增编辑业务状态机。

preview-server.mjs 仅绑定 127.0.0.1 随机端口，使用独立临时工作区和既有 Rust 测试进程；限制 Host、Origin、JSON 请求、固定文档、请求体大小及静态文件白名单，设置 CSP。该服务不是生产 CLI/IPC；不作为正式桌面长期架构。Ctrl+C 正常退出时清理临时资料，强杀无法保证清理；不要存真实资料。

运行前先 cargo build --offline --locked --manifest-path prototypes/knowledge-store/Cargo.toml，再 node prototypes/markdown-editor/preview-server.mjs，打开其输出地址。小样验证原文编辑，不包含 Markdown 渲染预览。

实际 node --test 三文件：6 passed/0 failed，包含资源可访问、真实写入重读和跨 Origin 拒绝。首次受限执行出现本机 EACCES，正常权限重跑通过；不将权限失败算作功能通过。尚未操作真实浏览器，焦点、点击和布局未实测；P0 及正式 Tauri 桌面链仍未完成。

## 浏览器实际交互与断点修复

2026-11-03 使用本机预览及 IAB 实测：中文输入后 dirty 状态正确，重读禁用；Tab 从输入框到保存，Return 保存为版本 2；点击重读内容一致。整页刷新发现硬编码初始版本 1 导致显示旧内容，数据库未丢失。

修复：Store 增加读取当前对象版本的查询；测试进程 read revision=0 表示解析当前版本，响应带实际版本；页面先取当前版本再调用已有 load，历史版本读取仍显式指定。此语义仅为小样，正式接口仍按统一契约制定。

修复后真实浏览器刷新保持版本 2 与中文正文，截图见 images/editor-browser-save.png。回归：Rust 7 项、Node 6 项、Clippy 与构建均通过。不是 Tauri/安装验收；仅验证当前视口的基本可读性和上述键盘路径，没有宣称完整可访问性或多屏适配通过。
