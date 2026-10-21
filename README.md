# Skill Manager — Agent 技能管理

macOS 桌面应用（Tauri 2 + React + TypeScript），用于查看和管理各 AI coding agent 挂载的 skill。

## 支持管理的 Agent

下表是「本机能识别哪些 agent」的映射。**界面只展示本机实际存在的 agent**：每次拉取列表时按
「探测路径」检查 `$HOME` 下是否有对应痕迹，命中才出现在侧边栏 / 总览 / 使用统计里。
所以卸载某个 agent 后它的条目会自动消失，不需要改代码。

| Agent | 技能目录 | 形态 | 探测路径（任一存在即展示） |
|---|---|---|---|
| Claude Code | `~/.claude/skills` | 混合（真实目录 + 符号链接） | `~/.claude`、`~/.claude.json` |
| Codex | `~/.codex/skills` | 内置(`.system`) + 本地安装；**用户级全局技能官方位置为 `~/.agents/skills`（中央仓库，直接生效）** | `~/.codex` |
| OMP | `~/.omp/agent/managed-skills` | 真实目录 | `~/.omp` |
| Kimi Code | `~/.kimi-code/skills` | 本地安装 + 符号链接挂载；**全局技能官方位置为 `~/.agents/skills`（中央仓库，直接生效）** | `~/.kimi-code` |
| Grok | `~/.grok/skills` | 全符号链接 | `~/.grok` |
| CodeBuddy | `~/.codebuddy/skills` | 全符号链接 | `~/.codebuddy` |
| OpenClaw | `~/.openclaw/skills` | 混合 | `~/.openclaw` |
| 中央仓库 | `~/.agents/skills` | 跨 agent 共享技能仓 | `~/.agents/skills` |
| DeepSeek Harness | —（无本地目录） | **全局技能来自 `~/.agents/skills`（中央仓库，直接生效）** | `~/.dsh`、`~/.deepseek` |
| OpenCode | `~/.config/opencode/skills` | 技能发现：本地目录 + `~/.agents/skills`（全局）+ `~/.claude/skills`（Claude 兼容） | `~/.config/opencode` |

新增一个 agent 只需在 `src-tauri/src/adapters/mod.rs` 的 registry 里追加一条（带 `probe` 探测路径），
前端侧边栏、Agent 页、MCP 目标下拉都会自动出现，无需改任何前端常量。
MCP 可写目标同理，由后端 `list_mcp_targets` 按「源是否可写 + 配置文件是否存在」运行时决定。

## 功能

- **总览**：各 agent 技能数、中央仓库统计、全局搜索
- **Agent 页**：技能表格（挂载类型徽标 / 名称 / 描述 / 来源 / 更新时间 / 大小 / 文件数）
- **挂载矩阵**：技能 × agent 关系一览（符号链接 / 本地真实目录 / 全局 / 兼容 / 已禁用 / 未挂载 / 不适用）
- **详情抽屉**：SKILL.md 渲染预览、文件树、frontmatter 编辑（自动备份）
- **管理操作**：从中央仓库挂载（创建符号链接）、卸载（仅删除符号链接）、编辑 frontmatter、**禁用/启用**（重命名隐藏，可逆）、**删除**（移动到 `~/.skill-manager-trash` 回收站）

## 安全设计

- 卸载只允许删除**符号链接**，真实目录一律拒绝
- **删除**仅真实目录，且移动至 `~/.skill-manager-trash/<时间戳>-<技能名>` 回收站（可手动恢复），不做不可逆物理删除
- **禁用**通过重命名为 `.技能名`（隐藏目录）实现，agent 扫描跳过；可随时启用
- 全局 / 兼容 / `.system` 技能不允许禁用与删除
- `.system` 内置技能、DSH / OpenCode 为只读
- 不触碰任何含 API key 的配置文件（settings.json / config.yml 等）
- lock 文件（`~/.agents/.skill-lock.json`）只读，不写入
- frontmatter 编辑前生成 `SKILL.md.bak-<时间戳>` 备份

## 开发

```bash
npm install
npm run tauri dev        # 开发模式
npm run tauri build      # 打包 .app/.dmg
```

## 架构

```
src-tauri/src/
├── adapters/       # Agent 注册表（可插拔，新增 agent 只需加一行配置）
├── scanner.rs      # 扫描：lstat 区分真实目录/符号链接/断链、frontmatter 解析、体积统计
├── lockfile.rs     # 只读解析 .skill-lock.json
├── fsops.rs        # 挂载/卸载/frontmatter 编辑（含安全硬规则）
├── commands.rs     # Tauri 命令
└── model.rs        # 数据模型（serde camelCase）
```

前端 `src/`：Dashboard / AgentPage / MatrixPage 三个页面 + 详情抽屉组件。
