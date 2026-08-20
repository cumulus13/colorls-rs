# colorls (Rust)

A fast, production-ready Rust rewrite of [colorls](https://github.com/athityakumar/colorls)
(the Ruby gem): a beautified `ls` with Nerd Font icons, per-type colors,
git status, and a tree view. Same *purpose* as upstream colorls — not a
line-for-line port — rebuilt from scratch as a single static binary with a
YAML config-override system that plays the same role as upstream's
`~/.config/colorls/*.yaml` files.

Cross-platform: Linux, macOS, and Windows. No Ruby, no gems, no runtime
dependencies beyond an optional `git` binary on `PATH` for `--gs`.

## Supported platforms

Every tagged release (`vX.Y.Z`) builds and publishes prebuilt binaries for:

| OS | Targets |
|---|---|
| Linux (glibc) | `x86_64`, `aarch64`, `armv7` (armhf, e.g. Raspberry Pi), `i686` |
| Linux (musl, static) | `x86_64`, `aarch64`, `armv7` |
| Android / Termux | `aarch64` (arm64, most phones), `armv7` (armv7l, older 32-bit devices), `i686`, `x86_64` (emulators) |
| macOS | `x86_64` (Intel), `aarch64` (Apple Silicon) |
| Windows | `x86_64`, `i686`, `aarch64` (Windows-on-ARM, e.g. Surface Pro X) |

See [Releases](https://github.com/cumulus13/colorls-rs/releases) for
prebuilt archives, or build from source for anything not listed (any target
`rustc`/`cross` supports should work — the code has no target-specific
`unsafe`/FFI beyond standard library and `users`/`colored`, both of which
are cross-platform).

### Termux (Android)

Termux runs standard Android NDK (`*-linux-android`) binaries directly,
since Termux itself targets bionic libc the same way an NDK build does.
Download the `aarch64-linux-android` archive (or `armv7-linux-androideabi`
for older 32-bit devices) from Releases, then:

```sh
tar xzf colorls-*-android*.tar.gz
cd colorls-*-android*
chmod +x colorls
mv colorls $PREFIX/bin/
colorls --init-config
```

Owner/group columns in `-l` show numeric uid/gid on Android/Termux instead
of resolved names (bionic doesn't expose the conventional multi-user
passwd/group database glibc/musl provide, so there's no name to resolve).
`--init-config`, git status (`--gs`, if `git` is installed via `pkg install
git`), and everything else behaves identically to the Linux build.

## Build

Requires Rust 1.75+ (matches the toolchain used for the project's other
Rust tools). `cargo build` alone works — no native system libraries are
required (git status is invoked via `git` on `PATH` at runtime, not linked).

```sh
cargo build --release
# binary at target/release/colorls (colorls.exe on Windows)
```

Cross-compiling for another target locally (with the target installed via
rustup, or via [`cross`](https://github.com/cross-rs/cross) for targets
that need a foreign C toolchain, e.g. Android or musl/ARM):

```sh
rustup target add aarch64-unknown-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu

# or, for targets needing a cross-toolchain (Android, musl, ARM, ...):
cargo install cross --git https://github.com/cross-rs/cross
cross build --release --target aarch64-linux-android
```

### CI / releases

- `.github/workflows/ci.yml` — runs `fmt`, `clippy -D warnings`, `build`,
  and `test` on Linux/macOS/Windows for every push and PR.
- `.github/workflows/release.yml` — on every `vX.Y.Z` tag push, builds the
  full platform matrix above (native `cargo` for Linux x86_64/macOS/Windows,
  [`cross`](https://github.com/cross-rs/cross) via Docker for every other
  target, including all four Android/Termux ABIs), packages each as a
  checksummed `.tar.gz`/`.zip`, and publishes them to a GitHub Release.
  Can also be triggered manually (`workflow_dispatch`) against an existing
  tag, to re-run a single failed platform leg without cutting a new tag.

## Install

Copy `target/release/colorls` (or `colorls.exe`) anywhere on `PATH`. There
is no installer and no required config — it works with sane defaults out of
the box. To alias it over the real `ls`/`dir`, add to your shell profile:

```sh
alias ls='colorls'
alias ll='colorls -l'
alias la='colorls -a'
alias lt='colorls --tree'
```

On Windows (PowerShell profile):

```powershell
function ls { colorls @args }
function ll { colorls -l @args }
```

## Usage

```
colorls [OPTIONS] [PATHS]...
```

| Flag | Meaning |
|---|---|
| `-a`, `--all` | show hidden entries, including `.` and `..` |
| `-A`, `--almost-all` | show hidden entries, without `.`/`..` |
| `-l`, `--long` | long listing: permissions, owner, size, date |
| `-1`, `--oneline` | one entry per line |
| `--tree[=N]` | tree view, default depth 3 |
| `--gs`, `--git-status` | show per-entry git status column |
| `--sd`, `--group-directories-first` | directories before files |
| `--sf`, `--sort-files` | files before directories |
| `-t` | sort by modification time, newest first |
| `-S`, `--sort-size` | sort by size, largest first |
| `-X`, `--sort-extension` | sort by extension |
| `-r`, `--reverse` | reverse the active sort order |
| `-R`, `--recursive` | recurse into sub-directories (flat, `ls -R` style) |
| `--light` / `--dark` | color theme |
| `--report` | print a directory/file/size summary after listing |
| `--no-icons` / `--icons` | disable/force-enable Nerd Font icons |
| `--color <auto|always|never>` | control ANSI color output |
| `--config <PATH>` | use a specific config dir (or its `config.yaml`) |
| `--init-config` | write default config files, without clobbering existing ones |
| `--print-config-dir` | print the resolved config directory and exit |
| `-q`, `--quiet` | suppress non-fatal warnings |
| `-v`, `-vv` | increase verbosity |

Run `colorls --help` for the full, always-up-to-date list.

**Nerd Font required for icons.** If your terminal font doesn't include
[Nerd Font](https://www.nerdfonts.com/) glyphs, icons render as boxes/`?`.
Either install a Nerd Font, or run with `--no-icons`.

## Configuration

`colorls` looks for a config directory in this order:

1. `--config <path>` (a directory, or a `config.yaml` file inside one)
2. `$COLORLS_CONFIG` environment variable
3. Platform default:
   - Linux: `~/.config/colorls`
   - macOS: `~/Library/Application Support/colorls`
   - Windows: `%APPDATA%\colorls`
   - Android/Termux: `$HOME/.config/colorls` (Termux's own `$HOME`, since
     the platform's usual directory-resolution APIs need JNI and aren't
     available to a plain native binary)

Run once to scaffold it:

```sh
colorls --init-config
```

This writes seven files (only the ones missing — existing files are never
overwritten):

| File | Purpose |
|---|---|
| `config.yaml` | default behaviour (theme, icons on/off, git status on/off, sort options, `--report`, tree depth) |
| `dark_colors.yaml` / `light_colors.yaml` | category → color name, e.g. `dir: bright_cyan` |
| `icons.yaml` | file extension → Nerd Font glyph |
| `filenames.yaml` | exact filename (e.g. `dockerfile`, `readme.md`) → glyph, takes priority over extension |
| `folders.yaml` | folder name (e.g. `node_modules`, `.git`) → glyph |
| `aliases.yaml` | file extension → color category (`source_code`, `image`, `document`, ...) |

Any key you add to these files overrides the built-in default for that key
only — you don't need to redefine everything, just the entries you want to
change. Example, to make `.rs` files render in bright red instead of the
default source-code color:

```yaml
# ~/.config/colorls/aliases.yaml
rs: dead_link   # (or add a new category + matching color entry)
```

or simpler, just recolor an entire category in `dark_colors.yaml`:

```yaml
source_code: bright_red
```

`config.yaml` example:

```yaml
theme: dark
icons: true
git_status: true
group_directories_first: true
sort_files_first: false
report: false
tree_depth: 4
long: false
all: false
```

Every `config.yaml` key has a matching CLI flag; the CLI flag always wins
when both are given, so `config.yaml` only sets your personal defaults.

## Design notes

- **Git status** shells out to `git status --porcelain=v1 --ignored` rather
  than linking `libgit2`; this keeps the build dependency-free and avoids
  the native-toolchain headaches `libgit2`-based crates cause when
  cross-compiling for Windows. `--gs` is silently disabled (with a one-line
  warning, suppressible with `-q`) when `git` isn't on `PATH` or the target
  isn't inside a work tree.
- **Broken pipes** (`colorls | head`, output redirected then closed early)
  exit cleanly instead of panicking — all output goes through a small
  writer that treats `BrokenPipe` as a normal, silent exit(0).
- **Color** is force-synced with `colored::control::set_override` so
  `--color=always`/`--color=never` are honored even when stdout isn't a
  TTY (piped into `head`, redirected to a file, etc.) instead of being
  silently overridden by the crate's own auto-detection.
- **Owner/group** columns are populated on Unix via the `users` crate and
  show `-` on Windows, which has no equivalent POSIX uid/gid concept.

---

## 👤 Author
        
[Hadi Cahyadi](mailto:cumulus13@gmail.com)
    

[![Buy Me a Coffee](https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png)](https://www.buymeacoffee.com/cumulus13)

[![Donate via Ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/cumulus13)
 
[Support me on Patreon](https://www.patreon.com/cumulus13)