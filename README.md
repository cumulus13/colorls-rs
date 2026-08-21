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

## Install

### Cargo

```bash
$ cargo install colorls
```

### Build from source

Requires Rust 1.75+ (matches the toolchain used for the project's other
Rust tools). `cargo build` alone works — no native system libraries are
required (git status is invoked via `git` on `PATH` at runtime, not linked).

```sh
cd colorls-rs
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

---

Every build produces **two identical binaries**: `colorls` and `lls`. Same
code, same flags, same everything — `lls` is just a shorter, collision-free
name for the common case of an old Ruby `colorls`/`colorls.bat` already
sitting on `PATH` (as e.g. `where colorls` on Windows would show if you've
ever `gem install colorls`'d). `--help`/`--version` on each reports its own
name, not the other one's.

Copy `target/release/colorls` and/or `target/release/lls` (`.exe` on
Windows) anywhere on `PATH`. There is no installer and no required config —
it works with sane defaults out of the box. To alias it over the real
`ls`/`dir`, add to your shell profile:

```sh
alias ls='lls'
alias ll='lls -l'
alias la='lls -a'
alias lt='lls --tree'
```

On Windows (PowerShell profile):

```powershell
function ls { lls @args }
function ll { lls -l @args }
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
| `--gs`, `--git-status` | show per-entry git status column, plus a branch header above the listing |
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
| `-p`, `--paginate` | page output through `$PAGER` (or `less -R`), preserving color |

Run `colorls --help` for the full, always-up-to-date list.

**Nerd Font required for icons.** If your terminal font doesn't include
[Nerd Font](https://www.nerdfonts.com/) glyphs, icons render as boxes/`?`.
Either install a Nerd Font, or run with `--no-icons`.

## Piping and paging colored output

By default, `colorls`/`lls` disables color whenever stdout isn't a
terminal — same convention as `ls --color=auto`, `grep --color=auto`, etc.
So `lls -rt | more` showing no color is expected, not a bug: `more`
(especially the classic Windows `more.com`) doesn't understand ANSI color
codes at all, and even tools that do (like `less`) will show the raw
`^[[92m...` escape sequences as garbage text instead of color unless told
to interpret them.

Two ways to get colored output through a pipe:

**`-p` / `--paginate` (recommended)** — spawns `$PAGER` (or `less -R` if
`$PAGER` isn't set) itself, forces color on for it specifically, and pages
interactively:

```sh
lls -rt -p
```

If no usable pager is found on `PATH` (common on a bare Windows install
without `less.exe`), it prints a one-line warning and falls back to plain
output rather than risking unreadable escape-code garbage.

**Manual piping** — if you'd rather manage your own pager pipeline, force
color explicitly and make sure your pager is told to interpret raw control
characters:

```sh
lls -rt --color=always | less -R
```

Do **not** pipe `--color=always` output into `more` (or `less` without
`-R`) — both will show literal escape codes instead of color, which is
exactly what "chaos" looks like. `-p`/`--paginate` avoids this entirely by
using `less -R` as its own default.

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

This writes eight files (only the ones missing — existing files are never
overwritten):

| File | Purpose |
|---|---|
| `config.yaml` | default behaviour (theme, icons on/off, git status on/off, sort options, `--report`, tree depth) |
| `dark_colors.yaml` / `light_colors.yaml` | category → color name, e.g. `dir: bright_cyan` |
| `icons.yaml` | file extension → Nerd Font glyph |
| `filenames.yaml` | exact filename (e.g. `dockerfile`, `readme.md`) → glyph, takes priority over extension |
| `folders.yaml` | folder name (e.g. `node_modules`, `.git`) → glyph |
| `aliases.yaml` | file extension → color category (`source_code`, `image`, `document`, ...) |
| `extension_colors.yaml` | file extension → color name **directly**, highest priority (see below) |

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
dir: "#00FFFF"
document: FFFF00
```

### Coloring one specific extension without touching its whole category

`aliases.yaml` + `dark_colors.yaml`/`light_colors.yaml` color by *category*
(all "compressed" files share one color, all "source_code" files share
another). If you want a single extension to have its own color without
inventing a whole new category, use `extension_colors.yaml` instead — it's
checked first, before category resolution even runs:

```yaml
# ~/.config/colorls/extension_colors.yaml
zip: bright_red
jar: bright_green
bz2: bright_yellow
gz: yellow
log: bright_black
7z: "#FF5500"
rar: 00AAFF
```

This is exactly how the built-in defaults keep archive formats visually
distinct from each other out of the box (`.zip`/`.rar`/`.7z` red, `.gz`/
`.bz2`/`.xz` yellow, `.tar` magenta, `.jar` green) instead of every archive
type sharing the single "compressed" category color. Any extension not
listed here just falls back to its category color as usual.

### Hex colors

Anywhere a color name is accepted — `dark_colors.yaml`, `light_colors.yaml`,
and `extension_colors.yaml` — you can use a 24-bit hex color instead of a
named one:

```yaml
# ~/.config/colorls/extension_colors.yaml
zip: FF8800        # bare hex — recommended, see gotcha below
jar: "#00FF88"      # quoted hex also works
log: "#666"          # 3-digit shorthand, quoted (each digit doubled: 666 -> 666666)
```

**YAML gotcha:** an *unquoted* `#` starts a comment, so `zip: #FF0000`
silently parses as an empty value, not the color you meant — colorls will
warn you about exactly this (without crashing or discarding the rest of
the file) if it happens. Either quote the value (`zip: "#FF0000"`) or drop
the `#` entirely (`zip: FF0000`); both are treated identically.

Hex colors always render as exact true 24-bit color (`\e[38;2;r;g;bm`)
wherever the terminal honors 24-bit ANSI — which is effectively every
terminal still receiving updates in 2026. Unlike most tools that check the
`COLORTERM` environment variable and silently substitute the nearest of 16
named colors when it's absent, colorls does **not** do that downgrade for
explicitly-configured hex colors: `COLORTERM` is an unreliable signal in
practice (frequently missing over SSH, in tmux, or other passthrough
shells even when the terminal displaying the output supports truecolor
fine), and a color you explicitly typed as hex should render as exactly
that color, not a guess.

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
- **`-p`/`--paginate`** redirects all output through a spawned pager's
  stdin pipe instead of real stdout (see `util::init_output_writer` /
  `close_output` in `src/util.rs`), and upgrades `--color=auto` to
  `--color=always` for the duration — since the destination is no longer
  the terminal directly, plain TTY detection would otherwise (correctly,
  in isolation) strip color right before it reaches the pager.

---

## 👤 Author
        
[Hadi Cahyadi](mailto:cumulus13@gmail.com)
    

[![Buy Me a Coffee](https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png)](https://www.buymeacoffee.com/cumulus13)

[![Donate via Ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/cumulus13)
 
[Support me on Patreon](https://www.patreon.com/cumulus13)
