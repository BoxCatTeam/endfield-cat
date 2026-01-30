<p align="center">
  <img src="./src/assets/icon.webp" width="128" height="128" alt="EndCat Logo">
</p>

<h1 align="center">EndCat</h1>

<p align="center">
  专为 <strong>明日方舟：终末地 (Arknights: Endfield)</strong> 打造的现代化游戏工具箱。
</p>

<p align="center">
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/License-GPL_v2-blue.svg" alt="License">
  </a>
  <img src="https://img.shields.io/badge/Version-0.1.0-green.svg" alt="Version">
  <img src="https://img.shields.io/badge/Tauri-v2-orange.svg" alt="Tauri">
  <img src="https://img.shields.io/badge/Vue-v3-42b883.svg" alt="Vue">
</p>

<div align="center">
  
  **简体中文** | [English](docs/README_EN.md)

</div>

---

## 📖 简介

**EndCat** 是一个非官方的跨平台游戏工具箱，专为 *明日方舟：终末地* 设计。项目采用最新的 Web 技术与 Rust 构建，旨在提供流畅的游戏数据管理、唤醒记录（抽卡）分析等功能。

> **注意**: 本项目目前处于积极开发阶段。

## ✨ 功能特性

- **📊 唤醒记录分析 (Gacha Analysis)**
  - 支持导入/导出唤醒记录。
  - 可视化统计：保底计数、六星历史记录、平均抽数等。
  - 基于 ECharts 的交互式图表。
  - 本地数据库存储 (SQLite)，保障隐私与速度。

- **🗂️ 元数据管理**
  - 自动从远程源获取游戏数据（图片、文本）。
  - 支持多种源：GitHub (jsDelivr)、镜像源 (Mirror) 或自定义 CDN。
  - 智能缓存与增量更新。

- **🎨 现代化 UI**
  - 基于 **Varlet UI** (Material Design) 构建。
  - 完全响应式布局。
  - 支持 **深色模式** / 跟随系统主题。
  - 精美动态背景。

- **🛠️ 更多工具 (计划中)**
  - 游戏启动器 & 路径管理。
  - Wiki / 攻略集成。

## 🛠️ 技术栈

- **核心**: [Tauri v2](https://tauri.app) (Rust)
- **前端**: [Vue 3](https://vuejs.org) + [TypeScript](https://www.typescriptlang.org)
- **UI 框架**: [Varlet UI](https://varlet.gitee.io)
- **状态管理**: [Pinia](https://pinia.vuejs.org)
- **数据库**: SQLite (via `tauri-plugin-sql`)
- **路由**: Vue Router
- **构建工具**: Vite

## 🚀 开发指南

### 前置要求
- Node.js (v18+)
- Rust (最新稳定版)
- VS Code (推荐)

### 启动项目

1. **克隆仓库**
   ```bash
   git clone https://github.com/BoxCatTeam/endfield-cat.git
   cd endfield-cat
   ```

2. **安装依赖**
   ```bash
   npm install
   ```

3. **启动开发模式**
   ```bash
   npm run tauri dev
   ```

4. **构建生产版本**
   ```bash
   npm run tauri build
   ```

## 📄 许可证

本项目采用 **GPLv2 许可证** 进行授权。
Copyright © 2026 [BoxCat](https://boxcat.org).

详见 [LICENSE](LICENSE) 文件。

## ⚠️ 免责声明

本项目为非官方工具，与 **鹰角网络 (Hypergryph)** 及其旗下组织团体、工作室没有任何关联。所有游戏图片与数据版权归各自所有者所有。

- 本软件以“现状”提供，不作任何明示或默示的保证（包括但不限于可用性、稳定性、准确性或适销性/特定用途适用性）；因使用本软件造成的任何直接或间接损失由用户自行承担。
- 本软件仅供个人学习与研究使用，禁止商业化使用、再分发或提供任何增值服务；因违反上述限制产生的责任由使用者自行承担。
- 使用本软件须遵守所在国家/地区的适用法律法规、游戏/平台服务条款及知识产权要求；如对合规或安全存在疑虑，请立即停止使用并卸载。
- 本项目不采集、存储或上传用户的个人隐私数据，涉及的游戏数据均由用户自行选择导入/导出。
