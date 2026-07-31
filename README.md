# OxiLine

> Time as a playhead — macOS native routine/day-management app.

A Rust core + Tauri v2 desktop shell, designed CLI-first for agents and humans.

| | |
|---|---|
| **Stack** | Rust core (oxiline-core, oxiline-cli) · Tauri v2 desktop shell (oxiline-app) |
| **License** | [MIT](LICENSE) |
| **Status** | Early development — design phase → [roadmap](doc/08-roadmap.md) |

## Design

The full design specification lives in [`doc/`](doc/00-README.md) — eight markdown documents walking through product vision, UX research, data model, architecture, CLI spec, design system, screens, and roadmap.

## Build

```bash
# Workspace at root
cargo build --release

# CLI only (no Tauri runtime)
cargo build -p oxiline-cli --release
```

## Workspace

```
crates/
├── oxiline-core/   # Domain logic, SQLite schema, CLI parser
├── oxiline-cli/    # CLI binary (oxiline)
└── oxiline-app/    # Tauri v2 desktop shell
    └── src-tauri/  # Backend for the desktop app
```

## Related

Part of the [project-oxi](https://github.com/project-oxi) ecosystem. See the [organization profile](https://github.com/project-oxi/.github/blob/main/profile/README.md) for the full product line.

## License

[MIT](LICENSE) — see [`doc/06-design-system.md`](doc/06-design-system.md) for the brand specs inherited from the project-oxi design system.
