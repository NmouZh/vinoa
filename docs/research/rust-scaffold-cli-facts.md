# Load-Bearing Facts for a Rust Project-Scaffolding CLI (2026)

**Research date:** 2026-09-25.
**Scope:** exact current versions, APIs, and conventions for building a project-scaffolding CLI in Rust.
**Method / evidence rules:** every claim below is traced to a primary source — the crates.io API, a
published `.crate` artifact (the manifest Cargo actually uploaded), docs.rs rustdoc, or an official
book/repo. Blog posts are not used as sources.

**Local toolchain at time of research:** `rustc 1.98.1 (48a229cea 2026-09-01)`, `cargo 1.98.1 (797e8a9bc 2026-08-05)`.

> **Environment caveat (affects two citations).** In this workspace `cargo-generate.github.io` resolves to
> `127.0.0.1` and `opensource.axo.dev` / `crate-ci.github.io` / `raw.githubusercontent.com` were
> intermittently unreachable. Where a documentation site could not be fetched, the claim is instead
> sourced from the **crate artifact itself** (`cargo-generate-0.25.0.crate` ships its own `guide/`
> mdBook sources and `src/args.rs`), which is a stronger primary source than the rendered site.

---

## 0. TL;DR — load-bearing decisions

| Decision | Recommendation | Why |
|---|---|---|
| CLI parsing | `clap` **4.6.7** with `features = ["derive"]` | De-facto standard; MSRV 1.85; `default_value_t` + `ValueEnum` + `ArgAction::SetTrue` cover the interactive/flags duality |
| Interactive prompts | `inquire` **0.9.4** (`Text`/`Select`/`MultiSelect`/`Confirm` + `with_validator`) | Maintained through Feb 2026, far richer built-ins than `dialoguer` |
| Template embedding | **`rust-embed` 8.12.0** (with `debug-embed` in dev, `include-exclude` for filters) | Actively maintained (Jul 2026); `include_dir` 0.7.4 has had no release since Jun 2024 |
| Template rendering | **`minijinja` 2.24.0** (`render_str` / `render_named_str`) for simple var + `{% if %}` substitution | Runtime templates, tiny dep, actively maintained; `askama` is compile-time-only and does not fit "copy a tree" |
| Edition / MSRV | `edition = "2024"`, `rust-version = "1.85"`, `resolver = "3"` implied | Edition 2024 stabilized in Rust 1.85.0 (Feb 2025) |
| Distribution | `cargo-dist` **0.32.0** (binary is `dist`) for packaging, `cargo-release` **1.1.6** for versioning | Both current and released within the last ~5 months |

---

## 1. `clap` v4 — versions, derive API, interactive-by-default-but-flaggable

### Versions (crates.io API)

| Crate | Current version | Published |
|---|---|---|
| `clap` | **4.6.7** | 2026-09-14 |
| `clap_derive` | **4.6.7** | 2026-09-14 |
| `clap_builder` | **4.6.7** | 2026-09-14 |
| `clap_complete` | **4.6.11** | 2026-09-15 |
| `clap_complete_nushell` | **4.6.2** | 2026-08-11 |

`clap` 4.6.7 declares `rust-version = "1.85"` and `edition = "2024"` in its published manifest.
Default features are `["std", "color", "help", "usage", "error-context", "suggestions"]`; the
`derive` feature is **not** default and must be enabled.
Sources: <https://crates.io/api/v1/crates/clap>, <https://crates.io/crates/clap>,
<https://docs.rs/clap/4.6.7/clap/>.

### Derive API that matters for a scaffolder

From the official derive tutorial and derive reference:

- **Subcommands:** derive with `#[derive(Subcommand)]` and attach via `#[command(subcommand)]` on the
  field. `T` implies a *required* subcommand; **`Option<T>` makes the subcommand optional**.
  `#[command(propagate_version = true)]` makes `--version` available in every subcommand.
- **Flags:** a `bool` field implies `.action(ArgAction::SetTrue)` — no explicit attribute needed.
- **Optional values:** `Option<T>` implies `.action(ArgAction::Set).required(false)`.
  `Option<Option<T>>` implies `.num_args(0..=1)`, i.e. an optional *value* on an optional argument.
- **Defaults:** `#[arg(default_value_t = 2020)]` for a typed literal (implies the arg is not
  required); `default_value` for a string form.
- **Value enums:** `#[derive(ValueEnum)]` on the enum plus `#[arg(value_enum)]` on the field. clap
  then emits a graceful error listing the valid values. Per-variant tuning uses `#[value(...)]`.
- **Globals / delegation:** `#[arg(global = true)]` for a flag visible in all subcommands, and
  `#[command(flatten)]` to share an `Args` struct across subcommands.

Sources: <https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html>,
<https://docs.rs/clap/latest/clap/_derive/index.html>.

### Making a subcommand interactive-by-default but fully flaggable for CI

clap itself has no "prompt the user" concept — it only parses. The established pattern is
**model the choices as flags with `Option<T>`/`default_value_t` and let your own code decide whether
to prompt**; this is exactly what `cargo-generate` does (`--name`, `--define`, `--silent`).

Concrete clap affordances that make this work:

- Omitting a flag → `Option<T>` is `None` → **you** prompt interactively.
- Passing the flag → `Some(value)` → skip the prompt. This is the CI path.
- `#[arg(default_value_t = ...)]` for non-optional choices with a sensible default.
- `#[arg(short, long, action)]` on a `bool` → `--yes/-y` to skip confirmation (implies
  `ArgAction::SetTrue`).
- `#[arg(long, global = true)]` for `--yes`/`--force`/`--verbose` usable under every subcommand.
- `#[derive(ValueEnum)]` + `#[arg(value_enum)]` so `--license mit` is validated at parse time.
- `#[command(arg_required_else_help(true))]` (used by cargo-generate) to print help instead of
  prompting when invoked with nothing.

**Cleanest CI contract:** require an explicit `--yes` (or `--silent`) for the non-interactive path and
hard-fail if a required value is missing rather than silently prompting — `cargo-generate --silent`
does exactly this ("If a value is missing the project generation will fail").
Sources: <https://docs.rs/clap/latest/clap/_derive/index.html>,
`cargo-generate-0.25.0/src/args.rs` (see <https://crates.io/crates/cargo-generate>).

**Do not build the interactive layer on clap.** Use `clap` for parsing and `inquire` (or
`dialoguer`) for prompts; both are in the dependency tree of the major precedent,
`cargo-generate`, which uses `clap ~4.6` **and** `dialoguer ~0.12`.

---

## 2. `inquire` — version, maintenance status, prompt types, alternatives

### Version and maintenance status

| Fact | Value |
|---|---|
| Current version | **0.9.4** |
| Published | 2026-02-24 |
| Downloads (all time) | 21,045,358 |
| Declared MSRV | `rust-version = "1.80.0"` |
| License | MIT |
| Repo | <https://github.com/mikaelmello/inquire> |

**Maintenance evidence (all from primary sources):**

- Release cadence is *alive but modest*: v0.9.0 and v0.9.1 on 2025-09-16, v0.9.2 on 2026-01-17,
  v0.9.3 on 2026-02-06, **v0.9.4 on 2026-02-24** (GitHub Releases API).
- Repo state: 2,633 stars, 89 open issues, **not archived**; default-branch `pushed_at` =
  **2026-03-02**; the latest commit on the default branch is the `chore: release v0.9.4` commit
  (2026-02-24). So there have been no non-release commits on `main` since roughly March 2026 —
  i.e. **maintained, but low activity in the last ~6 months**.
- No deprecation notice; crates.io `updated_at` matches the last release.

**Verdict:** still the recommended choice for a Rust scaffolder today on *feature* grounds, but note
the activity gap; `dialoguer` has a more recent commit.

Sources: <https://crates.io/api/v1/crates/inquire>, <https://crates.io/crates/inquire>,
<https://docs.rs/inquire/latest/inquire/>, <https://github.com/mikaelmello/inquire>,
<https://api.github.com/repos/mikaelmello/inquire/releases>.

### Prompt types suitable for a scaffolder

Verified against the crate's rustdoc and its published `src/`:

| Prompt | Purpose | Gate |
|---|---|---|
| `Text` | free text (project name, package name) | default |
| `Select` | pick one from a list (license, edition, framework) | default |
| `MultiSelect` | pick N (features, integrations) | default |
| `Confirm` | yes/no (proceed? overwrite?) | default |
| `CustomType` | parse text into a custom type (numbers, UUID) | default |
| `Password` | secret text | default |
| `Editor` | open `$EDITOR` for long input | `editor` feature |
| `DateSelect` | calendar date picker | `date` feature (`chrono`) |

Prompt source modules present in 0.9.4: `confirm`, `custom_type`, `dateselect`, `editor`,
`multiselect`, `password`, `select`, `text`.
Source: <https://docs.rs/inquire/latest/inquire/>, `inquire-0.9.4/src/prompts/`.

**Builder surface (verified in the published source):**

- `Text::new(msg)`, `.with_default("...")`, `.with_help_message(...)`, `.with_placeholder(...)`,
  **`.with_validator(v)`**, `.with_validators(&[Box<dyn StringValidator>])`.
- `Select::new(msg, options)`, `.with_help_message(...)`, `.with_page_size(n)` — **note: `Select`
  has no `with_default` builder in 0.9.4**; preselect by ordering the options.
- `MultiSelect::new(msg, options)`, `.with_default(&[usize])`, `.with_page_size(n)`.
- `Confirm::new(msg)`, `.with_default(bool)`, `.with_help_message(...)`.

**Validation — important correction to a common assumption:** in 0.9.4 the `validator` module
exports only **four** ready-made validators — `ValueRequiredValidator`, `MaxLengthValidator`,
`MinLengthValidator`, `ExactLengthValidator` — plus the `with_validator` free-function form
(`Fn(&str) -> Result<Validation, CustomUserError>`, used for custom rules such as crate-name
validity). A blanket "inquire has built-in Email/NotEmpty validators" claim is **not** true for
0.9.4. Always validate the chosen project name yourself.
Sources: `inquire-0.9.4/src/validator.rs`, <https://docs.rs/inquire/latest/inquire/validator/>.

**Backends / features:** `default = ["macros", "crossterm", "one-liners", "fuzzy"]`; optional
`console ^0.16` and `termion ^4.0` backends exist as optional deps. MSRV 1.80.0.
Source: `inquire-0.9.4/Cargo.toml`.

### Alternatives — evidence, not vibes

| Crate | Latest | Published | Repo activity | Downloads |
|---|---|---|---|---|
| `inquire` | **0.9.4** | 2026-02-24 | last branch push 2026-03-02 | 21.0M |
| `dialoguer` | **0.12.0** | 2025-08-23 (crates.io) | **last commit 2026-09-21** (active) | 84.0M |
| `promkit` | **0.17.0** | 2026-09-14 | niche (real-time prompt toolkit) | 109k |
| `requestty` | **0.6.3** | 2025-12-02 | low downloads | 414k |
| `promptuity` | **0.0.5** | 2024-01-14 | **stale; pre-1.0, 15.6k downloads** | 15.6k |

- **`promptuity` does exist** on crates.io, but at 0.0.5 from January 2024 with ~15.6k lifetime
  downloads it is not a serious 2026 choice. Source: <https://crates.io/crates/promptuity>.
- **`dialoguer` 0.12.0** is the most-downloaded option and its repo is the most recently active
  (commit 2026-09-21, 1,614 stars, 97 open issues). It is the choice `cargo-generate` itself makes.
- **Recommendation:** `inquire` if you want the richer prompt set (fuzzy `Select`, `MultiSelect`,
  `CustomType`, autocompletion) out of the box; `dialoguer` if you weight commit recency above
  features and want the same stack as `cargo-generate`. Both are MIT and actively usable.

Sources: <https://crates.io/api/v1/crates/dialoguer>, <https://crates.io/api/v1/crates/promptuity>,
<https://crates.io/api/v1/crates/promkit>, <https://crates.io/api/v1/crates/requestty>,
<https://github.com/console-rs/dialoguer>, `cargo-generate-0.25.0/Cargo.toml`.

---

## 3. Template embedding and rendering

### 3a. Shipping a template tree inside one binary

| Approach | Version | Published | Maintained? | Recursive tree? |
|---|---|---|---|---|
| `include_str!`/`include_bytes!` (std) | — | — | std | **No** — one file per invocation |
| `include_dir` | **0.7.4** | 2024-06-17 | **No release in >2 years**; last commit 2024-06-17 | Yes |
| `rust-embed` | **8.12.0** | 2026-07-08 | Yes (release Jul 2026) | Yes |

**`include_str!` / `include_bytes!`** embed exactly one file each and return `&'static str` /
`&'static [u8]`. There is no directory walk and no path-preserving tree, so a template tree requires
one macro call per file (typically generated by a `build.rs`). They are the zero-dependency floor,
but they do not model a tree.
Source: <https://doc.rust-lang.org/std/macro.include_str.html>,
<https://doc.rust-lang.org/std/macro.include_bytes.html>.

**`include_dir` 0.7.4** — "An extension to the `include_str!()` and `include_bytes!()` macro for
embedding an entire directory tree into your binary."
- `static PROJECT_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets");`
- `get_file(path)`, `contents_utf8()`, `dirs()`, `files()`, `entries()`.
- Feature `glob` adds `find(pattern)` for `**/*.rs`-style filtering; `metadata` adds mtime;
  `nightly` enables `track_path`/`proc_macro_tracked_env` for better incremental caching.
- **Warning from its own docs:** the macro "expands to a fairly large amount of code … including a
  large number or files or files which are particularly big may cause the compiler to use large
  amounts of RAM or spend a long time parsing your crate." Prefer `$CARGO_MANIFEST_DIR` over relative
  paths.
- **Maintenance risk:** last crates.io release 0.7.4 (2024-06-17), last repo commit 2024-06-17
  (395 stars, 37 open issues). MSRV 1.64.
Sources: <https://docs.rs/include_dir/latest/include_dir/>, <https://crates.io/crates/include_dir>,
<https://github.com/Michael-F-Bryan/include_dir>.

**`rust-embed` 8.12.0** — `#[derive(Embed)] #[folder = "..."] struct Assets;`
- `Assets::get(path) -> Option<EmbeddedFile>`, `Assets::iter()`.
- **Critical behavioural difference vs `include_dir`:** in **debug** builds the bytes are *not*
  embedded by default — they are read from the filesystem (resolved relative to where the binary is
  run from in debug, vs `Cargo.toml` in release). Enable the **`debug-embed`** feature to force
  embedding in debug too; enable **`compression`** (via `include-flate`) to shrink the binary.
- `include-exclude` gives `#[include = "*.txt"]` / `#[exclude = "*.jpg"]` filters (globset;
  `exclude` wins); `interpolate-folder-path` allows `#[folder = "$CARGO_MANIFEST_DIR/foo"]`;
  `deterministic-timestamps` zeroes timestamps for reproducible builds; `prefix = "..."` prefixes
  all paths.
- MSRV 1.80.0. Note the crate self-describes as `[badges.maintenance] status = "passively-maintained"`
  in its manifest, yet it published 8.12.0 in July 2026.
Sources: <https://docs.rs/rust-embed/latest/rust_embed/>, <https://crates.io/crates/rust-embed>,
`rust-embed-8.12.0/Cargo.toml` + `README.md`.

> **Recommendation:** **`rust-embed` 8.12.0** for embedding the template tree. It is the only
> maintained tree-embedding option, and its debug-vs-release behaviour is actually a *feature* for a
> scaffolder: during development you edit template files on disk with no rebuild; in release the
> templates are baked in. Add `debug-embed` in CI so tests exercise the embedded path, and
> `include-exclude` to keep `.DS_Store`/`target/`-style junk out. Use bare `include_str!` only if
> you generate the per-file calls in `build.rs` and want zero dependencies.

### 3b. Rendering — current versions, maintenance, and fit

| Engine | Latest | Published | Model | MSRV | Fit for "copy a tree, substitute a few vars" |
|---|---|---|---|---|---|
| `minijinja` | **2.24.0** (3.0.0-alpha.2 exists) | 2026-08-12 (2.24.0) | runtime | 1.70 | **Best fit** |
| `handlebars` | **6.4.4** | 2026-08-12 | runtime | 1.85 | Strong, heavier |
| `tera` | **2.4.0** | 2026-09-11 | runtime | n/a | Good, Jinja2/Django-like |
| `liquid` | **0.26.11** | 2025-02-04 | runtime | 1.83 | What cargo-generate uses |
| `askama` | **0.16.1** | 2026-09-04 | **compile-time** | 1.88 | **Poor fit** |

- **`minijinja` 2.24.0** — `Environment::render_str(source, ctx)` and `render_named_str(name, source,
  ctx)` render **arbitrary runtime strings**; `add_template` / `add_template_owned` register named
  runtime templates. It is Jinja2-compatible, so `{% if %}` / `{% for %}` / `{{ var }}` all work,
  and it is small ("minimal dependencies" by its own description). There is an active
  `3.0.0-alpha.2` pre-release (2026-09-23); **stay on the 2.x stable line** for a shipping tool.
  MSRV 1.70. Actively maintained (last commit 2026-09-23).
- **`handlebars` 6.4.4** — mature, very popular (94M downloads), actively maintained (last commit
  2026-09-12). MSRV 1.85. More features (partials, helpers, block helpers) than a scaffolder needs,
  and a correspondingly larger dependency tree.
- **`askama` 0.16.1** — "Type-safe, **compiled** Jinja-like templates for Rust." Templates are
  compiled into the binary at build time via the derive macro; there is **no runtime template
  loading** in the public API (its `src/` has no dynamic-template path). For a scaffolder whose
  templates are data — often user-supplied or fetched — this is the wrong model unless you are happy
  to rebuild for every template change. MSRV 1.88 (edition 2024).
- **`tera` 2.4.0** — Jinja2/Django-style runtime engine, actively maintained (last commit
  2026-09-11). Fine, but no advantage over `minijinja` for this job.
- **Plain string replace** — for a genuinely tiny variable set, `str::replace` plus conditional
  inclusion decided in Rust code is the lowest-dependency answer. It stops scaling the moment you
  want loops, conditionals inside files, or escaped output, which is why the tools below use a real
  engine.

> **Recommendation:** **`minijinja` 2.24.0** with `render_str` for the common case, wrapping a
> "copy tree → substitute" loop. Plain `replace` is acceptable only if you commit to a fixed,
> tiny variable set. Avoid `askama` for this use case.

Sources: <https://crates.io/api/v1/crates/minijinja>, <https://docs.rs/minijinja/latest/minijinja/>,
<https://crates.io/api/v1/crates/handlebars>, <https://crates.io/api/v1/crates/askama>,
<https://crates.io/api/v1/crates/tera>, <https://crates.io/api/v1/crates/liquid>,
`minijinja-2.24.0/src/environment.rs`, `askama-0.16.1/Cargo.toml`.

### 3c. How the well-known tools actually do it

**`cargo-generate` 0.25.0** (published 2026-09-18; repo last pushed 2026-09-24, 2,486 stars) —
**git-repository-as-template model:**
- "leverage a pre-existing git repository as a template." Invocations: `cargo generate --git <url>`,
  shorthand `cargo generate gh:user/repo`, `gl:` (GitLab), `bb:` (Bitbucket), `sr:` (SourceHut).
  All fetching goes through `gix` (with `reqwest` for HTTP), gated behind the default `git` feature;
  library consumers who only use local templates can disable it.
- **Rendering: Shopify Liquid** — the manifest pins `liquid ~0.26` (plus `liquid-core`,
  `liquid-derive`, `liquid-lib`). Files ending `.liquid` have that suffix stripped after rendering;
  `README.md.liquid` overwrites `README.md`.
- Conditional/selector behaviour lives in the template's `cargo-generate.toml`:
  `[template] include/exclude/ignore`, `[placeholders]` (with `prompt`, `choices`, `default`,
  `type`, `regex`, and `value`/`label` pairs), and `[conditional.'<rhai expr>']` sections that switch
  `include`/`exclude`/`ignore`/`placeholders` based on a **Rhai** expression — e.g.
  `[conditional.'crate_type == "lib"'] ignore = ["src/main.rs"]`.
- Built-in placeholders: `project-name`, `crate_name`, `crate_type` (`--bin`/`--lib`), `authors`,
  `username`, `os-arch`, `within_cargo_project`, `is_init`. `--define key=value`,
  `--values-file`, and `CARGO_GENERATE_VALUE_<NAME>` env vars override them.
- Non-interactive/CI: `--silent` (requires `--name`) takes **all** variables from the values
  file/defines and **fails if a value is missing**; `--quiet` (requires `--continue-on-error`) and
  `--continue-on-error` complete the CI story.
- Template trust: template hooks can run arbitrary commands; `--allow-commands` is required to run
  them without a prompt, and the help text explicitly warns about the risk. This is the
  supply-chain cost of the remote-template model.
Sources: <https://crates.io/crates/cargo-generate>, `cargo-generate-0.25.0/Cargo.toml`,
`cargo-generate-0.25.0/src/args.rs`, `cargo-generate-0.25.0/guide/src/README.md`,
`.../guide/src/templates/{conditional,include_exclude,builtin_placeholders,template_defined_placeholders}.md`.

**`cargo new` / `cargo init`** — **built-in, hardcoded template; no fetching at all.**
- `cargo new [options] path`: "create a new Cargo package in the given directory. This includes a
  simple template with a Cargo.toml manifest, sample source file, and a VCS ignore file."
- `cargo init [options] [path]`: "create a new Cargo manifest in the **current** directory…
  If there are typically-named Rust source files already in the directory, those will be used."
- `--edition` **defaults to 2024** ("Possible values: 2015, 2018, 2021, 2024"), `--bin` (default) /
  `--lib`, `--name` (defaults to the directory name), `--vcs git|hg|pijul|fossil|none`.
- **Exit status, quoted:** `0: Cargo succeeded.` / `101: Cargo failed to complete.`
Sources: <https://doc.rust-lang.org/cargo/commands/cargo-new.html>,
<https://doc.rust-lang.org/cargo/commands/cargo-init.html>.

**Node `create-*-app` tools — templates shipped *inside* the npm package.**
`create-vite` 9.2.1 (published 2026-09-10, npm `latest`) ships its templates as sibling directories
**in the published tarball** — `package/template-vanilla`, `template-react-ts`, `template-vue`,
`template-svelte-ts`, … (16 template dirs) — and selects one with `--template`:
`npm create vite@latest my-vue-app -- --template vue`. No network fetch of the template itself; the
package *is* the template bundle, and getting a new template means publishing a new package version.
`create-react-app` 5.1.0 is still the npm `latest` (registry `modified` 2025-05-07).
Sources: <https://registry.npmjs.org/create-vite>, the published `create-vite-9.2.1.tgz` tarball
layout, <https://registry.npmjs.org/create-react-app>.

### 3d. Embedded vs remotely fetched templates — the real trade-off

| Axis | Embedded (in-binary / in-package) | Remote (git/npm/registry) |
|---|---|---|
| Offline use | **Works offline by construction** | Requires network unless cached |
| Version skew | **None** — template and CLI ship as one artifact | Real: template evolves independently of the CLI that consumes it |
| Updating a template | **Requires shipping a new binary** | Template owners push; users get it on next run |
| Trust / supply chain | **Templates are code you reviewed and built** | Fetched executable content; `cargo-generate` needs `--allow-commands` to run hooks unprompted |
| Template ecosystem | Closed — only your templates | Open — a whole community can publish templates |

Precedents map cleanly: `cargo new` (embedded, hardcoded) and `create-vite` (embedded in the npm
package) sit on the embedded side; `cargo-generate` (git repos, Liquid, Rhai hooks, `cargo-generate
topic` on GitHub for discovery) sits on the remote side and makes the CLI a *template runner* rather
than a template bundle.

> **Recommendation for a 2026 scaffolder:** **embed by default, allow override.** Ship your curated
> template tree via `rust-embed` so the common path is offline and version-locked, and add an
> explicit `--template <git-url|path>` escape hatch for power users. If you ever fetch remote
> templates, do what `cargo-generate` does: require an explicit opt-in before executing any template
> hook, and treat the template as untrusted input.

---

## 4. Edition 2024 — MSRV and what a new binary crate must know

- **Edition 2024 was stabilized in Rust 1.85.0**, announced 2025-02-20: "The Rust team is happy to
  announce the release of Rust 1.85.0. This stabilizes the 2024 edition as well."
  → **minimum supported `rustc` for edition 2024 is 1.85.0**.
  <https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/>
- The Edition Guide confirms it: "Rust 2024 · RFC #3501 · **Release version 1.85.0**".
  <https://doc.rust-lang.org/edition-guide/rust-2024/index.html>
- `cargo new` already defaults to it: "`--edition edition` … **Default is 2024.** Possible values:
  2015, 2018, 2021, 2024". <https://doc.rust-lang.org/cargo/commands/cargo-new.html>
- A freshly generated manifest in this workspace (cargo 1.98.1) literally reads:
  `edition = "2024"`.

### `rust-version`

- "The `rust-version` field is an optional key that tells cargo what version of the Rust toolchain
  you support for your package." It takes a bare version (`rust-version = "1.56"`) — no semver
  operators, no pre-release identifiers. (Cargo book heading: "MSRV: Respected as of 1.56".)
- **Enforcement is an error, and bypassable:** "When your package is compiled on an unsupported
  toolchain, Cargo will report that as an error to the user… A user can opt-in to an unsupported
  build of a package with the `--ignore-rust-version` flag."
- It also drives tooling: `cargo add` picks the newest dependency version compatible with your
  `rust-version`, the resolver may honour it, and `cargo clippy`'s `incompatible_msrv` lint uses it.
- **Note:** "Changing `rust-version` is assumed to be a minor incompatibility" — relevant when you
  bump your CLI's MSRV.
  <https://doc.rust-lang.org/cargo/reference/rust-version.html>
- crates.io publishes it: the API exposes `rust_version` per version (that is how the MSRVs in this
  document were read). <https://crates.io/api/v1/crates/askama>

### `resolver`

- Quoted from the Cargo book: `"3" (edition = "2024" default, requires Rust 1.84+): Change the
  default for resolver.incompatible-rust-versions from allow to fallback`.
- So an edition-2024 crate gets **resolver 3 implicitly**; no `resolver` key is needed. It is a
  **global/workspace** option — in a virtual workspace it belongs in `[workspace]`.
- Resolver 3 = **MSRV-aware resolution**: with `fallback`, "the resolver will prefer packages with a
  Rust version that is less than or equal to your own Rust version."
  <https://doc.rust-lang.org/cargo/reference/resolver.html>
- Corroborated by the 1.84.0 announcement: "1.84.0 stabilizes the minimum supported Rust version
  (MSRV) aware resolver." <https://blog.rust-lang.org/2025/01/09/Rust-1.84.0/>

### Edition 2024 changes a new binary crate should be aware of

From the official 1.85 announcement's own list: RPIT lifetime capture rules; `if let` temporary
scope; tail-expression temporary scope; match-ergonomics reservations; `unsafe extern` blocks;
unsafe `export_name`/`link_section`/`no_mangle` attributes; `unsafe_op_in_unsafe_fn` **now warns by
default**; references to `static mut` are a **deny-by-default error**; never-type fallback changes;
macro fragment specifier changes; **`gen` is now a reserved keyword**; and `#"foo"#`-style reserved
syntax.
Sources: <https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/>,
<https://doc.rust-lang.org/edition-guide/rust-2024/index.html>.

**Practical takeaway for a new binary crate:** `edition = "2024"`, `rust-version = "1.85"` (raise it
if a dependency demands more — e.g. `askama` 0.16.1 needs 1.88, `clap` 4.6.7 needs 1.85,
`handlebars` 6.4.4 needs 1.85), and no `resolver` key. Adding `rust-version` gives users a clean
diagnostic instead of a syntax error, and lets `cargo add` do MSRV-aware version picking for you.

---

## 5. CLI conventions a scaffolder should follow

### 5.1 Exit codes

- `std::process::ExitCode` implements the `Termination` trait, which is what `fn main()` may return:
  "A trait for implementing arbitrary return types in the `main` function… The default
  implementations are returning `libc::EXIT_SUCCESS`… In case of a failure, `libc::EXIT_FAILURE`."
  `impl Termination for Result<T, E>` means `fn main() -> Result<(), E>` yields a failure code
  automatically. <https://doc.rust-lang.org/std/process/trait.Termination.html>
- **Precedent:** the Cargo book documents `cargo new`'s status as `0: Cargo succeeded.` /
  `101: Cargo failed to complete.` — i.e. cargo deliberately uses a non-1 failure code.
  <https://doc.rust-lang.org/cargo/commands/cargo-new.html>
- **Canonical richer mapping:** `sysexits` **0.13.0** (published 2026-02-28, MSRV 1.87.0,
  edition 2024) provides "the system exit code constants as defined by `<sysexits.h>`" as an
  `ExitCode` enum with `Ok` default plus `Usage = 64`, `DataErr = 65`, `NoInput = 66`, … It
  implements `Termination` so it can be returned from `main`, and offers `is_success()` /
  `is_failure()`. <https://crates.io/crates/sysexits>, `sysexits-0.13.0/src/exit_code.rs`.
- The legacy `exitcode` **1.1.2** crate has not been released since **2017-06-18**; prefer
  `sysexits` or plain `ExitCode::from(n)` today. <https://crates.io/crates/exitcode>

**Suggested mapping:** `0` success; `64` (`EX_USAGE`) bad flags/args; `65` (`EX_DATAERR`) invalid
project name or template variable; `73` (`EX_CANTCREAT`) cannot write the destination; `74`
(`EX_IOERR`) mid-write failure after rollback; `78` (`EX_CONFIG`) missing/broken template.

### 5.2 `--dry-run`

Cargo's own conventions, quoted verbatim:
- `cargo publish --dry-run`: "**Perform all checks without uploading.**"
- `cargo install --dry-run` / `-n`: "**(unstable)** Perform all checks **without installing**."
Sources: <https://doc.rust-lang.org/cargo/commands/cargo-publish.html>,
<https://doc.rust-lang.org/cargo/commands/cargo-install.html>.

So the established Rust meaning is **do everything except the irreversible step**. For a scaffolder:
create nothing on disk, but run resolution, name validation, template rendering, and conflict
detection, then print the exact file list that *would* be written (and exit non-zero if the run
would fail). This is also the single most useful CI smoke test for a scaffolder.

### 5.3 Non-empty / already-existing directories

**Empirically verified in this workspace with `cargo 1.98.1` (primary evidence):**

```
$ mkdir nonempty && touch nonempty/existing.txt
$ cargo new nonempty
error: destination `/tmp/cargotest/nonempty` already exists

Use `cargo init` to initialize the directory

$ cargo init nonempty        # succeeds, leaves existing.txt alone
$ cargo new nonempty         # same error again
```

So: **`cargo new` refuses an existing destination directory** (it does not care whether it is empty —
existence alone is enough) and points the user at `cargo init`; **`cargo init` is the command that
tolerates an existing, non-empty directory** and preserves the files already there. The Cargo book
frames the split the same way: "See `cargo-init(1)` for a similar command which will create a new
manifest in an existing directory."
Sources: local `cargo 1.98.1` reproduction; <https://doc.rust-lang.org/cargo/commands/cargo-new.html>,
<https://doc.rust-lang.org/cargo/commands/cargo-init.html>.

> Minor gotcha worth knowing: the observed `cargo new` error printed to stderr while the shell
> reported a `0` status for the pipeline because of `| head`; do not trust a piped exit code here —
> the book's documented contract remains `0`/`101`.

**`cargo-generate` 0.25.0** — refuses to touch an existing target directory, from its source:

```rust
if project_dir.exists() {
    bail!("… Target directory already exists, aborting!");
}
```

and on the per-file level, `copy.rs` skips (with a `[Skipping] File already exists … and
`--overwrite` was not passed` warning) unless `--overwrite` is given. The CLI help is explicit that
**`--force` does *not* mean overwrite**: "Note that cargo generate won't overwrite an existing
directory, even if `--force` is given" (`--force` only suppresses kebab-case name conversion).
Source: `cargo-generate-0.25.0/src/template_variables/project_dir.rs`, `.../src/copy.rs`,
`.../src/args.rs`.

**Recommendation:** refuse a non-empty destination by default with a clear message; offer
`--force`/`--overwrite` for explicit opt-in and, crucially, **do not conflate the two meanings the
way cargo-generate does** — document precisely what your flags do. Consider an `--init` mode
(generate into the current directory) mirroring `cargo init`, and a union of both tools' semantics:
error by default, `--overwrite` to replace files, `--skip-existing` to merge.

### 5.4 Atomic / rollback behaviour on failure

No scaffolder in evidence performs a true multi-file transaction, so the convention is
**build-then-swap at the directory level**:

- `tempfile` **3.27.0** (published 2026-03-11, MSRV 1.63) is the maintained primitive: create a
  `TempDir`, render the whole template into it, then move it into place (for directories the move is
  a `rename`, which is atomic within one filesystem). Latest release is actively maintained.
  <https://crates.io/crates/tempfile>
- `atomicwrites` **0.4.4** has had **no release since 2024-09-19** and is niche — prefer
  `tempfile` plus `std::fs::rename` for the file-level case.
  <https://crates.io/crates/atomicwrites>
- Corroborating precedent: `cargo-generate` 0.25.0 depends on `tempfile = "3.27.0"` and
  `remove_dir_all ~1.0` — exactly the "materialise then swap / clean up" shape.
  Source: `cargo-generate-0.25.0/Cargo.toml`.
- For recursive walks in the same shape, `walkdir ~2.5` is what `cargo-generate` uses; `fs_extra`
  1.3.0 has had no release since 2023-02-03 and should be avoided for new code.
  <https://crates.io/crates/fs_extra>

**Recommended sequence:** validate inputs → render into a `TempDir` sibling of the destination →
(optionally run template hooks *inside the temp dir*, never in the user's tree) → `rename` into
place → on any error, drop the temp dir and exit non-zero. If a hook fails after the rename, say so
explicitly and leave an actionable message rather than pretending the run was atomic.

---

## 6. Producing a cross-platform single binary (brief)

**Targets (official platform support page):**

- **Tier 1 with host tools** (guaranteed to work, official binary releases, tested): `aarch64-apple-darwin`,
  `aarch64-pc-windows-msvc`, `aarch64-unknown-linux-gnu`, `i686-unknown-linux-gnu`,
  `x86_64-pc-windows-gnu`, `x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu`.
- **Tier 2 with host tools** (includes all the static-linking musl targets): `x86_64-unknown-linux-musl`
  ("64-bit Linux with musl 1.2.5") and `aarch64-unknown-linux-musl` ("ARM64 Linux with musl 1.2.5"),
  plus `riscv64gc-`/`powerpc64le-`/`loongarch64-unknown-linux-musl`.
- Practical matrix for a scaffolder: `x86_64-unknown-linux-gnu` + `x86_64-unknown-linux-musl`
  (static, no glibc floor), `aarch64-unknown-linux-musl`, `x86_64-pc-windows-msvc`,
  `aarch64-pc-windows-msvc`, `aarch64-apple-darwin`, `x86_64-apple-darwin`.
- Static linking: the musl targets above are the answer; note they are **Tier 2**, i.e. tested but
  not "guaranteed to work". If you need TLS, prefer `rustls` over OpenSSL to avoid vendoring.
  <https://doc.rust-lang.org/nightly/rustc/platform-support.html>

**`cargo-dist` — current version 0.32.0, published 2026-05-22, MSRV 1.74.**
Description: "Shippable application packaging for Rust." **It has been renamed: the package is still
`cargo-dist`, but the installed binary is `dist`** — its README is titled "`dist` (formerly known as
`cargo-dist`)" and the manifest declares `[[bin]] name = "dist"`. The library exposes `Init Args` /
`Generate Args` behind `dist init` / `dist generate`, and it builds installers, archives, and
checksums from CI. Declared homepage (unreachable from this sandbox): `https://axodotdev.github.io/cargo-dist`.
Sources: <https://crates.io/crates/cargo-dist>, <https://docs.rs/cargo-dist/latest/cargo_dist/>,
`cargo-dist-0.32.0/Cargo.toml` + `README.md`.

**`cargo-release` — current version 1.1.6, published 2026-09-16, MSRV 1.92.**
Purpose: release automation (version bump, tag, publish, changelog) rather than packaging. Its
docs.rs blurb warns: "cargo-release's versioning tracks compatibility for the binaries, not the API.
We upload to crates.io to distribute the binary. If using this as a library, be sure to pin the
version with a `=` version requirement operator." Official docs: <https://crate-ci.github.io/cargo-release/>
(host unreachable from this sandbox); sources: <https://crates.io/crates/cargo-release>,
<https://docs.rs/cargo-release/latest/cargo_release/>.

**Division of labour:** `dist` (cargo-dist 0.32.0) for cross-platform binaries, archives, and
installers; `cargo-release` 1.1.6 for cutting the version. Note `cargo-release`'s MSRV of **1.92** is
the highest MSRV in this document — if you adopt it as a *library*, raise your own MSRV accordingly
(or use it only as a CI binary).

---

## Sources

crates.io (versions, publish dates, MSRVs, download counts — all read from the registry API):
- <https://crates.io/api/v1/crates/clap> · <https://crates.io/crates/clap>
- <https://crates.io/api/v1/crates/clap_derive> · <https://crates.io/api/v1/crates/clap_builder>
- <https://crates.io/api/v1/crates/clap_complete> · <https://crates.io/api/v1/crates/clap_complete_nushell>
- <https://crates.io/api/v1/crates/inquire> · <https://crates.io/crates/inquire>
- <https://crates.io/api/v1/crates/dialoguer>
- <https://crates.io/api/v1/crates/promptuity>
- <https://crates.io/api/v1/crates/promkit>
- <https://crates.io/api/v1/crates/requestty>
- <https://crates.io/api/v1/crates/include_dir>
- <https://crates.io/api/v1/crates/rust-embed>
- <https://crates.io/api/v1/crates/handlebars>
- <https://crates.io/api/v1/crates/askama>
- <https://crates.io/api/v1/crates/minijinja>
- <https://crates.io/api/v1/crates/tera>
- <https://crates.io/api/v1/crates/liquid>
- <https://crates.io/crates/cargo-generate>
- <https://crates.io/crates/cargo-dist> · <https://crates.io/crates/cargo-release>
- <https://crates.io/crates/sysexits> · <https://crates.io/crates/exitcode>
- <https://crates.io/crates/tempfile> · <https://crates.io/crates/atomicwrites>
- <https://crates.io/crates/fs_extra>

docs.rs (rustdoc for the exact published versions):
- <https://docs.rs/clap/4.6.7/clap/>
- <https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html>
- <https://docs.rs/clap/latest/clap/_derive/index.html>
- <https://docs.rs/inquire/latest/inquire/>
- <https://docs.rs/inquire/latest/inquire/validator/>
- <https://docs.rs/include_dir/latest/include_dir/>
- <https://docs.rs/rust-embed/latest/rust_embed/>
- <https://docs.rs/minijinja/latest/minijinja/>
- <https://docs.rs/cargo-dist/latest/cargo_dist/>
- <https://docs.rs/cargo-release/latest/cargo_release/>

Published crate artifacts (downloaded from the crates.io static CDN — the manifests and `guide/`
sources Cargo itself shipped; likewise `<name>-<version>/Cargo.toml`, `README.md`, and `src/` paths
cited inline):
- <https://static.crates.io/crates/clap/clap-4.6.7.crate>
- <https://static.crates.io/crates/inquire/inquire-0.9.4.crate>
- <https://static.crates.io/crates/include_dir/include_dir-0.7.4.crate>
- <https://static.crates.io/crates/rust-embed/rust-embed-8.12.0.crate>
- <https://static.crates.io/crates/cargo-generate/cargo-generate-0.25.0.crate>
- <https://static.crates.io/crates/cargo-dist/cargo-dist-0.32.0.crate>
- <https://static.crates.io/crates/sysexits/sysexits-0.13.0.crate>
- <https://static.crates.io/crates/minijinja/minijinja-2.24.0.crate>
- <https://static.crates.io/crates/askama/askama-0.16.1.crate>
- <https://static.crates.io/crates/tera/tera-2.4.0.crate>
- <https://static.crates.io/crates/tempfile/tempfile-3.27.0.crate>

Official books / language docs:
- <https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/> — "Announcing Rust 1.85.0 and Rust 2024"
- <https://blog.rust-lang.org/2025/01/09/Rust-1.84.0/> — MSRV-aware resolver stabilization
- <https://doc.rust-lang.org/edition-guide/rust-2024/index.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-new.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-init.html>
- <https://doc.rust-lang.org/cargo/commands/cargo-publish.html> — `--dry-run`
- <https://doc.rust-lang.org/cargo/commands/cargo-install.html> — `-n`/`--dry-run`
- <https://doc.rust-lang.org/cargo/reference/rust-version.html>
- <https://doc.rust-lang.org/cargo/reference/resolver.html>
- <https://doc.rust-lang.org/std/process/trait.Termination.html>
- <https://doc.rust-lang.org/std/macro.include_str.html>
- <https://doc.rust-lang.org/std/macro.include_bytes.html>
- <https://doc.rust-lang.org/nightly/rustc/platform-support.html>

Official repositories:
- <https://github.com/clap-rs/clap>
- <https://github.com/mikaelmello/inquire>
- <https://github.com/console-rs/dialoguer>
- <https://github.com/Michael-F-Bryan/include_dir>
- <https://github.com/pyrossh/rust-embed>
- <https://github.com/sunng87/handlebars-rust>
- <https://github.com/askama-rs/askama>
- <https://github.com/mitsuhiko/minijinja>
- <https://github.com/Keats/tera>
- <https://github.com/cargo-generate/cargo-generate>
- <https://github.com/axodotdev/cargo-dist>
- <https://github.com/crate-ci/cargo-release>
- <https://github.com/sorairolake/sysexits-rs>

Package registries / tool sites used for the Node comparison:
- <https://registry.npmjs.org/create-vite> (and the published `create-vite-9.2.1.tgz` tarball layout)
- <https://registry.npmjs.org/create-react-app>
- <https://cargo-generate.github.io/cargo-generate/> (official book — referenced by the crate README,
  but DNS-null-routed in this sandbox)
- <https://crate-ci.github.io/cargo-release/> (official docs — unreachable from this sandbox)
