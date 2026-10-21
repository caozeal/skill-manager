# skill-manager 项目长期约定


## 构建与交付约定（老曹 2026-09-09 定）

- **默认只构建 `.app`，不再打 dmg**。`src-tauri/tauri.conf.json` 的 `bundle.targets` 已改为 `["app"]`。
  原因：dmg 步骤（hdiutil 挂载）在沙箱内常被拦，且老曹本地直接用 .app，dmg 是多余步骤。
- 需要 dmg 分发时：临时 `tauri build -- --bundles dmg`，或用
  `hdiutil create -volname "Skill Manager" -srcfolder dmg_stage -ov -format UDZO <out.dmg>`
  （dmg_stage 里放 app + Applications 软链）。
- 构建命令（cargo 不在默认 PATH）：
  `cd /Users/caozeal/projects/github/skill-manager && export PATH="/Users/caozeal/.cargo/bin:$PATH" && npm run tauri build`
- 构建产出的 .app 自带签名通常是坏的（codesign -v 报 "code has no resources..."），
  交付前必须 `codesign --force --deep --sign - "src-tauri/target/release/bundle/macos/Skill Manager.app"`。
- 产物路径：`src-tauri/target/release/bundle/macos/Skill Manager.app`

## Agent / MCP 目标清单：一律动态，不写死（老曹 2026-09-22 定）

- **agent 清单的唯一来源是后端运行时探测**。`adapters/mod.rs` 的 registry 只是「能力定义」，
  每个 adapter 必须带 `probe_paths`（相对 $HOME 的痕迹路径）；`is_installed()` 任一命中才算存在。
  UI 列表 / 全量扫描 / 使用统计统一走 `installed_adapters()`。
- 卸载某个 agent 后其在界面上自动消失；**不要在 registry 里手工删条目**，否则重装后认不出来。
- MCP 侧同理：`mcp.rs::targets()` → `list_mcp_targets` 命令给出 `{id,name,writable,available}`，
  name 取自 adapters registry（与侧边栏显示一致）。前端用 `useMcpAgentMeta()` 派生名字与可写下拉。
- 新增 agent = 只在 registry 追加一条（含 probe），前端零改动。
- 禁止再出现类似 `MCP_AGENT_NAMES` / `MCP_WRITABLE_AGENTS` / `MCP_SOURCE_AGENTS` 这类写死常量。

## MCP 写入策略（各配置源）

- TOML（codex / grok）：`toml_edit` 原地改，注释与格式保留。
- JSON（claude / codebuddy）：serde_json 全量重写整个文件（这两份没有注释，可接受）。
- JSONC（opencode）：**段落级原地替换**，只动 `"mcp"` 段；段内检测到注释就拒绝写入并回滚。
  可写与否看后端 `mcp.rs::sources()` 的 `writable` 字段，前端只消费 `list_mcp_targets`。
- Claude 的项目级 MCP（`projects.*.mcpServers`）依旧只读。
- 无 `enabled` 字段的 agent（claude / opencode）禁用 = 移除条目 + 原始配置存影子文件
  `~/.skill-manager/mcp-disabled.json`，可原样恢复。
