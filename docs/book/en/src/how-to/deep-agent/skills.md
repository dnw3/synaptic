# Skills

Skills extend a Deep Agent's behavior by injecting domain-specific instructions into the system prompt. A skill is defined by a `SKILL.md` file with YAML frontmatter and a body of Markdown instructions. The `SkillsMiddleware` discovers skills from the backend filesystem and presents an index to the agent, which can then read the full skill file on demand via the `read_file` tool.

## SKILL.md Format

Each skill file starts with YAML frontmatter between `---` markers containing `name` and `description` fields:

```markdown
---
name: search
description: Search the web for information
---

# Search Skill

Detailed instructions for how to perform web searches effectively...
```

The frontmatter fields:

| Field | Required | Description |
|-------|----------|-------------|
| `name` | yes | Unique identifier for the skill |
| `description` | no | One-line summary shown in the skill index (defaults to empty string if omitted) |

The parser extracts `name` and `description` by scanning lines between the `---` markers for `name:` and `description:` prefixes. Values may optionally be quoted with single or double quotes.

## Skills Directory Structure

Place skill files in a `.skills/` directory at the workspace root:

```text
my-project/
  .skills/
    search/SKILL.md
    testing/SKILL.md
    documentation/SKILL.md
  src/
    main.rs
```

Each skill lives in its own subdirectory. The `SkillsMiddleware` discovers them by listing directories under the configured `skills_dir` and reading `{skills_dir}/{dir}/SKILL.md` from each.

## How Discovery Works

The `SkillsMiddleware` implements the `AgentMiddleware` trait. On each call to `before_model()`, it:

1. Lists entries in the skills directory via the backend's `ls()` method.
2. For each directory entry, reads the first 50 lines of `{dir}/SKILL.md`.
3. Parses the YAML frontmatter to extract `name` and `description`.
4. Builds an `<available_skills>` section and appends it to the system prompt.

The injected section looks like:

```text
<available_skills>
- **search**: Search the web for information (read `.skills/search/SKILL.md` for details)
- **testing**: Guidelines for writing tests (read `.skills/testing/SKILL.md` for details)
</available_skills>
```

The agent sees this index and can read the full SKILL.md file via the `read_file` tool when it needs the detailed instructions.

## Configuration

Skills are enabled by default. Configure via `DeepAgentOptions`:

```rust,ignore
use std::sync::Arc;
use synaptic::deep::{create_deep_agent, DeepAgentOptions};

let mut options = DeepAgentOptions::new(backend);
options.skills_dir = Some(".skills".to_string());  // default
options.enable_skills = true;                       // default
let agent = create_deep_agent(model, options)?;
```

To disable skills entirely, set `enable_skills = false`. To change the skills directory, set `skills_dir` to a different path within the backend.

## Example: Adding a Rust Refactoring Skill

Create the file `.skills/rust-refactoring/SKILL.md` in your workspace:

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

Once this file is present in the backend, the `SkillsMiddleware` will automatically discover it and include it in the system prompt index. The agent can then read the full file for detailed instructions when it encounters a refactoring task.

There is no programmatic skill registration API. All skills are filesystem-based, discovered at runtime by scanning the backend.

## More Examples

### Code Review Skill

A code review skill injects a structured checklist so the agent applies consistent review standards:

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

This turns the agent into a disciplined reviewer that categorizes findings by severity rather than giving unstructured feedback.

### TDD Workflow Skill

A TDD skill constrains the agent to follow a strict Red-Green-Refactor cycle:

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

This prevents the agent from jumping ahead to write implementation code before tests exist.

### API Design Conventions Skill

A conventions skill encodes team-wide API standards so every endpoint the agent creates follows the same patterns:

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

Any agent working on the API layer will automatically produce consistent endpoints without per-task reminders.

### Multi-Skill Cooperation

When multiple skills exist in the workspace, the agent sees all of them in the index and reads the relevant ones based on the current task. Consider this layout:

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

The `SkillsMiddleware` injects the full index into the system prompt:

```text
<available_skills>
- **code-review**: Structured code review checklist with severity levels (read `.skills/code-review/SKILL.md` for details)
- **tdd**: Enforce test-driven development workflow (read `.skills/tdd/SKILL.md` for details)
- **api-conventions**: Team API design standards for REST endpoints (read `.skills/api-conventions/SKILL.md` for details)
- **rust-refactoring**: Best practices for refactoring Rust code (read `.skills/rust-refactoring/SKILL.md` for details)
</available_skills>
```

The agent then selectively reads skills that match the task at hand:

- **"Add a new `/users` endpoint with tests"** — the agent reads `api-conventions` and `tdd`, then follows the TDD cycle while applying the URL and response format standards.
- **"Review this pull request"** — the agent reads `code-review` and produces findings with severity levels.
- **"Refactor the auth module"** — the agent reads `rust-refactoring` and `code-review` (to self-check the result).

Skills are composable: each one contributes a focused set of instructions, and the agent combines them as needed. This is more maintainable than a single monolithic system prompt.

## Best Practices

**Keep skills focused and concise.** Each skill should cover one topic. A 20–50 line SKILL.md is ideal. If a skill grows beyond 100 lines, consider splitting it.

**Use action-oriented language.** Write instructions as directives ("Run tests before committing", "Use kebab-case for URLs") rather than descriptions ("Tests should ideally be run").

**Format with Markdown structure.** Use headings, numbered lists, and bold text. The agent processes structured content more reliably than prose paragraphs.

**Name directories in kebab-case.** Use lowercase with hyphens: `code-review/`, `api-conventions/`, `rust-refactoring/`. Avoid spaces, underscores, or camelCase.

**Skills vs. system prompt.** Use skills for instructions that are reusable across tasks and discoverable by name. Use the system prompt directly for instructions that always apply to every interaction. If you find yourself copying the same instructions into multiple prompts, extract them into a skill.
