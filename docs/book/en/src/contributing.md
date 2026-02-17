# Contributing

Thank you for your interest in contributing to Synaptic. This guide covers the workflow and standards for submitting changes.

## Getting Started

1. **Fork** the repository on GitHub.
2. **Clone** your fork locally:
   ```bash
   git clone https://github.com/<your-username>/synaptic.git
   cd synaptic
   ```
3. **Create a branch** for your changes:
   ```bash
   git checkout -b feature/my-change
   ```

## Development Workflow

Before submitting a pull request, make sure all checks pass locally.

### Run Tests

```bash
cargo test --workspace
```

All tests must pass. If you are adding a new feature, add tests for it in the appropriate `tests/` directory within the crate.

### Run Clippy

```bash
cargo clippy --workspace
```

Fix any warnings. Clippy enforces idiomatic Rust patterns and catches common mistakes.

### Check Formatting

```bash
cargo fmt --all -- --check
```

If this fails, run `cargo fmt --all` to auto-format and commit the result.

### Build the Workspace

```bash
cargo build --workspace
```

Ensure everything compiles cleanly.

## Submitting a Pull Request

1. Push your branch to your fork.
2. Open a pull request against the `main` branch.
3. Provide a clear description of what your change does and why.
4. Reference any related issues.

## Guidelines

### Code

- Follow existing patterns in the codebase. Each crate has a consistent structure with `src/` for implementation and `tests/` for integration tests.
- All traits are async via `#[async_trait]`. Tests use `#[tokio::test]`.
- Use `Arc<RwLock<_>>` for shared registries and `Arc<tokio::sync::Mutex<_>>` for callbacks and memory.
- Prefer factory methods over struct literals for core types (e.g., `Message::human()`, `ChatRequest::new()`).

### Documentation

- When adding a new feature or changing a public API, update the corresponding documentation page in `docs/book/en/src/`.
- How-to guides go in `how-to/`, conceptual explanations in `concepts/`, and step-by-step walkthroughs in `tutorials/`.
- If your change affects the project overview, update the README at the repository root.

### Tests

- Each crate has a `tests/` directory with integration-style tests in separate files.
- Use `ScriptedChatModel` or `FakeBackend` for testing model interactions without real API calls.
- Use `FakeEmbeddings` for testing embedding-dependent features.

### Commit Messages

- Write clear, concise commit messages that explain the "why" behind the change.
- Use conventional prefixes when appropriate: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`.

## Project Structure

The workspace contains 17 library crates in `crates/` plus example binaries in `examples/`. See [Architecture Overview](architecture-overview.md) for a detailed breakdown of the crate layers and dependency graph.

## Questions

If you are unsure about an approach, open an issue to discuss before writing code. This helps avoid wasted effort and keeps changes aligned with the project direction.
