# 编辑组件与桌面适配阶段评审

日期：2026-11-03｜开发分支：dev｜本轮起点：f3f377f；代码提交：ff4e2cd；既有研发档案合并提交：b88b59a。

## 范围与交付

用户新要求：后续工作记录、代码与相关资料一起进入 dev。已将 research 的已提交档案合入 dev，冲突仅为 .gitignore，保留双方忽略规则；主 research 工作区未跟踪的 framework-baseline.md 保持原状，未复制/覆盖。

prototypes/markdown-react 已改为 TypeScript，分别保留 SourceEditor、MarkdownPreview 和应用编排；CodeMirror 编辑，react-markdown/GFM/sanitize 渲染；原始 HTML 不解释、图片不请求、外链只显示。应用直接复用 markdown-editor/createEditor 的草稿、冲突和重复保存保护，没有重写第二套编辑状态机。

新增 src-tauri 为最小本机桌面适配，read_note/save_note 直接调用已有 Store，在 spawn_blocking 中执行磁盘与 SQLite 操作。使用内存持有的独立临时目录，仅固定合成笔记，正文上限 1 MiB；不访问真实资料，不实现 shell、文件选择或网络工具。该适配仍为技术小样，缺少生产 epoch、统一错误/授权和迁移，不能冒充正式领域服务。

## 实际验证

| 项目 | 结果与边界 |
| --- | --- |
| 类型及前端生产构建 | tsc --noEmit 与 Vite build 通过 |
| 渲染自动化 | Vitest 2 项通过：GFM 表格/任务列表/代码；危险 HTML、iframe、活动链接与远程图像不产生对应元素 |
| 桌面构建 | cargo build --offline 成功，Tauri 2.12.0/tauri-build 2.7.0；本机窗口在应用列表中可见 |
| 桌面读取边界 | cargo test --offline --locked：1 项通过，最新/历史版本、中文及不存在版本错误 |
| Rust 检查 | cargo fmt、cargo clippy --offline --locked --all-targets -- -D warnings 通过 |
| 浏览器实际操作 | 中文文本插入、实时标题渲染、任务列表/表格/代码展示、Ctrl+Z 撤销及 Windows Ctrl+Y 重做观察通过 |
| 桌面端到端 | 未通过验收：Computer Use 窗口访问审批超时，没有实际点击保存/重载证据 |

截图：images/markdown-component-browser.png。浏览器中文插入不等于真实 IME 拼音选词/组合输入验证；大文档、混合附件和复杂输入仍待测。Ctrl+Shift+Z 在本次 Windows 路径未生效，默认 Ctrl+Y 重做有效。

初次构建发现 CSS 类型声明和 tauri-build 版本不匹配，已补 Vite 类型并按 Tauri 依赖要求固定版本；Windows 图标使用项目生成的简单纯色 ICO。未下载来源不明图标。

## 可维护性与性能

按实际职责拆分编辑/渲染组件，通过 React.lazy 分开加载：主包从约 892.53 kB 降为 224.12 kB，渲染 166.30 kB，编辑 502.61 kB（均为压缩前产物）；不是总量同比减少。编辑 chunk 仍触发 500 kB 警告，未提高阈值隐藏；进一步优化应依据启动/编辑实测，不机械拆文件。

本轮为增量自评，不是独立评审。保留接口版本与单一存储边界，暂未引入 Manager/Facade 或通用插件框架。Tauri 小样的字符串错误、固定对象和临时目录不可直接推广到生产接口。

收尾发现 Vite 曾监听 src-tauri/target 中被 Rust 占用的 exe，出现 EBUSY 并退出。补 vite.config.ts 排除整个 src-tauri 目录，Rust 变更仍由 Tauri 自身构建/重启处理；不让前端 HMR 扫描二进制产物。该项是实际发现的开发环境缺陷，不将其隐去。

修复复验：启动新 Vite 实例，同时执行 TypeScript/Vite 生产构建与 Rust 测试（1 passed）；之后本机 GET 返回 200。该轮未再出现 EBUSY；不扩大为长期稳定性通过。

## 下一决策与未完成项

下一步计划从临时演示转为持久工作区，数据库、附件和备份保护方案会影响驱动、格式和恢复方式，因此将原计划“试用前”的加密决策前移到生产存储契约冻结前。请用户选择：首版不做应用级加密；仅加密导出备份；或数据库、正文附件与备份全部加密。任何方案都需明确密钥遗失与恢复边界；此前只用合成数据，不替用户默认采纳。

暂停依赖该决策的生产存储实现，尚未完成任务/知识关联闭环、持久重启恢复或发行版；没有将四个工作包全部标为完成。桌面 UI 审批、真实 IME、完整依赖许可审计及性能仍列为缺口，不能据构建成功宣称通过。

后续更新：用户已选择首版不做应用级加密，见 ADR 0004；上述决策暂停解除，继续持久工作区小样。此处保留当时阶段结论，不将后续选择改写为此前已经批准。
