# 当前开发状态

更新：2026-11-03。当前代码与工作记录维护于 dev，main 未晋级发行。

- Worktrace 固定版本本机复验：Rust 791 通过/1 忽略；前端 229 通过及生产构建；这些是旧项目基线证据。
- 知识存储小样：7 项 Rust 测试；编辑状态与存储/HTTP 集成 6 项 Node 测试，已在之前提交记录中通过。
- 本轮：React TypeScript 构建、2 项渲染测试、Tauri 构建、1 项桌面读取边界测试及 Clippy 通过；浏览器中文插入与预览、撤销重做观察通过。
- 桌面窗口可启动，但窗口控制授权超时，原生编辑保存尚未实测。浏览器测试不能代替此项。
- 阶段结论：P0/编辑适配仍未完全验收，知识—任务—计时—回顾—备份的 Nexus 闭环尚未完成。
- 当前决策点：持久工作区加密范围，见 validation/2026-11-03-editor-desktop-review.md；在决定前不转入真实资料存储。

运行组件：在 prototypes/markdown-react 执行 pnpm dev（浏览器仅组件模式）；pnpm tauri dev 自动启动本机前端与桌面临时合成工作区。需要 Node/pnpm/Rust、MSVC 与 WebView2。先 pnpm install --frozen-lockfile，再执行 pnpm test 和 pnpm build；不要把临时小样用于工作资料。
