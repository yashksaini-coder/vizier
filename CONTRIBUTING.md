# Contributing to Rustlens

Thank you for your interest in contributing to Rustlens! 🎉

## Getting Started

1. **Fork the repository** and clone it locally
2. **Install Rust** (1.75.0 or later): https://rustup.rs/
3. **Set up development tools**:
   ```bash
   make dev-setup
   ```

## Development Workflow

### Before Submitting

Run the full check suite:

```bash
make check
```

This runs:
- `cargo fmt --check` — Code formatting
- `cargo clippy` — Linting
- `cargo check` — Type checking

### Running Tests

```bash
make test
```

### Building

```bash
make build      # Debug build
make release    # Optimized release build
```

## Pull Request Guidelines

### Commit Messages

We follow [Conventional Commits](https://www.conventionalcommits.org/). Format:

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Types:**
- `feat` — New feature
- `fix` — Bug fix
- `docs` — Documentation only
- `style` — Formatting, missing semicolons, etc.
- `refactor` — Code change that neither fixes a bug nor adds a feature
- `perf` — Performance improvement
- `test` — Adding or fixing tests
- `build` — Build system or external dependencies
- `ci` — CI configuration
- `chore` — Other changes that don't modify src or test files

**Examples:**
```
feat(search): add fuzzy matching support
fix(parser): handle empty function bodies
docs: update installation instructions
refactor!: rename App to RustlensApp  # Breaking change
```

### PR Title

Your PR title should also follow Conventional Commits format, as it will be used in the changelog.

### Breaking Changes

For breaking changes:
1. Add `!` after the type/scope: `refactor!: change API`
2. Add `BREAKING CHANGE:` in the commit body explaining the change

## Code Style

- Run `cargo fmt` before committing
- Follow Rust naming conventions
- Add documentation for public APIs
- Keep functions focused and reasonably sized

## Architecture

```
src/
├── main.rs           # Entry point
├── app/              # Application state management
├── analyzer/         # Code parsing (syn-based)
├── config/           # Configuration system
├── error/            # Error types
├── ui/               # TUI components (ratatui)
└── utils/            # Helper utilities
```

### Key Modules

- **analyzer/parser.rs** — Parses Rust files using `syn`
- **ui/app.rs** — Main TUI widget
- **ui/inspector.rs** — Detail panel for items
- **ui/theme/** — Color themes

## Questions?

Feel free to open an issue for discussion before starting work on large changes.
