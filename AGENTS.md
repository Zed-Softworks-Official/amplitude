# AGENTS.md — Amplitude

Amplitude is a cross-platform desktop audio mixer built with **Tauri v2** (Rust backend) and **React 19 + TypeScript + Vite** (frontend). Package manager is **pnpm**.

---

## Build / Dev / Lint Commands

### Frontend (from repo root)

| Task | Command |
|------|---------|
| Dev server (frontend only) | `pnpm dev` |
| Full Tauri dev (frontend + Rust) | `pnpm tauri dev` |
| Production build | `pnpm build` (runs `tsc && vite build`) |
| Tauri production build | `pnpm tauri build` |
| Format all files | `pnpm format` (Biome) |
| Lint all files | `pnpm lint` (Biome) |
| Type-check only | `npx tsc --noEmit` |

### Rust backend (from `src-tauri/`)

| Task | Command |
|------|---------|
| Check compilation | `cargo check` |
| Build | `cargo build` |
| Lint | `cargo clippy` |
| Format | `cargo fmt` |
| Format check | `cargo fmt --check` |

### Tests

There is **no test framework** configured for the frontend. No Jest, Vitest, or similar runner exists. There are no frontend test files.

For Rust tests, run from `src-tauri/`:
```
cargo test                    # run all tests
cargo test <test_name>        # run a single test by name
cargo test --lib              # run only library tests
```

---

## Project Structure

```
src/                          # React/TypeScript frontend
  components/
    mixer/                    # Application components (channel-strip, meter, etc.)
    ui/                       # shadcn/radix UI primitives (do not edit by hand)
  hooks/                      # Custom React hooks
  lib/
    tauri-api.ts              # Typed wrappers around Tauri invoke() calls
    types.ts                  # Shared TypeScript interfaces
    utils.ts                  # cn() utility (clsx + tailwind-merge)
  App.tsx                     # Root component
  main.tsx                    # React entry point
src-tauri/                    # Rust backend (Tauri v2)
  src/
    audio/                    # Audio processing (backend, node, sink)
    backend/                  # Platform backends (CoreAudio, PipeWire)
    commands/                 # Tauri command handlers (bus, channel)
    core/                     # Engine, config, state management
    lib.rs / main.rs          # Entry points
```

---

## Code Style — TypeScript / React

### Formatting (enforced by Biome)

- **4-space indentation** (spaces, not tabs)
- **Single quotes** for strings
- **No semicolons** (Biome "asNeeded" = omit where possible)
- **80-character line width**
- **Trailing commas** in multi-line structures

Run `pnpm format` before committing. Biome auto-organizes imports.

### Imports

- Use the **`~/` path alias** for all imports from `src/` (e.g., `import { cn } from '~/lib/utils'`). Never use `../` to escape a directory — only use relative `./` for siblings within the same folder.
- Use **`import type`** for type-only imports: `import type { Channel } from '~/lib/types'`.
- Import order (enforced by Biome): third-party packages first, then `~/` aliases, then relative `./` imports.

### Naming Conventions

| Element | Convention | Example |
|---------|-----------|---------|
| Files | kebab-case | `channel-strip.tsx`, `tauri-api.ts` |
| Components | PascalCase named export | `export function ChannelStrip()` |
| Hooks | camelCase with `use` prefix | `useSubscription` |
| Props interfaces | PascalCase + `Props` suffix | `ChannelStripProps` |
| Constants | UPPER_SNAKE_CASE | `SEGMENT_COUNT`, `INPUT_DEVICES` |
| Variables / functions | camelCase | `handleVolumeChange` |

### Component Patterns

- Use **function declarations** (not arrow functions) for all exported components.
- Use **named exports** (no default exports), except `App.tsx`.
- Define **props via `interface`** directly above the component.
- Internal sub-components are plain `function` declarations with inline typed props (not exported).
- Section dividers in longer files:
  ```ts
  // ---------------------------------------------------------------------------
  // Volume conversion helpers
  // ---------------------------------------------------------------------------
  ```

### Types

- TypeScript **strict mode** is enabled (`strict: true`, `noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch`).
- Prefer `interface` for object shapes and props; use `type` for unions and aliases.
- Use `as const` for readonly constant arrays/objects.
- Shared types live in `src/lib/types.ts`.

### Error Handling

- Fire-and-forget Tauri commands: `.catch(console.error)`.
- User-facing errors: catch with `(err: unknown)`, narrow via `err instanceof Error` / `typeof err === 'string'`, then display to user.
- Async hook cleanup: catch listen failures with `.catch((err: unknown) => console.error(...))`.

### Styling

- **Tailwind CSS v4** with CSS custom properties (oklch colors) defined in `App.css`.
- **shadcn/ui** components live in `components/ui/` — do not manually edit these; use `npx shadcn` to add/update.
- Use the `cn()` utility from `~/lib/utils` for conditional class merging.
- `class-variance-authority` (`cva`) for component variants.
- App is **dark mode by default**.

### State Management

- Local state with `useState` / `useEffect`; state is lifted to `MixerApp` and passed as props.
- Backend events consumed via the custom `useSubscription` hook (wraps Tauri `listen()`).
- `@tanstack/react-query` is available but not yet in use.

---

## Code Style — Rust

### Formatting (enforced by rustfmt)

- **Block indent style**, **80-character max width** (see `src-tauri/rustfmt.toml`).
- Run `cargo fmt` from `src-tauri/` before committing.
- Run `cargo clippy` for lints.

### Patterns

- Tauri commands are exposed via `tauri::generate_handler![]` in `lib.rs`.
- Shared state is `Mutex<AudioEngine>` managed by Tauri's state system.
- Platform-specific code: `backend/coreaudio.rs` (macOS), `backend/pipewire.rs` (Linux).
- Serialization via `serde` with `#[derive(Serialize, Deserialize)]`.
- Config persistence: TOML format via the `toml` crate, stored in platform dirs.
- UUIDs generated with `uuid::Uuid::new_v4()`.

---

## Key Dependencies

| Frontend | Purpose |
|----------|---------|
| `react` 19 | UI framework |
| `@tauri-apps/api` | Tauri IPC bridge |
| `radix-ui` / `shadcn` | UI component primitives |
| `@dnd-kit/*` | Drag-and-drop for channel reordering |
| `tailwindcss` v4 | Styling |
| `@biomejs/biome` | Linting + formatting |

| Backend (Rust) | Purpose |
|-----------------|---------|
| `tauri` 2.x | Desktop app framework |
| `serde` / `serde_json` | Serialization |
| `coreaudio-rs` | macOS audio backend |
| `pipewire` | Linux audio backend |
