# endfield-cat Agent Guide

> 项目名称暂定为 `endfield-cat`，一个「明日方舟：终末地」工具箱。

## 项目概览
- 前端：Vite + Vue 3 + TypeScript（ESM）
- 桌面端：Tauri 2
- 主要依赖：Pinia、vue-router、Varlet UI

## 开发约定
- TypeScript 严格模式；提交前确保 `yarn build` 通过。
- 风格跟随现有代码：ESM import、双引号、分号、Vue SFC 使用 `<script setup lang="ts">`。
- UI：应尽可能优先使用组件库（Varlet UI）现成组件来实现界面与交互，**禁止**手写 CSS 从零构建组件（如用 `div` 模拟按钮），避免重复造轮子。
- API 封装：所有 Tauri `invoke` 调用必须封装在 `src/api/` 的函数中，禁止在组件中直接调用。
- 状态管理：使用 Pinia store (`src/stores/`) 统一管理全局状态。
- 布局：**强烈建议**使用 `<var-space>` 组件来代替手写 `display: flex`，除非遇到无法满足的复杂场景。
- 图表：允许/鼓励使用图表库实现统计可视化（优先选成熟方案，如 ECharts）。
- Vite/Tauri 开发端口默认 `14200`（严格占用）；设置 `TAURI_DEV_HOST` 时 HMR 走 `14201`。
- 适配范围：不考虑移动端兼容（以桌面端/PC 体验为准）。

## 依赖版本（来自 `package.json`）
- `dependencies`：`vue@^3.5.13`、`vue-router@^4.6.4`、`pinia@^3.0.4`、`@varlet/ui@^3.13.1`、`@tauri-apps/api@^2`、`@tauri-apps/plugin-opener@^2`
- `devDependencies`：`vite@^6.0.3`、`@vitejs/plugin-vue@^5.2.1`、`typescript@~5.6.2`、`vue-tsc@^2.1.10`、`@tauri-apps/cli@^2`、`unplugin-auto-import@^21.0.0`、`unplugin-vue-components@^31.0.0`、`@varlet/import-resolver@^3.13.1`

## 环境要求
- Node.js（建议 `>= 18`）
- Yarn v1（仓库包含 `yarn.lock`）
- Tauri 开发/打包需要 Rust 工具链（`rustup` / `cargo`）

## 常用命令（来自 `package.json`）
- 安装依赖：`yarn`
- Web 开发：`yarn dev`
- Web 构建：`yarn build`（先跑 `vue-tsc --noEmit` 再 `vite build`）
- Web 预览：`yarn preview`
- Tauri：`yarn tauri dev` / `yarn tauri build` / `yarn tauri <subcommand>`

## GitHub 提交约定（Conventional Commits）
请勿提交 `AGENT.md`;`char.txt`;`findings.md`;`progress.md`;`task_plan.md`;`swagger.yaml`

- 操作约定：默认**不主动执行 `git commit` 或推送远程**，仅在用户明确要求时进行。
- 格式：`<type>(<scope>): <subject>`（`scope` 可选）
- `type`：`feat` `fix` `docs` `style` `refactor` `perf` `test` `build` `ci` `chore` `revert`
- 规范：一句话描述改动；尽量使用中文；不加句号；单次提交聚焦一个逻辑变更
- 关联 Issue/PR：在正文或尾部使用 `Refs #123` / `Closes #123`
- 破坏性变更：在 `type!:` 或尾部加 `BREAKING CHANGE: ...`
- 示例：
  - `feat(router): 添加权限守卫`
  - `fix(tauri): 修复问候指令的调用错误`
  - `chore: 升级依赖`

## 沟通与工具约定
- 沟通：默认使用中文
- 输出：回复以`[好的喵]`开头；总结以`[总结喵]`开头
- 命令：仅使用 Shell 原生命令（PowerShell/系统命令），不使用 Python 脚本

## 目录结构
- `src/`：Vue 前端
- `src/api/`：API 封装（Tauri 命令调用）
- `src/stores/`：Pinia 状态管理
- `src/theme/`：主题相关（主题变量/配色/主题入口）
- `src/router/`：路由相关（路由表与导航守卫）
- `public/`：静态资源
- `src-tauri/`：Tauri（Rust）工程

## 界面布局约定
- 桌面端左侧固定侧边栏导航（顶部品牌/中部菜单/底部用户与设置），右侧为页面内容区
- 标题栏独立于页面组件，置顶固定显示（路由切换仅更新标题内容）

## 数据存储
- 寻访记录使用 SQLite（Tauri `sql` 插件），数据库文件：`sqlite:endcat.db`（路径相对 `AppConfig` 目录）
- 账号信息（`uid` / `user_token` / `oauth_token` / `u8_token`）同样存储在 SQLite，用于多账号切换与后续同步

## 沉浸式标题栏（可选）
- 方案：使用 Tauri 无边框窗口 + 前端自绘标题栏
- 前端：标题栏容器添加 `data-tauri-drag-region` 作为拖拽区；按钮调用 `@tauri-apps/api/window` 的 `minimize()` / `toggleMaximize()` / `close()`
- 配置：在 `src-tauri/tauri.conf.json` 的窗口配置里将 `decorations` 设为 `false`（启用后会移除系统标题栏与原生按钮）
