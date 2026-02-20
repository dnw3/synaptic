# 技能

技能通过向系统提示词注入特定领域的指令来扩展 Deep Agent 的行为。技能由一个 `SKILL.md` 文件定义，包含 YAML 前置元数据和 Markdown 格式的指令正文。`SkillsMiddleware` 从后端文件系统发现技能，并向代理展示索引，代理随后可以通过 `read_file` 工具按需读取完整的技能文件。

## SKILL.md 格式

每个技能文件以 `---` 标记之间的 YAML 前置元数据开头，包含 `name` 和 `description` 字段：

```markdown
---
name: search
description: Search the web for information
---

# Search Skill

Detailed instructions for how to perform web searches effectively...
```

前置元数据字段：

| 字段 | 是否必需 | 描述 |
|------|----------|------|
| `name` | 是 | 技能的唯一标识符 |
| `description` | 否 | 在技能索引中显示的单行摘要（省略时默认为空字符串） |

解析器通过扫描 `---` 标记之间的行，查找 `name:` 和 `description:` 前缀来提取字段值。值可以使用单引号或双引号。

## 技能目录结构

将技能文件放在工作区根目录的 `.skills/` 目录下：

```text
my-project/
  .skills/
    search/SKILL.md
    testing/SKILL.md
    documentation/SKILL.md
  src/
    main.rs
```

每个技能位于自己的子目录中。`SkillsMiddleware` 通过列出配置的 `skills_dir` 下的目录并从每个目录读取 `{skills_dir}/{dir}/SKILL.md` 来发现技能。

## 发现机制

`SkillsMiddleware` 实现了 `AgentMiddleware` trait。在每次调用 `before_model()` 时，它会：

1. 通过后端的 `ls()` 方法列出技能目录中的条目。
2. 对每个目录条目，读取 `{dir}/SKILL.md` 的前 50 行。
3. 解析 YAML 前置元数据以提取 `name` 和 `description`。
4. 构建 `<available_skills>` 部分并追加到系统提示词中。

注入的部分如下所示：

```text
<available_skills>
- **search**: Search the web for information (read `.skills/search/SKILL.md` for details)
- **testing**: Guidelines for writing tests (read `.skills/testing/SKILL.md` for details)
</available_skills>
```

代理看到此索引后，当需要详细指令时，可以通过 `read_file` 工具读取完整的 SKILL.md 文件。

## 配置

技能默认启用。通过 `DeepAgentOptions` 进行配置：

```rust,ignore
use std::sync::Arc;
use synaptic::deep::{create_deep_agent, DeepAgentOptions};

let mut options = DeepAgentOptions::new(backend);
options.skills_dir = Some(".skills".to_string());  // 默认值
options.enable_skills = true;                       // 默认值
let agent = create_deep_agent(model, options)?;
```

要完全禁用技能，设置 `enable_skills = false`。要更改技能目录，将 `skills_dir` 设置为后端中的其他路径。

## 示例：添加 Rust 重构技能

在工作区中创建文件 `.skills/rust-refactoring/SKILL.md`：

```markdown
---
name: rust-refactoring
description: Best practices for refactoring Rust code
---

When refactoring Rust code, follow these guidelines:

1. Run `cargo clippy` before and after changes.
2. Prefer extracting functions over inline complexity.
3. Use `#[must_use]` on public functions that return values.
4. Write a test for every extracted function.
```

一旦该文件存在于后端中，`SkillsMiddleware` 会自动发现它并将其包含在系统提示词索引中。当代理遇到重构任务时，可以读取完整文件以获取详细指令。

没有编程方式的技能注册 API。所有技能都基于文件系统，在运行时通过扫描后端进行发现。

## 更多示例

### 代码审查技能

代码审查技能注入结构化的检查清单，使代理应用一致的审查标准：

```markdown
---
name: code-review
description: Structured code review checklist with severity levels
---

When reviewing code, evaluate each change against this checklist:

## Severity Levels
- **Critical**: Security vulnerabilities, data loss risks, correctness bugs
- **Major**: Performance issues, missing error handling, API contract violations
- **Minor**: Style inconsistencies, missing docs, naming improvements

## Review Checklist
1. **Correctness** — Does the logic match the stated intent?
2. **Error handling** — Are all failure paths covered?
3. **Security** — Any injection, auth bypass, or data exposure risks?
4. **Performance** — Unnecessary allocations, O(n²) loops, missing indexes?
5. **Tests** — Are new paths tested? Are edge cases covered?
6. **Naming** — Do names convey purpose without needing comments?

## Output Format
For each finding, report:
- File and line range
- Severity level
- Description and suggested fix
```

这将代理变成一个有纪律的审查者，按严重级别分类发现的问题，而不是给出非结构化的反馈。

### TDD 工作流技能

TDD 技能约束代理严格遵循红-绿-重构循环：

```markdown
---
name: tdd
description: Enforce test-driven development workflow
---

Follow the Red-Green-Refactor cycle strictly:

## Step 1: Red
- Write a failing test FIRST. Run it and confirm it fails.
- The test must describe the desired behavior, not the implementation.

## Step 2: Green
- Write the MINIMUM code to make the test pass.
- Do not add extra logic, optimizations, or edge case handling yet.
- Run the test and confirm it passes.

## Step 3: Refactor
- Clean up the implementation while keeping all tests green.
- Extract helpers, rename variables, remove duplication.
- Run the full test suite after each refactoring step.

## Rules
- Never write production code without a failing test.
- One behavior per test. If a test name contains "and", split it.
- Commit after each green-refactor cycle.
```

这防止了代理在测试存在之前就跳到编写实现代码。

### API 设计规范技能

规范技能编码团队统一的 API 标准，使代理创建的每个端点都遵循相同的模式：

```markdown
---
name: api-conventions
description: Team API design standards for REST endpoints
---

All REST endpoints must follow these conventions:

## URL Structure
- Use kebab-case for path segments: `/user-profiles`, not `/userProfiles`
- Nest resources: `/teams/{team_id}/members/{member_id}`
- Version prefix: `/api/v1/...`

## Request/Response
- Use `snake_case` for JSON field names
- Wrap collections: `{ "items": [...], "total": 42, "next_cursor": "..." }`
- Error format: `{ "error": { "code": "NOT_FOUND", "message": "..." } }`

## Status Codes
- 200 for success, 201 for creation, 204 for deletion
- 400 for validation errors, 404 for missing resources
- 409 for conflicts, 422 for semantic errors

## Naming
- List endpoint: `GET /resources`
- Create endpoint: `POST /resources`
- Get endpoint: `GET /resources/{id}`
- Update endpoint: `PATCH /resources/{id}`
- Delete endpoint: `DELETE /resources/{id}`
```

任何在 API 层工作的代理都会自动产生一致的端点，无需逐任务提醒。

### 多技能协作

当工作区中存在多个技能时，代理会在索引中看到所有技能，并根据当前任务读取相关的技能。考虑以下布局：

```text
my-project/
  .skills/
    code-review/SKILL.md
    tdd/SKILL.md
    api-conventions/SKILL.md
    rust-refactoring/SKILL.md
  src/
    main.rs
```

`SkillsMiddleware` 将完整索引注入系统提示词：

```text
<available_skills>
- **code-review**: Structured code review checklist with severity levels (read `.skills/code-review/SKILL.md` for details)
- **tdd**: Enforce test-driven development workflow (read `.skills/tdd/SKILL.md` for details)
- **api-conventions**: Team API design standards for REST endpoints (read `.skills/api-conventions/SKILL.md` for details)
- **rust-refactoring**: Best practices for refactoring Rust code (read `.skills/rust-refactoring/SKILL.md` for details)
</available_skills>
```

代理随后选择性地读取与当前任务匹配的技能：

- **"添加一个新的 `/users` 端点并编写测试"** — 代理读取 `api-conventions` 和 `tdd`，然后在应用 URL 和响应格式标准的同时遵循 TDD 循环。
- **"审查这个 Pull Request"** — 代理读取 `code-review`，并按严重级别输出发现的问题。
- **"重构认证模块"** — 代理读取 `rust-refactoring` 和 `code-review`（用于自检结果）。

技能是可组合的：每个技能贡献一组聚焦的指令，代理根据需要组合它们。这比单一的庞大系统提示词更易维护。

## 最佳实践

**保持技能聚焦简洁。** 每个技能应覆盖一个主题。20–50 行的 SKILL.md 是理想的。如果一个技能超过 100 行，考虑将其拆分。

**使用动作导向的语言。** 以指令形式编写说明（"提交前运行测试"、"URL 使用 kebab-case"），而不是描述（"理想情况下应该运行测试"）。

**使用 Markdown 结构化格式。** 使用标题、编号列表和粗体文本。代理处理结构化内容比处理散文段落更可靠。

**目录名使用 kebab-case。** 使用小写加连字符：`code-review/`、`api-conventions/`、`rust-refactoring/`。避免空格、下划线或 camelCase。

**技能 vs. 系统提示词。** 对于跨任务可复用且按名称可发现的指令，使用技能。对于每次交互都必须应用的指令，直接使用系统提示词。如果你发现自己在多个提示词中复制相同的指令，请将其提取为技能。
