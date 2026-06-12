# Code Review Report

## 审查范围

- 需求基线：`docs/01-09`、`docs/10`
- 审查对象：`src/`、`src-tauri/`
- 本轮性质：修复后复审

## 总体结论

本轮修复后，**上次审查中的核心阻塞问题已关闭**，当前代码可以通过主要质量门禁，核心业务链路达到可验收状态。

已确认通过的验证项：

- `npm run lint`
- `npm run test`
- `npm run build`
- `cargo test -q`

本轮仍保留 2 个中低优先级工程改进项：

1. 数据库迁移仍是源码内嵌 `MIGRATIONS`，和文档中的“版本化迁移目录”方案不完全一致。
2. 文档提到 Playwright 端到端测试，但仓库当前仍未接入。

这两项不再阻塞当前功能验收，但建议作为后续工程治理事项继续推进。

## 一、功能对比清单

| 模块 | 文档要求 | 当前状态 | 结论 |
|---|---|---|---|
| 届次管理 | 新建、编辑、切换、归档、解除归档、导出 | 已修复归档态切换限制，导出入口已打通，支持默认配置预填 | 通过 |
| 首页看板 | 当前届次、人数、作业、考勤、待处理、重点关注 | 已实现主要卡片和趋势展示 | 通过 |
| 学生管理 | 增删改查、筛选、Excel 导入导出、详情、重点关注 | 已实现主要能力 | 通过 |
| 作业管理 | 创建、登记、批量操作、未交导出、附件 | 已补齐备注登记闭环 | 通过 |
| 考勤管理 | 每日登记、全部正常、异常登记、查询、统计、导出 | 页面已修复构建问题，功能可用 | 通过 |
| 成绩管理 | 考试、科目、录入、导入、排名、趋势 | 页面已修复构建问题，功能可用 | 通过 |
| 班级事务 | 通知、值日、奖惩、班费 | 页面与命令基本齐备 | 通过 |
| 数据统计 | 当前届次统计、历史统计、学生画像、跨届对比 | 已覆盖主要统计视图 | 通过 |
| 系统设置 | 偏好、模板、备份恢复、按届导出、日志目录 | 默认值预填、导出入口与备份恢复链路可用 | 通过 |
| 发布与质量保障 | 可构建、可打包、具备测试保障 | 构建与测试门禁已恢复；E2E 与迁移治理仍可继续增强 | 基本通过 |

## 二、本轮已修复问题

### 已关闭的高优先级问题

1. **备份恢复/按届次导出遗漏 `exam_subject_config` 与 `class_fee`**
   - 修复结果：
     - 备份校验白名单已补齐这两张表。
     - 恢复删除顺序、导入顺序已补齐这两张表。
     - 按届次导出已包含 `exam_subject_configs.json` 与 `class_fee.json`。
     - Rust 集成测试已补充导出内容校验。
   - 关键位置：
     - [src-tauri/src/commands/backup.rs](/Users/nabijia/Desktop/projects/class-copilot/src-tauri/src/commands/backup.rs:244)
     - [src-tauri/src/commands/backup.rs](/Users/nabijia/Desktop/projects/class-copilot/src-tauri/src/commands/backup.rs:318)
     - [src-tauri/src/commands/backup.rs](/Users/nabijia/Desktop/projects/class-copilot/src-tauri/src/commands/backup.rs:741)
     - [src-tauri/tests/integration.rs](/Users/nabijia/Desktop/projects/class-copilot/src-tauri/tests/integration.rs:1738)

2. **前端生产构建失败**
   - 修复结果：
     - `AttendancePage` 的类型问题已修复。
     - `ScoreManagement` 的未使用导入/变量已清理。
     - `npm run lint` 与 `npm run build` 已通过。
   - 关键位置：
     - [src/features/attendance/AttendancePage.tsx](/Users/nabijia/Desktop/projects/class-copilot/src/features/attendance/AttendancePage.tsx:590)
     - [src/features/scores/ScoreManagement.tsx](/Users/nabijia/Desktop/projects/class-copilot/src/features/scores/ScoreManagement.tsx:1)

### 已关闭的中优先级问题

3. **归档态错误阻止切换到其他届次**
   - 修复结果：
     - 行操作改为按 `record.status` 判断，不再被全局 `isReadonly` 误伤。
   - 关键位置：
     - [src/features/cohorts/CohortList.tsx](/Users/nabijia/Desktop/projects/class-copilot/src/features/cohorts/CohortList.tsx:163)

4. **作业登记缺少备注编辑闭环**
   - 修复结果：
     - 作业详情页已支持备注输入并回写后端。
   - 关键位置：
     - [src/features/homework/HomeworkDetail.tsx](/Users/nabijia/Desktop/projects/class-copilot/src/features/homework/HomeworkDetail.tsx:120)

5. **届次管理页导出入口未打通**
   - 修复结果：
     - 设置页已读取 `?export=` 参数并自动预选届次、打开导出弹窗。
   - 关键位置：
     - [src/features/settings/SettingsPage.tsx](/Users/nabijia/Desktop/projects/class-copilot/src/features/settings/SettingsPage.tsx:68)

6. **默认学校/班主任/学期没有作用到新建届次**
   - 修复结果：
     - 届次新建弹窗已从系统设置中预填这些默认值。
   - 关键位置：
     - [src/features/cohorts/CohortList.tsx](/Users/nabijia/Desktop/projects/class-copilot/src/features/cohorts/CohortList.tsx:98)

7. **服务层保留不存在的 `export_homework` 死接口**
   - 修复结果：
     - 已移除死接口，避免后续误接入。
   - 关键位置：
     - [src/services/index.ts](/Users/nabijia/Desktop/projects/class-copilot/src/services/index.ts:349)

## 三、当前剩余风险

### P2

1. **迁移机制仍未完全对齐文档的版本化迁移方案**
   - 当前状态：
     - 仍使用源码内嵌 `MIGRATIONS` 执行 `CREATE TABLE IF NOT EXISTS`。
   - 风险：
     - 后续字段演进、线上排障和历史兼容性治理会比版本化迁移更难。
   - 位置：
     - [src-tauri/src/db.rs](/Users/nabijia/Desktop/projects/class-copilot/src-tauri/src/db.rs:8)

2. **缺少文档中提到的 Playwright E2E 自动化**
   - 当前状态：
     - 现有测试以 Vitest 与 Rust 集成为主，尚无 Playwright 依赖和脚本。
   - 风险：
     - 跨页面链路回归仍主要依赖人工验证。
   - 位置：
     - [package.json](/Users/nabijia/Desktop/projects/class-copilot/package.json:7)

## 四、安全性评估

### 正向项

- 已归档届次的写操作继续同时受前端与 Rust 命令层约束。
- 备份恢复链路在本轮修复后，数据完整性明显提升。
- 成绩、考勤、班费等关键写操作仍保留届次归属校验。

### 仍建议继续增强

- 后端输入标准化仍可继续收口，例如统一 `trim`、长度限制、枚举治理。
- 配置接口长期可考虑改成白名单式写入，而不是通用 `set_config`。

## 五、稳定性评估

### 本轮验证结果

- `npm run lint`: 通过
- `npm run test`: 通过
- `npm run build`: 通过
- `cargo test -q`: 通过

### 结论

当前版本的稳定性较上一轮明显提升，之前“不能构建、导出恢复链路不完整、入口未闭环”的问题已解决。剩余问题主要集中在工程治理层面，而不是当前业务功能正确性。

## 六、建议

### 建议尽快做

1. 把数据库初始化改造成真正的版本化迁移目录方案。
2. 为届次切换、按届次导出、备份恢复、学生导入、成绩导入补充 Playwright 级 E2E。

### 可作为后续优化

3. 继续收口后端输入校验，统一处理空白值、枚举和长度限制。
4. 视需要让更多设置项真正驱动业务行为，而不只是保存显示。
