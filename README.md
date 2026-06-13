# Class Copilot

<p align="center">
  <img src="./src-tauri/icons/app-icon.png" alt="Class Copilot Logo" width="140" />
</p>

<p align="center">
  面向班主任日常工作的本地优先桌面应用，聚焦班级档案、学生信息、作业、考勤、成绩、班级事务、统计分析与数据备份恢复。
</p>

<p align="center">
  <strong>Tauri 2</strong> · <strong>React</strong> · <strong>TypeScript</strong> · <strong>Rust</strong> · <strong>SQLite</strong>
</p>

## 项目简介

`Class Copilot` 是一个面向中小学班级管理场景的桌面端工具，目标是帮助班主任用更低的维护成本完成日常事务管理，而不是依赖复杂笨重的教务系统。

项目采用本地优先架构：

- 数据默认保存在本机 SQLite 数据库中，便于离线使用。
- 前端负责交互与可视化，后端通过 Tauri Commands 承担业务校验、文件操作和数据访问。
- 内置备份恢复与导入导出能力，方便迁移和长期维护。

## 已实现功能

### 核心业务

- 届次管理：创建、编辑、归档、解除归档、切换当前届次
- 学生管理：新增、编辑、导入、导出、详情查看、重点关注
- 作业管理：创建作业、批量登记、未交统计、趋势分析、导出
- 考勤管理：当日登记、请假记录、历史查询、周期筛选、导出
- 成绩管理：考试管理、考试科目配置、成绩导入、排名统计、趋势分析
- 班级事务：通知、值日、奖惩、班费管理
- 数据统计：首页看板、学生画像、跨届对比、Excel/PDF 导出
- 系统设置：模板下载、默认信息配置、数据备份恢复、届次导出、日志目录

### 数据安全

- 全量备份与恢复
- 恢复前自动备份当前数据库
- 备份文件结构校验与校验和校验
- 启动时自动执行数据库迁移
- 兼容旧版应用数据目录迁移

## 技术栈

| 层级 | 技术 |
|---|---|
| 桌面容器 | Tauri 2 |
| 前端 | React 18 + TypeScript + Vite |
| UI | Ant Design |
| 路由 | React Router |
| 状态管理 | Zustand + TanStack Query |
| 后端 | Rust |
| 数据库 | SQLite + SQLx |
| 导入导出 | calamine + rust_xlsxwriter + zip |
| 测试 | Vitest + React Testing Library + Cargo Test |

## 适用场景

- 班主任维护多个届次或班级档案
- 本地管理学生、作业、考勤、成绩与班级事务
- 希望具备离线使用、可备份、可迁移的数据管理工具
- 需要桌面端而不是纯 Web 的轻量工作台

## 快速开始

### 1. 环境要求

- Node.js 18+
- npm 9+
- Rust stable
- Tauri 2 构建环境

不同平台的 Tauri 前置依赖可参考官方文档安装。macOS 下通常需要 Xcode Command Line Tools。

### 2. 安装依赖

```bash
npm install
```

### 3. 启动前端开发环境

```bash
npm run dev
```

### 4. 启动 Tauri 桌面应用

```bash
npm run tauri dev
```

### 5. 生产构建

```bash
npm run build
```

### 6. 打包桌面应用

```bash
npm run tauri build
```

如果你在 macOS 上需要构建 DMG，也可以使用仓库内脚本：

```bash
./scripts/build-macos-dmg.sh
```

## 常用脚本

| 命令 | 说明 |
|---|---|
| `npm run dev` | 启动 Vite 开发服务器 |
| `npm run build` | 前端生产构建 |
| `npm run lint` | 执行 ESLint 检查 |
| `npm run test` | 运行前端测试 |
| `npm run test:watch` | 监听模式运行测试 |
| `npm run test:coverage` | 生成前端测试覆盖率 |
| `npm run typecheck` | TypeScript 类型检查 |
| `npm run tauri dev` | 启动桌面开发模式 |
| `npm run tauri build` | 打包桌面应用 |
| `cargo test` | 运行 Rust 单元与集成测试 |

## 项目结构

```text
class-copilot/
├── docs/                 项目文档
├── public/               静态资源
├── scripts/              构建与辅助脚本
├── src/                  React 前端
│   ├── app/              全局状态、Provider、路由
│   ├── features/         功能模块页面
│   ├── hooks/            复用 Hooks
│   ├── layouts/          应用布局
│   ├── services/         Tauri Command 调用封装
│   └── types/            前端类型定义
└── src-tauri/            Rust / Tauri 后端
    ├── src/commands/     Tauri Commands
    ├── src/db.rs         数据库初始化与迁移
    └── icons/            应用图标与 Logo 资源
```

## 数据与备份

- 默认数据存储为本地 SQLite 数据库
- 应用启动时会自动初始化数据库并执行迁移
- 系统设置页可执行全量备份与恢复
- 恢复前会自动备份当前数据库，避免误操作导致数据不可逆
- 支持按届次导出数据，便于归档与迁移

## 文档索引

项目已附带较完整的中文设计与交付文档：

- [docs/01-项目背景与建设目标.md](./docs/01-项目背景与建设目标.md)
- [docs/02-需求分析说明书.md](./docs/02-需求分析说明书.md)
- [docs/03-功能清单.md](./docs/03-功能清单.md)
- [docs/04-业务流程设计.md](./docs/04-业务流程设计.md)
- [docs/05-页面原型说明.md](./docs/05-页面原型说明.md)
- [docs/06-数据库设计.md](./docs/06-数据库设计.md)
- [docs/07-技术方案.md](./docs/07-技术方案.md)
- [docs/08-开发计划.md](./docs/08-开发计划.md)
- [docs/09-安装与使用说明.md](./docs/09-安装与使用说明.md)
- [docs/10-版本说明与发布记录.md](./docs/10-版本说明与发布记录.md)

## 测试与质量

当前仓库包含前端单元测试、Hooks 测试，以及 Rust 侧单元测试与集成测试，覆盖的重点包括：

- 届次切换与只读归档行为
- Excel 导入与字段映射
- 成绩、作业、考勤等核心业务链路
- 备份恢复与一致性校验
- 跨届数据隔离与导出

建议在提交前至少执行：

```bash
npm run lint
npm run test
npm run build
cd src-tauri && cargo test
```

## 路线图

欢迎继续扩展以下方向：

- 发布页与安装包自动化
- 更丰富的统计图表和趋势分析
- 云同步或多设备协作能力
- 家长端 / 学生端衍生版本
- AI 周报、异常学生识别、智能提醒