# endcat Agent Guide

> 项目名称暂定为 `endcat`，一个「明日方舟：终末地」工具箱。

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

## 版本号约定（SemVer）
- 使用 `MAJOR.MINOR.PATCH`（语义化版本）。
- **三处版本号必须保持一致**：`package.json#version`、`src-tauri/tauri.conf.json#version`、`src-tauri/Cargo.toml#package.version`。
- 升级规则：
  - `MAJOR`：破坏性变更（不兼容配置/数据/行为，或需要用户手动处理）。
  - `MINOR`：新增功能或对外行为变化（尽量向后兼容；默认路径/存储规则变化建议算 `MINOR`）。
  - `PATCH`：纯修复/小优化，不改变对外行为。
- 预发布：使用 `-beta.N` / `-rc.N`（例如 `0.3.0-beta.1`），正式版去掉后缀。

## 分支与发布流程（dev/preview/master）
- 分支含义：
  - `dev`：日常开发分支
  - `preview`：预发布分支（用于生成 prerelease）
  - `master`：正式发布分支（用于生成 release draft）
- CI/发布工作流（见 `.github/workflows/`）：
  - `dev.yml`：推送到 `dev`/`preview`/`master` 时跑 `yarn build` + `cargo test`
  - `preview.yml`：`dev` → `preview` 的 PR 合并后，自动 bump `-pre.*` 版本并发布 prerelease 资产
  - `release.yml`：`preview` → `master` 的 PR 合并后，自动去掉预发布后缀并创建 draft release

## 依赖版本（来自 `package.json`）
- `dependencies`：`vue@^3.5.13`、`vue-router@^4.6.4`、`pinia@^3.0.4`、`@varlet/ui@^3.13.1`、`@tauri-apps/api@^2`、`@tauri-apps/plugin-opener@^2`、`@tauri-apps/plugin-dialog@^2`、`@tauri-apps/plugin-sql@^2`
- `devDependencies`：`vite@^6.0.3`、`@vitejs/plugin-vue@^5.2.1`、`typescript@~5.6.2`、`vue-tsc@^2.1.10`、`@tauri-apps/cli@^2`、`unplugin-auto-import@^21.0.0`、`unplugin-vue-components@^31.0.0`、`@varlet/import-resolver@^3.13.1`

## 环境要求
- Node.js（与 `package.json#engines.node` 保持一致，当前为 `>= 24 < 25`）
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
- 寻访记录/账号信息使用 SQLite（后端 `sqlx` 直接读写 DB 文件）：`{dataDir}/database/endcat.db`
- 数据目录 `dataDir` 可由用户配置（存储在 AppConfig 的 `config.json` 中的 `dataDir` 字段）
- 若未配置 `dataDir`：优先使用便携目录 `exe_dir/data/`（若存在），否则使用 `Documents/endcat`
- 元数据目录：`{dataDir}/metadata/`
- 打开数据目录：前端通过后端命令打开（避免 opener 插件的 path scope 限制）

## 沉浸式标题栏（可选）
- 方案：使用 Tauri 无边框窗口 + 前端自绘标题栏
- 前端：标题栏容器添加 `data-tauri-drag-region` 作为拖拽区；按钮调用 `@tauri-apps/api/window` 的 `minimize()` / `toggleMaximize()` / `close()`
- 配置：在 `src-tauri/tauri.conf.json` 的窗口配置里将 `decorations` 设为 `false`（启用后会移除系统标题栏与原生按钮）
