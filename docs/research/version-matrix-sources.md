# Version Matrix Data Sources (Fill API + per-platform availability)

**Research date:** 2026-09-25 (all live endpoints fetched on this date; Minecraft `latest.release` = **26.3**).
**Scope:** primary-source facts needed to design vinoa's built-in version matrix (MC 1.8.9 → current date-based releases 26.x), plus the optional online-refresh path.
**Method / evidence rules:** every claim is traced to a primary source — the PaperMC **Fill v3 API** (`fill.papermc.io/v3`), the PaperMC docs Markdown (`PaperMC/docs` on GitHub, the exact source the site is built from), PaperMC Maven metadata (`repo.papermc.io`), the owning projects' own repos/build files (`raw.githubusercontent.com`), the Gradle Plugin Portal Maven repo and Maven Central, Gradle's own service endpoints, Mojang's `piston-meta` version manifest, and Sponge's own docs/Maven. Blog posts, wikis and tutorials are not used.

> **Environment caveats that affect citations.**
> - `api.github.com` returns **HTTP 403** and `github.com` HTML **times out** from this workspace. Where a GitHub *API* would be the natural source (e.g. PowerNukkitX releases), I used `raw.githubusercontent.com` (same content, different path) or the project's Maven repository, and flagged the gap in §8.
> - The `web_search` tool is unavailable in this session ("configured web provider not registered"), so nothing here rests on secondary summaries.
> - `repo.papermc.io` is a Nexus instance whose **metadata files are incomplete** for legacy coordinates: `com.destroystokyo.paper:paper-api/maven-metadata.xml` lists only `1.16.5-R0.1-SNAPSHOT`, while the repository **browse listing** and per-version `maven-metadata.xml` files prove 1.9.4 → 1.16.5 all exist. Where the two disagree, the per-version metadata is used and the discrepancy is called out in §2.3.
> - Nexus `browse` returns **HTTP 200 for non-existent directories** on `repo.papermc.io` (any path). Existence was therefore checked against per-version `maven-metadata.xml` / `maven-metadata.xml` lists, never against a bare directory 200.

---

## 0. Design-relevant conclusions (TL;DR)

| # | Finding | Consequence for the built-in matrix |
|---|---|---|
| 1 | Fill v3 is the **only** live PaperMC download API — `api.papermc.io/v2` is dead (404 at `https://api.papermc.io/v2/`) | Refresh path = Fill v3; no v2 fallback code |
| 2 | Fill `/builds/latest` **ignores** any `channel` query parameter; only `/builds?channel=STABLE` filters. PaperMC's own docs filter **client-side** with `map(select(.channel=="STABLE")) | .[0].id` | Do the channel filter in Rust, never rely on `latest` |
| 3 | Version IDs are **not** uniformly patch-level. Paper has `26.1.1`/`26.1.2` but **no** `26.1`; Paper also has **no** `1.8.9` (only `1.8.8`), no `1.20.3`, no `1.21.2` | The matrix must be a whitelist per platform, not a generated range |
| 4 | **No `paper-api` artifact exists for 1.8.9.** Earliest published is `com.destroystokyo.paper:paper-api:1.9.4-R0.1-SNAPSHOT` | For target 1.8.9, emit no `paper-api` dependency (Bukkit/Spigot API or Spigot-mapped `paper-api` is impossible) |
| 5 | The coordinate namespace flips at **1.17**: `com.destroystokyo.paper:paper-api` (1.9.4–1.16.5) → `io.papermc.paper:paper-api` (1.17+) | Two coordinate templates, switched on a version breakpoint |
| 6 | The version-**string** format flips at **26.1**: `{VERSION}-R0.1-SNAPSHOT` → `{VERSION}.build.{n}-{channel}` / `{VERSION}.build.+` | Parser must accept both; `+` dynamic versions only work on the new family |
| 7 | Java has **two** official answers that differ: Fill's `java.version.minimum` (hard floor to *run*) vs Paper's docs "Recommended Java Version" table. They disagree for 1.12–1.16.4 (min 8 / recommended 11), 1.16.5 (min 8 / rec 16), 1.17–1.17.1 (min 16 / rec 17), 1.18–1.19.4 (min 17 / rec 17), 1.20–1.20.4 (min 17 / rec 21) | Store **both** columns; pick `minimum` for `--java` sanity checks, `recommended` for the generated toolchain |
| 8 | Last MC on **Java 21** = `1.21.11`; first on **Java 25** = `26.1` (Paper docs table). Fill's minimum agrees | Single clean breakpoint for the toolchain default |
| 9 | Velocity/BungeeCord are **proxy-protocol** artifacts, not MC-version artifacts (Velocity ships `1.7.2 → 26.3` protocol support; BungeeCord ships `1.21-R0.4-SNAPSHOT`-style *major-only* strings) | Model proxies as `(platform, proxy_version)`, not `(platform, mc_version)` |
| 10 | Nukkit (and PowerNukkitX) are **Bedrock Edition** servers (their own READMEs say so; version badges track *Bedrock* builds like `26.50`, protocol `2193`) | Keep Nukkit **out** of the Java-Edition MC matrix; give it its own Bedrock-version axis |
| 11 | Folia's oldest available version is **1.19.4** | Hard lower bound for Folia; no `folia` target below that |
| 12 | Gradle 9.x **requires Java 17+ to run**; current stable is **9.8.0**; Java 25 toolchains need Gradle **9.1.0+** | Generated wrapper floor: 9.8.0 for Java 25 projects, 8.14.x if Java 8–11 support is ever needed |
| 13 | Plugin floors are encoded in Gradle Module Metadata as `org.gradle.plugin.api-version` (authoritative, not docs prose): shadow 9.6.1 → **Gradle 9.2.0**, paperweight 2.0.0-beta.24 → **Gradle 9.7.1**, run-paper 3.1.0 → **Gradle 9.7.0** | Read the `.module` file to derive floors; don't scrape changelogs |

---

## 1. PaperMC Fill v3 API (`https://fill.papermc.io/v3`)

### 1.1 Endpoints (all verified live)

| Method + path | Returns | Verified |
|---|---|---|
| `GET /v3/projects` | All projects + their version map | 200 |
| `GET /v3/projects/{project}` | One project: `{project:{id,name}, versions:{group:[versions…]}}` | 200 |
| `GET /v3/projects/{project}/versions/{version}` | Version metadata (`support`, `java`, recommended flags) + a **flat list of build ids** | 200 |
| `GET /v3/projects/{project}/versions/{version}/builds` | Array of build objects, **newest first** | 200 |
| `GET /v3/projects/{project}/versions/{version}/builds/{build}` | One build object | 200 |
| `GET /v3/projects/{project}/versions/{version}/builds/latest` | The newest build object (channel filter **ignored** — see §1.4) | 200 |

There is **no download-redirect endpoint**: `…/builds/232/downloads/{anything}` returns `{"ok":false,"error":"unknown_method",…}` for every key form tried (`server:default`, `server%3Adefault`, `paper-1.21.4-232.jar`, `default`, `server`). Downloads are taken from the `downloads` field of the build object.

Also verified dead: `https://api.papermc.io/v2/` → **404**. `https://fill.papermc.io/` → **302** to `/swagger-ui/index.html`, but the OpenAPI document itself is **not** served at `/v3/api-docs`, `/api-docs` or `/openapi.json` (all 404).

**Project ids available** (`https://fill.papermc.io/v3/projects`): `folia`, `paper`, `travertine`, `velocity`, `waterfall`. **There is no `bungeecord` project** — BungeeCord is not distributed through Fill (see §4.2).

### 1.2 Real response excerpts

`GET /v3/projects/paper` (truncated; note the two-level version map — *group* → *versions* — newest group first):

```json
{"project":{"id":"paper","name":"Paper"},
 "versions":{
   "26.3":["26.3","26.3-rc-3"],
   "26.2":["26.2","26.2-rc-2"],
   "26.1":["26.1.2","26.1.1"],
   "1.21":["1.21.11","1.21.11-rc3","1.21.11-rc2","1.21.11-rc1","1.21.11-pre5","1.21.11-pre4","1.21.11-pre3","1.21.10","1.21.9","1.21.9-rc1","1.21.9-pre4","1.21.9-pre3","1.21.9-pre2","1.21.8","1.21.7","1.21.6","1.21.5","1.21.4","1.21.3","1.21.1","1.21"],
   "1.20":["1.20.6","1.20.5","1.20.4","1.20.2","1.20.1","1.20"],
   "1.8":["1.8.8"],
   "1.7":["1.7.10"]}}
```

`GET /v3/projects/paper/versions/1.21.4` — the `.version.java.version.minimum` field is the **hard Java floor** (this is the field to consume for the Java matrix), and `.builds` is a flat array of ints:

```json
{"version":{"id":"1.21.4",
  "support":{"status":"UNSUPPORTED","end":"2025-07-17"},
  "java":{"version":{"minimum":21},
          "flags":{"recommended":["-XX:+AlwaysPreTouch","-XX:+DisableExplicitGC","…","-XX:SurvivorRatio=32"]}}},
 "builds":[232,231,230,…,2,1]}
```

`GET /v3/projects/paper/versions/1.21.4/builds/latest` — the build object shape (this is the whole schema; `channel` ∈ `ALPHA|BETA|STABLE`):

```json
{"id":232,
 "time":"2025-06-09T10:18:55.778Z",
 "channel":"STABLE",
 "commits":[{"sha":"12d8fe0beb21c1a1d9b093fb411884367cce9e7e",
             "time":"2025-06-09T09:57:21Z",
             "message":"Fix infinite loop in RegionFile IO\n\n…"}],
 "downloads":{"server:default":{
     "name":"paper-1.21.4-232.jar",
     "checksums":{"sha256":"5ee4f542f628a14c644410b08c94ea42e772ef4d29fe92973636b6813d4eaffc"},
     "size":51437498,
     "url":"https://fill-data.papermc.io/v1/objects/5ee4f542f628a14c644410b08c94ea42e772ef4d29fe92973636b6813d4eaffc/paper-1.21.4-232.jar"}}}
```

`GET /v3/projects/paper/versions/26.3/builds` (first element — newest) confirms that the newest build is not necessarily `STABLE`:

```json
[{"id":41,"time":"2026-09-25T03:26:20Z","channel":"ALPHA",
  "commits":[{"sha":"a15fed9c…","time":"2026-09-25T03:24:50Z","message":"Update Gradle to 9.8 and paperweight to beta.24\n"}],
  "downloads":{"server:default":{"name":"paper-26.3-41.jar","checksums":{"sha256":"2b77166e…"},"size":65176386,
    "url":"https://fill-data.papermc.io/v1/objects/2b77166ee61886a9bc9ab33dc9e4847fa3538b36d9ba6e5f2fa7ed90973aa748/paper-26.3-41.jar"}}}]
```

### 1.3 Error shape (uniform)

| Request | HTTP | Body |
|---|---|---|
| `/v3/projects/nope` | 404 | `{"ok":false,"error":"project_not_found","message":"No project was found with the given identifier."}` |
| `/v3/projects/paper/versions/1.99` | 404 | `{"ok":false,"error":"version_not_found","message":"No version was found with the given identifier."}` |
| `/v3/projects/paper/versions/1.99/builds` | 404 | same `version_not_found` |
| `/v3/projects/paper/versions/1.21.4/builds/999999` | 404 | `{"ok":false,"error":"build_not_found","message":"No build was found with the given identifier."}` |
| unknown route | 404 | `{"ok":false,"error":"unknown_method","message":"No endpoint GET /… ."}` |

Paper's own docs tell you to test `jq -e '.ok == false'` and read `.message`.

### 1.4 Channel (`STABLE`/`BETA`/`ALPHA`) filtering — the important gotcha

* `GET …/builds?channel=STABLE` **works**: Paper `1.21.4` returns 224 builds unfiltered and **130** with `?channel=STABLE` (94 `ALPHA`). Paper `26.2` → `{ALPHA:47, BETA:24, STABLE:43}`; `26.1.2` → `{ALPHA:44, BETA:5, STABLE:22}`; `1.21.11` → `{ALPHA:12, BETA:17, STABLE:63}`; `26.3` → `{ALPHA:39}` (no stable yet).
* `GET …/builds/latest?channel=STABLE` **ignores the parameter.** Proof: `paper/26.2/builds/latest?channel=ALPHA` returns `{"id":129,"channel":"STABLE"}`, and `paper/26.3/builds/latest?channel=STABLE` returns `{"id":41,"channel":"ALPHA"}`.
* PaperMC's **official** documented recipe filters client-side:
  `jq -r 'map(select(.channel == "STABLE")) | .[0] | .id'` over `…/builds`.
* Observed channel values across `paper`, `folia`, `velocity`, `waterfall`: only `ALPHA`, `BETA`, `STABLE`. No `RECOMMENDED` value was observed in any live response (the PaperMC docs' internal TypeScript union does mention `"RECOMMENDED"`, so treat it as accepted-but-unused).
* No pagination parameters were needed: `…/builds` returned all 224 rows for 1.21.4 in one response.

### 1.5 Reverse lookup: "which builds exist for a given Minecraft version?"

`GET /v3/projects/{project}/versions/{mcVersion}/builds` **is** the reverse lookup. Builds are returned newest-first, so the stable build is `first(builds | select(.channel=="STABLE"))`. The `version` id is exactly the Minecraft version string (`1.21.4`, `26.2`), so no mapping table is needed for this step.

Practical refresh algorithm (mirrors PaperMC's own docs):
1. `GET /v3/projects/{project}` → flatten `.versions | to_entries[].value[]`, sort descending (the docs use `sort -V -r`).
2. For each version, `GET /v3/projects/{project}/versions/{v}/builds` and take the first `STABLE` entry.
3. Cache `{project, version, build_id, channel, downloads["server:default"].url, sha256}`; keep the previous matrix when the network fails.

### 1.6 Request requirements (operational)

PaperMC's downloads-service documentation **requires** a non-generic `User-Agent` that identifies the software and carries a contact URL or e-mail:

> "All requests must now include a valid User-Agent header that: Clearly identifies your software or company / Is not generic (e.g. curl, wget, or similar defaults) / Includes a contact URL or email address"

Example given by PaperMC: `mc-image-helper/1.39.11 (https://github.com/itzg/docker-minecraft-server)`.

---

## 2. `io.papermc.paper:paper-api` coordinate formats

### 2.1 The two families

From the PaperMC docs source (`project-setup.mdx`, verbatim):

> If you want to reference a specific build, you can do so by replacing the `+` with the build identifier (i.e. `{VERSION}.build.25-stable` for the 25th stable build of a version). Before `26.1` (`1.21.11` and below), the version string format used was `{VERSION}-R0.1-SNAPSHOT`, with no way to reference a specific build.

| Family | Versions | Coordinate template | Specific-build pinning |
|---|---|---|---|
| **Legacy** | 1.9.4 → 1.16.5 | `com.destroystokyo.paper:paper-api:{MC}-R0.1-SNAPSHOT` | impossible |
| **Modern (old string)** | 1.17 → 1.21.11 | `io.papermc.paper:paper-api:{MC}-R0.1-SNAPSHOT` | impossible |
| **Date-based** | 26.1.1 → current | `io.papermc.paper:paper-api:{MC}.build.+` (dynamic) or `{MC}.build.{N}-{alpha\|beta\|stable}` (pinned) | yes |

Repository for all of them: `https://repo.papermc.io/repository/maven-public/` (id `papermc`). The docs' own Gradle snippet is:

```kotlin
repositories {
  maven(url = "https://repo.papermc.io/repository/maven-public/") { name = "papermc" }
}
dependencies {
  compileOnly("io.papermc.paper:paper-api:{LATEST_PAPER_RELEASE}.build.+")
}
java { toolchain.languageVersion.set(JavaLanguageVersion.of(25)) }
```

Paper's docs also derive the pinned form programmatically. From `PaperMC/docs` `src/utils/versions.ts`:

```ts
export const LATEST_PAPER_BUILD_API_VERSION =
  `${LATEST_PAPER_RELEASE}.build.${paperBuild?.id}-${paperBuild?.channel.toLocaleLowerCase()}`;
```

i.e. **`{VERSION}.build.{buildId}-{channel-lowercase}`** — `26.2.build.129-stable`. The Maven equivalent needs a range: `[{VERSION}.build,)`.

### 2.2 Exact artifact strings for the requested versions

| MC version | Coordinate | Published? |
|---|---|---|
| **1.8.9** | **none exists** | **NO** — no `paper-api` artifact was ever published for 1.8.8 or 1.8.9 (both `…/com/destroystokyo/paper/paper-api/1.8.9-R0.1-SNAPSHOT/maven-metadata.xml` and the 1.8.8 equivalent return **404**) |
| 1.8.8 | none exists | NO (Paper *server* builds exist: `fill` paper `1.8.8`, 3 builds, all `STABLE`) |
| **1.12.2** | `com.destroystokyo.paper:paper-api:1.12.2-R0.1-SNAPSHOT` | yes (`<value>1.12.2-R0.1-20190714.184133-413`) |
| **1.16.5** | `com.destroystokyo.paper:paper-api:1.16.5-R0.1-SNAPSHOT` | yes (`<value>1.16.5-R0.1-20211218.082619-371`) |
| **1.17.1** | `io.papermc.paper:paper-api:1.17.1-R0.1-SNAPSHOT` | yes |
| **1.19.4** | `io.papermc.paper:paper-api:1.19.4-R0.1-SNAPSHOT` | yes |
| **1.20.6** | `io.papermc.paper:paper-api:1.20.6-R0.1-SNAPSHOT` | yes |
| **1.21.4** | `io.papermc.paper:paper-api:1.21.4-R0.1-SNAPSHOT` | yes |
| **1.21.11** | `io.papermc.paper:paper-api:1.21.11-R0.1-SNAPSHOT` | yes (plus `1.21.11-rc1/2/3`, `1.21.11-pre3/4/5`) |
| **26.2** | `io.papermc.paper:paper-api:26.2.build.+` (dynamic) / `26.2.build.129-stable` (pinned) | yes — metadata tail is `26.2.build.126-stable … 26.2.build.129-stable` |

**Earlier than 1.12.2, for completeness** — published `com.destroystokyo.paper:paper-api` versions (complete browse listing): `1.9.4`, `1.10.2`, `1.11`, `1.11.1`, `1.11.2`, `1.12`, `1.12.1`, `1.12.2`, `1.13-pre7`, `1.13`, `1.13.1`, `1.13.2`, `1.14`, `1.14.1`, `1.14.2`, `1.14.3`, `1.14.4`, `1.15`, `1.15.1`, `1.15.2`, `1.16.1`, `1.16.2`, `1.16.3`, `1.16.4`, `1.16.5`. **Earliest = `1.9.4-R0.1-SNAPSHOT`.**

**Modern family** — published `io.papermc.paper:paper-api` versions (complete browse listing, 1.17 → 26.x): `1.17`, `1.17.1`, `1.18`, `1.18-rc3`, `1.18.1`, `1.18.2`, `1.19`, `1.19.1`, `1.19.2`, `1.19.3`, `1.19.4`, `1.20`, `1.20.1`, `1.20.2`, `1.20.3`, `1.20.4`, `1.20.5`, `1.20.6`, `1.21`, `1.21.1`, `1.21.3`, `1.21.4`, `1.21.5`, `1.21.5-no-moonrise-SNAPSHOT`, `1.21.6`, `1.21.7`, `1.21.8`, `1.21.9`(+`pre2/3/4`, `rc1`), `1.21.10`, `1.21.11`(+`pre3/4/5`, `rc1/2/3`), then `26.1.1.build.8-alpha` … `26.3-pre-2.build.0-alpha`.

### 2.3 Two traps

1. **`io.papermc.paper:paper-api` starts at `1.17-R0.1-SNAPSHOT`.** There is no `io.papermc.paper:paper-api:1.16.5-R0.1-SNAPSHOT` — old blog posts that use it are wrong.
2. **Artifact existence ≠ server-build existence, in both directions.** `io.papermc.paper:paper-api:1.20.3-R0.1-SNAPSHOT` **exists** (`<value>1.20.3-R0.1-20231207.043048-3`) although Fill lists **no Paper 1.20.3 server build**. Conversely `1.21` exists as an artifact but Paper has no `1.16` artifact while it *does* have a `1.16.5` artifact. The matrix therefore needs an explicit `paper_api` field per version, not a derived rule.
3. `repo.papermc.io`'s aggregate `com/destroystokyo/paper/paper-api/maven-metadata.xml` lists **only `1.16.5-R0.1-SNAPSHOT`** — do not use it to enumerate the legacy range.

---

## 3. Java version per Minecraft version

Two official tables exist and they answer different questions.

### 3.1 Paper's official **recommended** table (docs, verbatim)

Source: `PaperMC/docs` → `src/content/docs/paper/admin/getting-started/getting-started.mdx`

```
| Paper Version     | Recommended Java Version |
|-------------------|--------------------------|
| 1.7.10 to 1.11    | Java 8                   |
| 1.12 to 1.16.4    | Java 11                  |
| 1.16.5            | Java 16                  |
| 1.17 to 1.19      | Java 17                  |
| 1.20 to 1.21.11   | Java 21                  |
| 26.1+             | Java 25                  |
```

### 3.2 Fill API's **hard minimum** (`version.java.version.minimum`), swept across every Paper version

| MC range | minimum | verified endpoints |
|---|---|---|
| 1.8.8 → 1.16.5 | **8** | 1.8.8, 1.9.4, 1.10.2, 1.11.2, 1.12, 1.12.1, 1.12.2, 1.13, 1.13.1, 1.13.2, 1.14–1.14.4, 1.15–1.15.2, 1.16.1–1.16.5 — all `8` |
| 1.17 → 1.17.1 | **16** | 1.17, 1.17.1 |
| 1.18 → 1.20.4 | **17** | 1.18, 1.18.1, 1.18.2, 1.19–1.19.4, 1.20, 1.20.1, 1.20.2, 1.20.4 |
| 1.20.5 → 1.21.11 | **21** | 1.20.5, 1.20.6, 1.21, 1.21.1, 1.21.3–1.21.11 |
| 26.1.1 → 26.3 | **25** | 26.1.1, 26.1.2, 26.2, 26.3 |

### 3.3 The breakpoints that matter for the design

| Question | Answer | Source |
|---|---|---|
| Last MC on **Java 21** | **`1.21.11`** | Paper docs table (`1.20 to 1.21.11 → Java 21`); Fill: `1.21.11 → minimum 21` |
| First MC on **Java 25** | **`26.1`** (Paper ships `26.1.1`/`26.1.2`, no plain `26.1`) | Paper docs table (`26.1+ → Java 25`); Fill: `26.1.1 → minimum 25` |
| First MC requiring Java 17 as a *minimum* | `1.18` (1.17/1.17.1 only need 16) | Fill sweep |
| First MC requiring Java 21 as a *minimum* | `1.20.5` | Fill sweep |
| Java 16 appears **only** at 1.17–1.17.1 as a minimum, and only at 1.16.5 as a *recommendation* | — | both tables |

**Design note:** store `java_min` (Fill) and `java_recommended` (docs) separately. vinoa's `--java` validation should compare against `java_min`; the generated `toolchain.languageVersion` should default to `java_recommended` for the selected target.

**Folia** uses the same scale (Fill, project `folia`): `1.19.4 → 17`, `1.20.1/1.20.2/1.20.4 → 17`, `1.20.6 → 21`, `1.21.4/1.21.8/1.21.11 → 21`, `26.1.2/26.2 → 25`.

---

## 4. Per-platform version-availability sources

Summary first; details per platform below.

| Platform | Kind | Canonical availability source | Machine-readable? | Java requirement |
|---|---|---|---|---|
| Paper | Java server | `https://fill.papermc.io/v3/projects/paper` | **yes, JSON API** | per-version, Fill `java.version.minimum` (8/16/17/21/25) |
| Folia | Java server (regionised) | `https://fill.papermc.io/v3/projects/folia` | **yes, JSON API** | per-version, Fill (17/21/25) |
| Velocity | Java proxy | `https://fill.papermc.io/v3/projects/velocity` + `ProtocolVersion.java` | **yes, JSON API** + source enum | per-version, Fill (8/11/17/21/25); docs: "Velocity requires at least Java 25" |
| Waterfall | Java proxy (EOL) | `https://fill.papermc.io/v3/projects/waterfall` | **yes, JSON API** | per-version, Fill |
| BungeeCord | Java proxy | `https://hub.spigotmc.org/nexus/content/groups/public/net/md-5/bungeecord-api/maven-metadata.xml` + `https://hub.spigotmc.org/jenkins/job/BungeeCord/api/json` | yes (Maven XML + Jenkins JSON) | `maven.compiler.release=17` in current `pom.xml` |
| Sponge (SpongeAPI / SpongeVanilla / SpongeForge) | Java server + API | `https://repo.spongepowered.org/repository/maven-public/org/spongepowered/spongevanilla/maven-metadata.xml` (also `spongeapi`, `spongeforge`) + docs API-versions table | **yes, Maven metadata** (version string encodes MC + API) | SpongeAPI 17.0.0 bytecode = **Java 21** (class major 65) |
| Minestom | Java server library (not MC-pinned runtime) | `https://repo1.maven.org/maven2/net/minestom/minestom/maven-metadata.xml` | **yes, Maven metadata** (version string encodes MC) | Gradle metadata `org.gradle.jvm.version = 25` |
| Nukkit / PowerNukkitX | **Bedrock** server | `https://repo.powernukkitx.org/releases/` + project README/releases | partially (repo is snapshot-only) | `build.gradle.kts`: `release = 25`, toolchain 25 |

### 4.1 Folia

* Fill project `folia`; complete version list (as of 2026-09-25): `26.2`, `26.1.2`, `1.21.11`, `1.21.8`, `1.21.6`, `1.21.5`, `1.21.4`, `1.20.6`, `1.20.4`, `1.20.2`, `1.20.1`, `1.19.4`.
* **Hard lower bound: `1.19.4`.** Everything 1.8.9–1.19.3 is a non-existent combination.
* Support status from Fill: `1.21.11 → SUPPORTED`, `26.1.2 → SUPPORTED`, `26.2 → SUPPORTED`, older → `UNSUPPORTED`.

### 4.2 BungeeCord

* **No Fill project, no `fill.papermc.io/v3/projects/bungeecord`.** The authoritative enumeration is the SpigotMC Nexus metadata:
  `https://hub.spigotmc.org/nexus/content/groups/public/net/md-5/bungeecord-api/maven-metadata.xml`
* `<latest>26.1-R0.1-SNAPSHOT</latest>`, `<lastUpdated>20260915210855</lastUpdated>`, 21 versions total:
  `1.4.7-SNAPSHOT`, `1.5-SNAPSHOT`, `1.6.1-SNAPSHOT`, `1.6.2-SNAPSHOT`, `1.6.4-SNAPSHOT`, `1.7-SNAPSHOT`, `1.8-SNAPSHOT`, `1.9-SNAPSHOT`, `1.10-SNAPSHOT`, `1.11-SNAPSHOT`, `1.12-SNAPSHOT`, `1.13-SNAPSHOT`, `1.14-SNAPSHOT`, `1.15-SNAPSHOT`, `1.16-R0.5-SNAPSHOT`, `1.17-R0.1-SNAPSHOT`, `1.18-R0.1-SNAPSHOT`, `1.19-R0.1-SNAPSHOT`, `1.20-R0.3-SNAPSHOT`, `1.21-R0.4-SNAPSHOT`, `26.1-R0.1-SNAPSHOT`.
* **Granularity is major-version only** (no `1.20.6`, no `1.21.4`). Note also that **`26.2` and `26.3` do not exist yet** for BungeeCord.
* The Jenkins job is `https://hub.spigotmc.org/jenkins/job/BungeeCord/` (last build `#2096`); its job description carries the historical build→version mapping (`#1119 = 1.7.10`, `#701 = 1.6.4`, …). Build descriptions in the API are `null`, so the Jenkins job is *not* a reliable per-build version source — use Nexus.
* Java: current `pom.xml` sets `<maven.compiler.release>17</maven.compiler.release>` (master). BungeeCord is a *proxy-protocol* artifact: it does **not** pin exact Minecraft client versions.

### 4.3 Velocity / Waterfall

* Fill project `velocity` — these are **proxy** versions, not MC versions. Version list: `1.0.10`, `1.1.9`, `3.1.0`, `3.1.1`, `3.2.0-SNAPSHOT`, `3.3.0-SNAPSHOT`, `3.4.0`, `3.5.0`, `3.5.1`, `3.6.0-SNAPSHOT`, `4.0.0`, `4.1.1`, `4.2.0`.
* Fill `java.version.minimum` per Velocity line: `1.0.10 → 8`, `1.1.9 → 8`, `3.1.0/3.1.1 → 11`, `3.2.0-SNAPSHOT → 11`, `3.3.0-SNAPSHOT → 17`, `3.4.0 → 17`, `3.5.0/3.5.1/3.6.0-SNAPSHOT → 21`, `4.0.0/4.1.1/4.2.0 → 25`.
* Support status: `4.1.1`/`4.2.0` → `SUPPORTED`; `3.5.1` → `DEPRECATED` (end `2026-09-30`); `4.0.0` → `UNSUPPORTED` (end `2026-08-24`); `3.5.0` → `UNSUPPORTED` with **0 builds** (do not offer it).
* **Protocol (not MC-version) support range.** From `api/.../network/ProtocolVersion.java`:
  * branch `dev/3.0.0` (3.x line): `MINIMUM_VERSION = MINECRAFT_1_7_2`, last enum constant `MINECRAFT_26_2` (protocol `776`).
  * branch `dev/4.0.0` (4.x line, `version=4.2.1-SNAPSHOT`): `MINIMUM_VERSION = MINECRAFT_1_7_2`, last enum constant `MINECRAFT_26_3` (protocol `777`).
  * `getVersionString()` is built as `"%s-%s".format(MINIMUM_VERSION.getVersionIntroducedIn(), MAXIMUM_VERSION.getMostRecentSupportedVersion())` → e.g. `1.7.2-26.3`.
  * The enum collapses patches to a single protocol: `MINECRAFT_1_21_9(773,"1.21.9","1.21.10")`, `MINECRAFT_26_1(775,"26.1","26.1.1","26.1.2")`, etc.
* Velocity docs (`docs.papermc.io/velocity/getting-started/`): "**Velocity requires at least Java 25**" (this is the 4.x docs).
* **Waterfall** is deprecated: the docs landing page says "We recommend you transition to Velocity." Fill still lists it (`1.21`…`1.11` groups) but `waterfall/1.21` reports `UNSUPPORTED` (end `2024-03-26`). Treat Waterfall as a legacy/opt-in platform.

### 4.4 Sponge

Two useful sources, both primary, and they currently disagree in coverage:

1. **Maven metadata (authoritative and current).** `org.spongepowered:spongevanilla` version strings are `{MC_VERSION}-{SPONGE_API_VERSION}-RC{build}`:
   * `26.2-20.0.0-RC2701`, `26.3-21.0.0-RC2721`, `1.21.11-18.0.0-RC…`, `1.16.5-8.2.1-RC…`, `1.12.2-7.4.8-RC…`
   * 3065 published versions; `org.spongepowered:spongeapi` has `<release>17.0.0</release>` and `<latest>20.0.0-SNAPSHOT</latest>`; `org.spongepowered:spongeforge` uses `{MC}-{FORGE}-{API}-RC{n}` (e.g. `26.2-65.1.1-20.0.0-RC2703`).
   * Derived MC → SpongeAPI mapping: the canonical, machine-readable form is the `sponge_api` column of the table in **§7**. It is computed as the *dominant* API major line for that MC version (the major with the most SpongeVanilla builds), then the highest patch within it — which reproduces the docs table for the early lines (`1.8.9→4.2.0`, `1.9.4→5.x`, `1.10.2→5.2.0`, `1.11.2→6.x`, `1.12.2→7.4.8`, `1.16.5→8.2.1`) and extends it past the docs' cutoff (`1.21.11→18.0.0`, `26.1.x→19.0.0`, `26.2→20.0.0`, `26.3→21.0.0`).
   * **Caveat — one MC version can legitimately carry builds from two API lines.** Examples: `1.12.2` has 401 API-7.x builds **and** 51 early API-8.x builds; `1.21.8` has both `16.0.1` and `17.0.0`; `1.11.2` has 73 API-6.x and 17 API-7.x builds. The dominant line is the right default, but the data file should allow an explicit API-version override rather than deriving it.
2. **Docs table.** `docs.spongepowered.org/stable/en/plugin/api-versions.html` — "This page explains which API versions exist, and to which Minecraft version(s) their implementations belong", with rows from API 5.0.0 → 14.0.0 mapping to `SpongeVanilla`/`SpongeForge`/`SpongeNeo` MC versions (e.g. `14.0.0 → SpongeVanilla (1.21.4)`). **This page is stale** relative to Maven (it stops at 14.0.0 while Maven has 21.0.0). There is also a "Reading the Download Filename" page (`versions/versioning.html`) stating that implementation version strings "include the target Minecraft version as well as the SpongeAPI version".
3. The same docs page documents how to read the MC version from the sources: for SpongeAPI 8+, `gradle.properties` in the project root contains `minecraftVersion`; for SpongeForge/SpongeVanilla the same key lives in the SpongeCommon repo.
4. **Java:** `org.spongepowered:spongeapi:17.0.0`'s classes are **class-file major 65 = Java 21**. (Earlier lines target lower: API 8.x = Java 8/11 era.)

### 4.5 Minestom

* **Maven Central is the availability source, and the version string itself encodes the target MC version**: `net.minestom:minestom`, format `{YYYY.MM.DD[a|b|c]}-{MC_VERSION}`.
  Full tail: `… 2025.12.20b-1.21.11`, `2025.12.20c-1.21.11`, `2026.01.01-1.21.11` … `2026.05.11-1.21.11`, `2026.05.17b-26.1.1`, `2026.05.17c-26.1.1`, `2026.05.17-1.21.11`, `2026.06.02-26.1.2`, `…`, `2026.07.12-26.2`, `…`, `2026.09.12-26.2`. `<release>2026.09.12-26.2</release>`.
* The POM's own `<description>` is `"26.2 Lightweight Minecraft server"` — a direct, machine-readable confirmation of the intended MC version.
* **Minestom is a library, not a packaged server**: it tracks one MC version at a time (its `master` line); older MC versions are only reachable by pinning old releases (e.g. `2026.05.11-1.21.11` for 1.21.11). It is **not** an 1.8.9-capable platform.
* **Java:** Gradle Module Metadata for `2026.09.12-26.2` declares `org.gradle.jvm.version = 25` on `apiElements`/`runtimeElements`; built by Gradle `9.7.1`. So Minestom's current line requires **Java 25** (and a Gradle new enough to run on 25).
* Minestom is a *server platform you build a server with*, not a proxy.

### 4.6 Nukkit / PowerNukkitX — Bedrock, not Java Edition

* Cloudburst Nukkit's README: "Nukkit is nuclear-powered server software for **Minecraft Bedrock Edition**."
* PowerNukkitX's README: "A Minecraft **Bedrock Edition** Server Software, open source and written in java", with badges `minecraft v26.50 (Bedrock)` and `protocol 2193`.
* Availability sources:
  * PowerNukkitX official Maven repository `https://repo.powernukkitx.org/releases` (live; HTTP 200 at the root). Its `org.powernukkitx:server` artifact is **SNAPSHOT-only** — `<latest>nightly-SNAPSHOT</latest>`, **no `<release>` element**, 17 versions, all `*-SNAPSHOT` (`3.0.0-SNAPSHOT` … `3.0.5-SNAPSHOT`, `26_u4-SNAPSHOT`, `nightly-SNAPSHOT`). So the repo is *not* a stable version registry.
  * Maven Central carries only the stale line: `cn.powernukkitx:powernukkitx:1.20.40-r1` (Bedrock 1.20.40) and the older `org.powernukkit:powernukkit:1.6.0.1-PN`.
  * Upstream Cloudburst Nukkit downloads: `https://dl.opencollab.dev/nukkit`.
  * For release-grade versions, PowerNukkitX's GitHub Releases page is the intended source (`https://github.com/PowerNukkitX/PowerNukkitX/releases`) — **not fetchable from this workspace** (§8).
* **Java:** PowerNukkitX `build.gradle.kts` (master) sets `java.sourceCompatibility = JavaVersion.VERSION_25`, `targetCompatibility = VERSION_25`, `options.release.set(25)` and a Java 25 toolchain → **Java 25**.
* **Design consequence:** Nukkit must **not** share the Java-Edition MC-version axis. Its version axis is Bedrock build/protocol numbers (e.g. Bedrock `1.20.40`, `26.50`; protocol `2193`).

---

## 5. Gradle and Gradle plugin version floors

### 5.1 Gradle itself

* **Current stable: `9.8.0`** — `https://services.gradle.org/versions/current` reports `{"version":"9.8.0","buildTime":"20260924134000+0000","current":true}`, download `https://services.gradle.org/distributions/gradle-9.8.0-bin.zip`.
* Recent stable line: `9.6.0`, `9.6.1`, `9.7.0`, `9.7.1`, `9.8.0` (next milestone `9.9.0-milestone-2`).

**Java compatibility** (`https://docs.gradle.org/current/userguide/compatibility.html`, Table 1 — fetched for Gradle 9.8):

| Java version | Toolchain support | Support for **running** Gradle |
|---|---|---|
| 8 | N/A | 2.0 to 8.14.x |
| 9/10 | N/A | 4.3/4.7 to 8.14.x |
| 11 | N/A | 5.0 to 8.14.x |
| 12–14 | N/A | … to 8.14.x |
| 15 | 6.7 | 6.7 to 8.14.x |
| 16 | 7.0 | 7.0 to 8.14.x |
| **17** | 7.3 | **7.3 and after** |
| 18 | 7.5 | 7.5 and after |
| 19 | 7.6 | 7.6 and after |
| 20 | 8.1 | 8.3 and after |
| **21** | 8.4 | **8.5 and after** |
| 22 | 8.7 | 8.8 and after |
| 23 | 8.10 | 8.10 and after |
| 24 | 8.14 | 8.14 and after |
| **25** | **9.1.0** | **9.1.0 and after** |
| 26 | 9.4.0 | 9.4.0 and after |
| 27 | 9.8.0 | 9.8.0 and after |

Breakpoints that matter:
* **Gradle 9.x cannot run on Java 8/11** — its floor is Java **17** (the 8.14.x line is the last that runs on Java 8–24).
* To **build** with a Java **25** toolchain you need Gradle **9.1.0+**; Java 26 → 9.4.0+; Java 27 → 9.8.0+.
* Consequence for vinoa: a Java-25 plugin (Paper 26.1+) needs wrapper ≥ 9.1.0 — 9.8.0 is the safe current default. If vinoa ever supports 1.8.9–1.16.5 (Java 8/11) *builds*, only Gradle ≤ 8.14.x can run on those JDKs, but the Java toolchain/cross-compile path means the wrapper can still be 9.x provided the build JDK is ≥17.

### 5.2 `com.gradleup.shadow` (Shadow)

| Field | Value | Evidence |
|---|---|---|
| Current version | **`9.6.1`** | Gradle Plugin Portal marker metadata `<release>9.6.1</release>`; Maven Central `com.gradleup.shadow:shadow-gradle-plugin` `<release>9.6.1</release>` |
| **Gradle floor** | **`9.2.0`** | `shadow-gradle-plugin-9.6.1.module` → `apiElements.attributes["org.gradle.plugin.api-version"] = "9.2.0"` |
| **Java floor** | **17** | same `.module` → `org.gradle.jvm.version = 17`; `ShadowPlugin.class` bytecode major **61** |
| Changelog corroboration | "Bump min Gradle requirement to **9.2.0**" in release **9.5.0** (2026-07-06); "Bump min Gradle requirement to **9.0.0**" in **9.3.0** (2025-12-05); "Bump the min Gradle requirement from 8.0.0 to 8.3" in the 8.3.x line | `https://gradleup.com/shadow/changes/` |
| Last 8.x | **`8.3.11`** (`8.3.0`–`8.3.11`); `.module` declares `org.gradle.jvm.version = 8`, min Gradle **8.3**; from 8.3.2 the plugin itself needs **Java 11+** to run (jdependency 2.11 bump) | module metadata + changelog |
| Legacy id | `com.github.johnrengelman.shadow` — last version **`8.1.1`** | Plugin Portal marker metadata |
| Plugin id / marker | `com.gradleup.shadow` → `com.gradleup.shadow:com.gradleup.shadow.gradle.plugin` → implementation `com.gradleup.shadow:shadow-gradle-plugin`; `META-INF/gradle-plugins/com.gradleup.shadow.properties` → `implementation-class=com.github.jengelman.gradle.plugins.shadow.ShadowPlugin` (package kept for compatibility) | jar inspection |

### 5.3 `xyz.jpenilla.run-paper`

| Field | Value | Evidence |
|---|---|---|
| Current version | **`3.1.0`** | Plugin Portal marker `<release>3.1.0</release>` |
| Sibling ids, same artifact | `xyz.jpenilla.run-velocity` **3.1.0**, `xyz.jpenilla.run-waterfall` **3.1.0** | their marker metadata |
| Implementation artifact | `xyz.jpenilla:run-task:3.1.0` (all three plugin ids live in it, plus `RunPaperExtension$Folia` classes) | marker POM dependency |
| **Gradle floor** | **`9.7.0`** | `run-task-3.1.0.module` → `runtimeElements.attributes["org.gradle.plugin.api-version"] = "9.7.0"`, `createdBy.gradle.version = 9.7.0` |
| **Java floor** | **17** | `.module` → `org.gradle.jvm.version = 17`; plugin build sets `jvmToolchain(17)`, `jvmTarget = JVM_17`, `-Xjdk-release=17`; CI matrix `java: ["17"]` |
| Previous release | `3.0.0` declares `org.gradle.plugin.api-version = 8.14.3` (built with Gradle 8.14.3); `2.3.1` declares no api-version (built with Gradle 8.10) | `.module` files |
| dev snapshot | repo `version = 3.1.1-SNAPSHOT`, wrapper `gradle-9.7.0-bin.zip` | `plugin/build.gradle.kts`, `gradle/wrapper/gradle-wrapper.properties` |
| Relevant tasks/DSL | `runServer { minecraftVersion("1.21.8") }`; Velocity: `runVelocity { velocityVersion("3.4.0-SNAPSHOT") }`; a Folia mode exists (`RunPaperExtension$Folia$PluginsMode`) | README + jar classes |

Because the Gradle-API-version is an attribute on `runtimeElements`, Gradle's variant-aware plugin resolution **enforces** it: run-paper 3.1.0 will refuse to load on Gradle < 9.7.0. That is a hard, machine-readable floor — prefer it over README prose.

### 5.4 `io.papermc.paperweight.userdev`

| Field | Value | Evidence |
|---|---|---|
| **Recommended version** | **`2.0.0-beta.24`** — Paper docs inject `LATEST_USERDEV_RELEASE`, computed as *the newest GitHub tag of `PaperMC/paperweight`*: `const userdevVersions = await fetchGitHubTags("PaperMC/paperweight"); export const LATEST_USERDEV_RELEASE = userdevVersions[0];`. The Gradle Plugin Portal publishes betas up to **`2.0.0-beta.24`** (marker and `paperweight-userdev` metadata agree) | `PaperMC/docs` `src/utils/versions.ts` + `plugins.gradle.org/m2/io/papermc/paperweight/userdev/…` |
| Where betas live | **Gradle Plugin Portal only.** `repo.papermc.io` has **no** `2.0.0-beta.*` artifacts (all probed versions 404); its marker metadata tops out at `2.0.0-SNAPSHOT` / `2.0-SNAPSHOT` | probes of `repo.papermc.io` for `2.0.0-beta.21/24`, `2.0.0` |
| Last **stable** release on `repo.papermc.io` | `1.5.0` (`<release>1.5.0</release>`, `<latest>2.0.0-SNAPSHOT</latest>`); `.module` declares `org.gradle.jvm.version = 11`, built by Gradle `7.3.2` | Maven metadata + module |
| **Gradle floor (current beta)** | **`9.7.1`** | `paperweight-userdev-2.0.0-beta.24.module` → `shadowRuntimeElements.attributes["org.gradle.plugin.api-version"] = "9.7.1"`, `createdBy.gradle.version = 9.8.0` |
| **Java floor (current beta)** | **21** | same `.module` → `org.gradle.jvm.version = 21` |
| Docs-stated requirements | "Please make sure you are using the latest stable version of Gradle." / "The latest version of `paperweight-userdev` supports dev bundles for Minecraft **1.17.1 and newer**" / "Only the latest version of `paperweight-userdev` is officially supported" / "**paperweight-userdev** SNAPSHOT (pre-release) versions are only available through Paper's Maven repository" | `paper/dev/getting-started/userdev.md` |
| Usage | plugin id `io.papermc.paperweight.userdev`; dev-bundle dependency `paperweight.paperDevBundle("{VERSION}.build.+")`; plus `paperweight { javaLauncher = javaToolchains.launcherFor { languageVersion = JavaLanguageVersion.of(17) } }` as a workaround when a bundle rejects the configured toolchain | same doc |
| Version-specific behaviour | "From Minecraft version **26.1** onwards, Paper no longer supports obfuscated plugins"; "Since Minecraft version **26.1**, Paper no longer supports remapping your plugin to Spigot runtime mappings … reobfuscated plugins will not work from Paper 26.1 onwards"; the `reobfJar` task is **reference-only** from 26.1 | same doc |
| Interop | "If you have the shadow Gradle plugin applied in your build script, **paperweight-userdev** will [integrate]" | same doc |
| Modern alternative | None. Paper's docs' project-setup page presents `paper-api` (no internals) as the default and `paperweight-userdev` as the only supported route to server internals; there is no newer replacement plugin | `project-setup.mdx` + `userdev.md` |

### 5.5 Recommended floor matrix for generated builds

| Generated target | Gradle wrapper | Required JDK to *run* Gradle | Plugin versions |
|---|---|---|---|
| Paper 26.1+ (Java 25) | **9.8.0** | 17+ | `run-paper` 3.1.0, `shadow` 9.6.1 (only if shading) |
| Paper 1.20.5–1.21.11 (Java 21) | 9.8.0 | 17+ | same |
| Paper 1.18–1.20.4 (Java 17) | 9.8.0 | 17+ | same |
| Paper 1.17–1.17.1 (Java 16) / 1.9.4–1.16.5 (Java 8/11) | 8.14.x if the build JDK must be 8/11, else 9.8.0 with a toolchain | 8 (only ≤8.14.x) | `shadow` 8.3.11 if Gradle < 9.2.0; `run-paper` 3.0.0 if Gradle < 9.7.0 |
| NMS development | as above | as above | `io.papermc.paperweight.userdev` 2.0.0-beta.24 → needs Gradle **9.7.1+**, Java **21+** |

---

## 6. Platform × MC-version combinations that do NOT exist

Derived from the sources above; each row is a hard constraint the data file must encode.

### 6.1 Paper (Fill project `paper`)

* **No `1.8.9`** — Paper's oldest release is `1.8.8` (3 builds, all `STABLE`). 1.8.9 is a *server* release that Paper never built.
* No `1.16` (only `1.16.1`–`1.16.5`).
* No `1.20.3` (Paper has `1.20.2` and `1.20.4`).
* No `1.21.2` (Paper has `1.21.1` and `1.21.3`).
* No plain `26.1` (only `26.1.1`, `26.1.2`).
* Oldest 1.13/1.12/1.11/1.10/1.9 patches exist only as `1.13.x`, `1.12.2`, `1.11.2`, `1.10.2`, `1.9.4` — i.e. **most patches of 1.9–1.13 were never built**.
* `26.3` has **no `STABLE` build** (39 `ALPHA` builds only) → "newest Paper release" ≠ "newest MC release".

### 6.2 paper-api artifacts

* **No artifact at all for 1.8.x** (earliest is `1.9.4`).
* No artifact for `1.10`, `1.10.1`, `1.11.0`(as `1.11` it exists), `1.16` (only `1.16.1`+).
* No `io.papermc.paper:paper-api` below `1.17`.
* **No artifact for plain `26.1`** (only `26.1.1`, `26.1.2`).

### 6.3 Folia

* **Nothing before `1.19.4`** — Folia did not exist before regionised threading shipped with 1.19.4.
* Missing after that: `1.19`–`1.19.3`, `1.20` (bare), `1.20.3`, `1.20.5`, `1.21`, `1.21.1`, `1.21.2`, `1.21.3`, `1.21.7`, `1.21.9`, `1.21.10`, `26.3`.
* Present: `1.19.4`, `1.20.1`, `1.20.2`, `1.20.4`, `1.20.6`, `1.21.4`, `1.21.5`, `1.21.6`, `1.21.8`, `1.21.11`, `26.1.2`, `26.2`.

### 6.4 Velocity

* **Velocity versions are not MC versions.** A Velocity version supports a *protocol range* (`1.7.2 → 26.2` on the 3.x line, `1.7.2 → 26.3` on the 4.x line), so the combination "Velocity 3.5.1 × MC 1.12.2" is meaningless — `3.5.1` serves 1.12.2 clients *and* 26.2 clients.
* There is no "Velocity for MC X" artifact; the MC axis must be modelled as `protocol_min`/`protocol_max`, not a version list.
* No Velocity `3.5.0` build (Fill reports **0 builds**).

### 6.5 BungeeCord

* No per-patch artifacts: only `1.8`/`1.16-R0.5`/`1.21-R0.4`-style **major** strings. Any matrix cell like `1.20.6` is unimplementable.
* **No `26.2` and no `26.3`** artifact (latest is `26.1-R0.1-SNAPSHOT`).
* No `bungeecord` Fill project → the refresh path cannot use Fill for BungeeCord.

### 6.6 Sponge

* **No `1.13`, `1.14`** (Sponge skipped from SpongeAPI 7/1.12.2 straight to API 8/1.16.x).
* **No bare `1.18`** (only `1.18.1`, `1.18.2`), **no bare `1.19`**, **no `1.19.1`** (jumps `1.18.2` → `1.19.2`).
* No `1.20.3`, no `1.20.5` (jumps `1.20.2` → `1.20.4` → `1.20.6`).
* No `1.9`–`1.9.3` beyond `1.9`/`1.9.4`; no `1.11.1`; no `1.12`/`1.12.1` (only `1.12.2` in the API-7 line — `1.12`/`1.12.1` exist only as early builds).
* SpongeAPI 18/19/20/21 exist only as `-SNAPSHOT`/`-RC` lines for MC 1.21.11/26.1/26.2/26.3 while `<release>` is `17.0.0`.

### 6.7 Minestom

* **Nothing before `1.21.11` in the current published window.** The Maven metadata's observed MC tags are `1.21.11`, `26.1.1`, `26.1.2`, `26.2` only. Minestom historically targeted recent versions only; there is no 1.8.9/1.12.2/1.16.5 Minestom on Central.
* Minestom versions are *dated snapshots* of one MC version at a time — there is no "Minestom 1.20.6 LTS" concept.

### 6.8 Nukkit

* **Every "Nukkit × MC 1.x" cell is invalid**: Nukkit is Bedrock. The valid axis is Bedrock protocol versions (PowerNukkitX README: Bedrock `26.50`, protocol `2193`; Central's newest is Bedrock `1.20.40`).
* No stable `org.powernukkitx:*` **release** artifact on their Maven repo — `<release>` is absent; only `nightly-SNAPSHOT`/`3.0.x-SNAPSHOT`.

### 6.9 Cross-cutting: Java × platform

* Any platform build whose `java_min` is 25 (Paper `26.1+`, Folia `26.1+`, Velocity 4.x, Minestom current, PowerNukkitX) **cannot** be built or run on a JDK < 25, and needs **Gradle ≥ 9.1.0** to use the toolchain.
* `1.8.9` has **no** `paper-api` and **no** Sponge/Minestom/Folia artifact → the only viable 1.8.9 targets are Paper `1.8.8`-era servers via non-Paper APIs, or BungeeCord/Velocity as a *proxy* in front of a 1.8.9 server.

---

## 7. Machine-readable breakpoint table

One row per Minecraft **release** version from `1.8.9` to `26.3` (66 rows), per Mojang's `piston-meta` manifest.

Column semantics (all machine-parseable):
* `paper_builds` / `folia_builds` — `yes`/`NO`: does the platform publish any build for this exact version id? (`fill.papermc.io/v3/projects/{paper,folia}`)
* `paper_java_min` — the Java floor, from `fill` `version.java.version.minimum`. For the 55 Paper versions this is the **fetched** value (every Paper version was swept; the plateaus are flat). For the 11 MC releases that Paper never built (`1.9`–`1.9.3`, `1.10`, `1.10.1`, `1.11`, `1.11.1`, `1.16`, `1.20.3`, `1.21.2`, `26.1`) it is the value of the enclosing verified plateau. Recommended Java for all of them is available from the docs table in §3.1.
* `paper_java_recommended` — Paper docs' "Recommended Java Version" for the range containing this version.
* `sponge_api` — highest `org.spongepowered:spongeapi` version whose SpongeVanilla build targets this MC version (`-` = none).
* `paper_api` — the exact Maven coordinate to declare, or `NONE` (see §2.2/§2.3).

| mc_version | paper_builds | folia_builds | paper_java_min | paper_java_recommended | sponge_api | paper_api_coordinate |
|---|---|---|---|---|---|---|
| `1.8.9` | NO | NO | `8` | `8` | 4.2.0 | NONE |
| `1.9` | NO | NO | `8` | `8` | 5.0.0 | NONE |
| `1.9.1` | NO | NO | `8` | `8` | - | NONE |
| `1.9.2` | NO | NO | `8` | `8` | - | NONE |
| `1.9.3` | NO | NO | `8` | `8` | - | NONE |
| `1.9.4` | yes | NO | `8` | `8` | 5.0.0 | com.destroystokyo.paper:paper-api:1.9.4-R0.1-SNAPSHOT |
| `1.10` | NO | NO | `8` | `8` | - | NONE |
| `1.10.1` | NO | NO | `8` | `8` | - | NONE |
| `1.10.2` | yes | NO | `8` | `8` | 5.2.0 | com.destroystokyo.paper:paper-api:1.10.2-R0.1-SNAPSHOT |
| `1.11` | NO | NO | `8` | `8` | 6.0.0 | com.destroystokyo.paper:paper-api:1.11-R0.1-SNAPSHOT |
| `1.11.1` | NO | NO | `8` | `8` | - | com.destroystokyo.paper:paper-api:1.11.1-R0.1-SNAPSHOT |
| `1.11.2` | yes | NO | `8` | `8` | 6.1.0 | com.destroystokyo.paper:paper-api:1.11.2-R0.1-SNAPSHOT |
| `1.12` | yes | NO | `8` | `11` | 7.0.0 | com.destroystokyo.paper:paper-api:1.12-R0.1-SNAPSHOT |
| `1.12.1` | yes | NO | `8` | `11` | 7.0.0 | com.destroystokyo.paper:paper-api:1.12.1-R0.1-SNAPSHOT |
| `1.12.2` | yes | NO | `8` | `11` | 7.4.8 | com.destroystokyo.paper:paper-api:1.12.2-R0.1-SNAPSHOT |
| `1.13` | yes | NO | `8` | `11` | - | com.destroystokyo.paper:paper-api:1.13-R0.1-SNAPSHOT |
| `1.13.1` | yes | NO | `8` | `11` | - | com.destroystokyo.paper:paper-api:1.13.1-R0.1-SNAPSHOT |
| `1.13.2` | yes | NO | `8` | `11` | - | com.destroystokyo.paper:paper-api:1.13.2-R0.1-SNAPSHOT |
| `1.14` | yes | NO | `8` | `11` | - | com.destroystokyo.paper:paper-api:1.14-R0.1-SNAPSHOT |
| `1.14.1` | yes | NO | `8` | `11` | - | com.destroystokyo.paper:paper-api:1.14.1-R0.1-SNAPSHOT |
| `1.14.2` | yes | NO | `8` | `11` | - | com.destroystokyo.paper:paper-api:1.14.2-R0.1-SNAPSHOT |
| `1.14.3` | yes | NO | `8` | `11` | - | com.destroystokyo.paper:paper-api:1.14.3-R0.1-SNAPSHOT |
| `1.14.4` | yes | NO | `8` | `11` | - | com.destroystokyo.paper:paper-api:1.14.4-R0.1-SNAPSHOT |
| `1.15` | yes | NO | `8` | `11` | - | com.destroystokyo.paper:paper-api:1.15-R0.1-SNAPSHOT |
| `1.15.1` | yes | NO | `8` | `11` | - | com.destroystokyo.paper:paper-api:1.15.1-R0.1-SNAPSHOT |
| `1.15.2` | yes | NO | `8` | `11` | 8.0.0 | com.destroystokyo.paper:paper-api:1.15.2-R0.1-SNAPSHOT |
| `1.16` | NO | NO | `8` | `11` | - | NONE |
| `1.16.1` | yes | NO | `8` | `11` | - | com.destroystokyo.paper:paper-api:1.16.1-R0.1-SNAPSHOT |
| `1.16.2` | yes | NO | `8` | `11` | - | com.destroystokyo.paper:paper-api:1.16.2-R0.1-SNAPSHOT |
| `1.16.3` | yes | NO | `8` | `11` | - | com.destroystokyo.paper:paper-api:1.16.3-R0.1-SNAPSHOT |
| `1.16.4` | yes | NO | `8` | `11` | 8.0.0 | com.destroystokyo.paper:paper-api:1.16.4-R0.1-SNAPSHOT |
| `1.16.5` | yes | NO | `8` | `16` | 8.2.1 | com.destroystokyo.paper:paper-api:1.16.5-R0.1-SNAPSHOT |
| `1.17` | yes | NO | `16` | `17` | 9.0.0 | io.papermc.paper:paper-api:1.17-R0.1-SNAPSHOT |
| `1.17.1` | yes | NO | `16` | `17` | 9.0.0 | io.papermc.paper:paper-api:1.17.1-R0.1-SNAPSHOT |
| `1.18` | yes | NO | `17` | `17` | - | io.papermc.paper:paper-api:1.18-R0.1-SNAPSHOT |
| `1.18.1` | yes | NO | `17` | `17` | 9.0.0 | io.papermc.paper:paper-api:1.18.1-R0.1-SNAPSHOT |
| `1.18.2` | yes | NO | `17` | `17` | 9.0.0 | io.papermc.paper:paper-api:1.18.2-R0.1-SNAPSHOT |
| `1.19` | yes | NO | `17` | `17` | - | io.papermc.paper:paper-api:1.19-R0.1-SNAPSHOT |
| `1.19.1` | yes | NO | `17` | `17` | - | io.papermc.paper:paper-api:1.19.1-R0.1-SNAPSHOT |
| `1.19.2` | yes | NO | `17` | `17` | 10.0.0 | io.papermc.paper:paper-api:1.19.2-R0.1-SNAPSHOT |
| `1.19.3` | yes | NO | `17` | `17` | 10.0.0 | io.papermc.paper:paper-api:1.19.3-R0.1-SNAPSHOT |
| `1.19.4` | yes | yes | `17` | `17` | 10.0.0 | io.papermc.paper:paper-api:1.19.4-R0.1-SNAPSHOT |
| `1.20` | yes | NO | `17` | `21` | 11.0.0 | io.papermc.paper:paper-api:1.20-R0.1-SNAPSHOT |
| `1.20.1` | yes | yes | `17` | `21` | 11.0.0 | io.papermc.paper:paper-api:1.20.1-R0.1-SNAPSHOT |
| `1.20.2` | yes | yes | `17` | `21` | 11.0.0 | io.papermc.paper:paper-api:1.20.2-R0.1-SNAPSHOT |
| `1.20.3` | NO | NO | `17` | `21` | - | io.papermc.paper:paper-api:1.20.3-R0.1-SNAPSHOT |
| `1.20.4` | yes | yes | `17` | `21` | 11.0.0 | io.papermc.paper:paper-api:1.20.4-R0.1-SNAPSHOT |
| `1.20.5` | yes | NO | `21` | `21` | - | io.papermc.paper:paper-api:1.20.5-R0.1-SNAPSHOT |
| `1.20.6` | yes | yes | `21` | `21` | 11.0.1 | io.papermc.paper:paper-api:1.20.6-R0.1-SNAPSHOT |
| `1.21` | yes | NO | `21` | `21` | 12.0.0 | io.papermc.paper:paper-api:1.21-R0.1-SNAPSHOT |
| `1.21.1` | yes | NO | `21` | `21` | 12.0.4 | io.papermc.paper:paper-api:1.21.1-R0.1-SNAPSHOT |
| `1.21.2` | NO | NO | `21` | `21` | 13.0.0 | NONE |
| `1.21.3` | yes | NO | `21` | `21` | 13.0.1 | io.papermc.paper:paper-api:1.21.3-R0.1-SNAPSHOT |
| `1.21.4` | yes | yes | `21` | `21` | 14.0.1 | io.papermc.paper:paper-api:1.21.4-R0.1-SNAPSHOT |
| `1.21.5` | yes | yes | `21` | `21` | 15.0.1 | io.papermc.paper:paper-api:1.21.5-R0.1-SNAPSHOT |
| `1.21.6` | yes | yes | `21` | `21` | 16.0.0 | io.papermc.paper:paper-api:1.21.6-R0.1-SNAPSHOT |
| `1.21.7` | yes | NO | `21` | `21` | 16.0.0 | io.papermc.paper:paper-api:1.21.7-R0.1-SNAPSHOT |
| `1.21.8` | yes | yes | `21` | `21` | 16.0.1 | io.papermc.paper:paper-api:1.21.8-R0.1-SNAPSHOT |
| `1.21.9` | yes | NO | `21` | `21` | 17.0.0 | io.papermc.paper:paper-api:1.21.9-R0.1-SNAPSHOT |
| `1.21.10` | yes | NO | `21` | `21` | 17.0.1 | io.papermc.paper:paper-api:1.21.10-R0.1-SNAPSHOT |
| `1.21.11` | yes | yes | `21` | `21` | 18.0.0 | io.papermc.paper:paper-api:1.21.11-R0.1-SNAPSHOT |
| `26.1` | NO | NO | `25` | `25` | 19.0.0 | NONE |
| `26.1.1` | yes | NO | `25` | `25` | 19.0.0 | io.papermc.paper:paper-api:26.1.1.build.+ |
| `26.1.2` | yes | yes | `25` | `25` | 19.0.0 | io.papermc.paper:paper-api:26.1.2.build.+ |
| `26.2` | yes | yes | `25` | `25` | 20.0.0 | io.papermc.paper:paper-api:26.2.build.+ |
| `26.3` | yes | NO | `25` | `25` | 21.0.0 | io.papermc.paper:paper-api:26.3.build.+ |

### 7.1 Non-negotiable breakpoint list (the compact form for code)

```
java_min:            8  for 1.8.9 .. 1.16.5
                    16  for 1.17  .. 1.17.1
                    17  for 1.18  .. 1.20.4
                    21  for 1.20.5 .. 1.21.11
                    25  for 26.1  .. (current)
java_recommended:    8  for <=1.11     11 for 1.12 .. 1.16.4     16 for 1.16.5
                    17  for 1.17 .. 1.19                        21 for 1.20 .. 1.21.11
                    25  for >=26.1
paper_api_namespace: com.destroystokyo.paper  for 1.9.4 .. 1.16.5
                    io.papermc.paper          for 1.17  .. current
paper_api_string:    {MC}-R0.1-SNAPSHOT       for 1.9.4 .. 1.21.11
                    {MC}.build.+              for 26.1.1 .. current
paper_api_missing:   1.8.x (none), 1.16 (bare), 1.10, 1.10.1, 26.1 (bare)
folia_min:           1.19.4
velocity_protocol:   min 1.7.2 (always) ; max 26.2 on 3.x, 26.3 on 4.x
bungeecord_grain:    major version only (1.8, 1.16-R0.5, 1.21-R0.4, 26.1-R0.1) ; max = 26.1
sponge_gap:          no 1.13, 1.14, 1.18, 1.19, 1.19.1, 1.20.3, 1.20.5
minestom_window:     1.21.11, 26.1.1, 26.1.2, 26.2 (only)
nukkit_axis:         Bedrock protocol, NOT Java Edition MC versions
gradle_current:      9.8.0          gradle_min_for_java25_toolchain: 9.1.0   gradle_min_to_run: java 17
shadow_gradle_floor: 9.2.0 (shadow 9.6.1) ; java 17
runpaper_gradle_floor: 9.7.0 (run-paper 3.1.0) ; java 17
paperweight_floor:   9.7.1 / java 21 (io.papermc.paperweight.userdev 2.0.0-beta.24)
```

---

## 8. Confidence / unverified

* **Unverified — PowerNukkitX per-release Bedrock versions.** `github.com` HTML and `api.github.com` are unreachable from this workspace (timeout / HTTP 403) and `web_search` is disabled, so the releases list at `https://github.com/PowerNukkitX/PowerNukkitX/releases` was not read. The Bedrock target is attested by the README badges (`minecraft v26.50 (Bedrock)`, `protocol 2193`) and the official Maven repository is live, but the *full* version→Bedrock mapping is not captured here.
* **Unverified — Cloudburst Nukkit version list.** `https://repo.opencollab.dev/` is a Reposilite instance whose browse API paths tried (`/main/`, `/api/repositories`, `/main/org/cloudburstmc/nukkit/`) all 404, and the README only points at `https://dl.opencollab.dev/nukkit`. Whether Cloudburst Nukkit still publishes artifacts is therefore unresolved.
* **Partially verified — Sponge API-version doc page.** `docs.spongepowered.org/stable/en/plugin/api-versions.html` renders a table only up to SpongeAPI **14.0.0** (it is built from an older docs revision), while Maven has `spongeapi` `<release>17.0.0</release>` and SpongeVanilla builds for SpongeAPI 18–21. §4.4 therefore uses the **Maven metadata** as the primary mapping and the docs page as corroboration of the naming scheme.
* **Velocity's `MINIMUM_VERSION = 1.7.2`** is read from `ProtocolVersion.java` on branches `dev/3.0.0` and `dev/4.0.0`. I did not verify that the handshake handler does not impose a stricter runtime minimum elsewhere; treat `1.7.2` as the *declared* minimum.
* **`repo.papermc.io` legacy metadata is inconsistent** (§2.3). The legacy `com.destroystokyo.paper` range is taken from the repository browse listing plus per-version `maven-metadata.xml` files (both 200 with real `<value>` entries), not from the aggregate metadata file.
* **Gradle Module Metadata floors** (`org.gradle.plugin.api-version`, `org.gradle.jvm.version`) are the strongest available evidence for plugin floors, but they are *declared* values — a plugin can still work on slightly older Gradle in practice. The docs/changelogs in §5.2/§5.4 corroborate them.
* **Channel `RECOMMENDED`** appears in PaperMC's docs TypeScript union but was never observed in a live response; the data file should tolerate it but not expect it.
* **Timestamps:** all endpoints were fetched 2026-09-25. Fill's `26.3` is pre-release at that time (39 `ALPHA` builds); `26.2` is the newest Paper version with `STABLE` builds.

---

## Sources

Minecraft / Mojang:
* <https://piston-meta.mojang.com/mc/game/version_manifest_v2.json>
* <https://github.com/PaperMC/docs/blob/main/src/utils/versions.ts>

PaperMC Fill API v3:
* <https://fill.papermc.io/v3/projects>
* <https://fill.papermc.io/v3/projects/paper>
* <https://fill.papermc.io/v3/projects/paper/versions/1.21.4>
* <https://fill.papermc.io/v3/projects/paper/versions/1.21.4/builds>
* <https://fill.papermc.io/v3/projects/paper/versions/1.21.4/builds?channel=STABLE>
* <https://fill.papermc.io/v3/projects/paper/versions/1.21.4/builds/latest>
* <https://fill.papermc.io/v3/projects/paper/versions/1.21.4/builds/latest?channel=STABLE>
* <https://fill.papermc.io/v3/projects/paper/versions/26.3/builds>
* <https://fill.papermc.io/v3/projects/paper/versions/26.2/builds>
* <https://fill.papermc.io/v3/projects/paper/versions/1.21.11/builds>
* <https://fill.papermc.io/v3/projects/paper/versions/1.12.2/builds>
* <https://fill.papermc.io/v3/projects/paper/versions/1.8.8>
* <https://fill.papermc.io/v3/projects/folia>
* <https://fill.papermc.io/v3/projects/folia/versions/1.19.4>
* <https://fill.papermc.io/v3/projects/velocity>
* <https://fill.papermc.io/v3/projects/velocity/versions/3.5.1>
* <https://fill.papermc.io/v3/projects/velocity/versions/4.0.0>
* <https://fill.papermc.io/v3/projects/velocity/versions/4.2.0>
* <https://fill.papermc.io/v3/projects/waterfall>
* <https://fill.papermc.io/v3/projects/waterfall/versions/1.21>
* <https://fill.papermc.io/swagger-ui/index.html>
* <https://docs.papermc.io/misc/downloads-service/>

PaperMC docs (rendered + Markdown source):
* <https://docs.papermc.io/paper/getting-started/>
* <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/admin/getting-started/getting-started.mdx>
* <https://docs.papermc.io/paper/dev/project-setup/>
* <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/getting-started/project-setup.mdx>
* <https://docs.papermc.io/paper/dev/userdev/>
* <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/getting-started/userdev.md>
* <https://docs.papermc.io/velocity/>
* <https://docs.papermc.io/velocity/getting-started/>
* <https://docs.papermc.io/waterfall/>

PaperMC Maven / Nexus:
* <https://repo.papermc.io/repository/maven-public/io/papermc/paper/paper-api/maven-metadata.xml>
* <https://repo.papermc.io/repository/maven-public/io/papermc/paper/paper-api/>
* <https://repo.papermc.io/repository/maven-public/io/papermc/paper/paper-api/1.20.3-R0.1-SNAPSHOT/maven-metadata.xml>
* <https://repo.papermc.io/repository/maven-public/com/destroystokyo/paper/paper-api/>
* <https://repo.papermc.io/repository/maven-public/com/destroystokyo/paper/paper-api/maven-metadata.xml>
* <https://repo.papermc.io/repository/maven-public/com/destroystokyo/paper/paper-api/1.12.2-R0.1-SNAPSHOT/maven-metadata.xml>
* <https://repo.papermc.io/repository/maven-public/com/destroystokyo/paper/paper-api/1.16.5-R0.1-SNAPSHOT/maven-metadata.xml>
* <https://repo.papermc.io/repository/maven-public/com/destroystokyo/paper/paper-api/1.8.9-R0.1-SNAPSHOT/maven-metadata.xml> (404)
* <https://repo.papermc.io/repository/maven-public/io/papermc/paperweight/paperweight-userdev/maven-metadata.xml>
* <https://repo.papermc.io/repository/maven-public/io/papermc/paperweight/userdev/io.papermc.paperweight.userdev.gradle.plugin/maven-metadata.xml>
* <https://repo.papermc.io/repository/maven-public/io/papermc/paperweight/paperweight-userdev/1.5.0/paperweight-userdev-1.5.0.module>
* <https://api.papermc.io/v2/> (dead — 404)

Velocity / Waterfall sources:
* <https://raw.githubusercontent.com/PaperMC/Velocity/dev/3.0.0/api/src/main/java/com/velocitypowered/api/network/ProtocolVersion.java>
* <https://raw.githubusercontent.com/PaperMC/Velocity/dev/3.0.0/proxy/src/main/java/com/velocitypowered/proxy/protocol/StateRegistry.java>
* <https://raw.githubusercontent.com/PaperMC/Velocity/dev/4.0.0/api/src/main/java/com/velocitypowered/api/network/ProtocolVersion.java>
* <https://raw.githubusercontent.com/PaperMC/Velocity/dev/4.0.0/gradle.properties>
* <https://raw.githubusercontent.com/PaperMC/Velocity/dev/3.0.0/README.md>

BungeeCord / SpigotMC:
* <https://hub.spigotmc.org/nexus/content/groups/public/net/md-5/bungeecord-api/maven-metadata.xml>
* <https://hub.spigotmc.org/nexus/service/rest/v1/search?name=bungeecord-api>
* <https://hub.spigotmc.org/jenkins/job/BungeeCord/api/json>
* <https://raw.githubusercontent.com/SpigotMC/BungeeCord/master/README.md>
* <https://raw.githubusercontent.com/SpigotMC/BungeeCord/master/pom.xml>

Sponge:
* <https://repo.spongepowered.org/repository/maven-public/org/spongepowered/spongeapi/maven-metadata.xml>
* <https://repo.spongepowered.org/repository/maven-public/org/spongepowered/spongevanilla/maven-metadata.xml>
* <https://repo.spongepowered.org/repository/maven-public/org/spongepowered/spongeforge/maven-metadata.xml>
* <https://repo.spongepowered.org/repository/maven-public/org/spongepowered/spongeapi/17.0.0/spongeapi-17.0.0.jar> (class major 65)
* <https://docs.spongepowered.org/stable/en/plugin/api-versions.html>
* <https://docs.spongepowered.org/stable/en/versions/versioning.html>
* <https://docs.spongepowered.org/stable/en/versions/index.html>
* <https://docs.spongepowered.org/stable/en/index.html>

Minestom:
* <https://repo1.maven.org/maven2/net/minestom/minestom/maven-metadata.xml>
* <https://repo1.maven.org/maven2/net/minestom/minestom/2026.09.12-26.2/minestom-2026.09.12-26.2.pom>
* <https://repo1.maven.org/maven2/net/minestom/minestom/2026.09.12-26.2/minestom-2026.09.12-26.2.module>

Nukkit / PowerNukkitX:
* <https://raw.githubusercontent.com/CloudburstMC/Nukkit/master/README.md>
* <https://raw.githubusercontent.com/PowerNukkitX/PowerNukkitX/master/README.md>
* <https://raw.githubusercontent.com/PowerNukkitX/PowerNukkitX/master/build.gradle.kts>
* <https://raw.githubusercontent.com/PowerNukkitX/PowerNukkitX/master/gradle.properties>
* <https://repo.powernukkitx.org/releases/org/powernukkitx/server/maven-metadata.xml>
* <https://dl.opencollab.dev/nukkit>
* <https://search.maven.org/solrsearch/select?q=powernukkitx&rows=20&wt=json>
* <https://repo1.maven.org/maven2/org/powernukkit/powernukkit/maven-metadata.xml>

Gradle:
* <https://services.gradle.org/versions/current>
* <https://services.gradle.org/versions/all>
* <https://docs.gradle.org/current/userguide/compatibility.html>

Gradle plugins:
* <https://plugins.gradle.org/m2/com/gradleup/shadow/com.gradleup.shadow.gradle.plugin/maven-metadata.xml>
* <https://repo1.maven.org/maven2/com/gradleup/shadow/shadow-gradle-plugin/maven-metadata.xml>
* <https://repo1.maven.org/maven2/com/gradleup/shadow/shadow-gradle-plugin/9.6.1/shadow-gradle-plugin-9.6.1.module>
* <https://plugins.gradle.org/m2/com/gradleup/shadow/shadow-gradle-plugin/8.3.11/shadow-gradle-plugin-8.3.11.module>
* <https://plugins.gradle.org/m2/com/github/johnrengelman/shadow/com.github.johnrengelman.shadow.gradle.plugin/maven-metadata.xml>
* <https://gradleup.com/shadow/>
* <https://gradleup.com/shadow/changes/>
* <https://raw.githubusercontent.com/GradleUp/shadow/main/README.md>
* <https://plugins.gradle.org/m2/xyz/jpenilla/run-paper/xyz.jpenilla.run-paper.gradle.plugin/maven-metadata.xml>
* <https://plugins.gradle.org/m2/xyz/jpenilla/run-paper/xyz.jpenilla.run-paper.gradle.plugin/3.1.0/xyz.jpenilla.run-paper.gradle.plugin-3.1.0.pom>
* <https://plugins.gradle.org/m2/xyz/jpenilla/run-task/3.1.0/run-task-3.1.0.module>
* <https://plugins.gradle.org/m2/xyz/jpenilla/run-task/3.0.0/run-task-3.0.0.module>
* <https://plugins.gradle.org/m2/xyz/jpenilla/run-task/2.3.1/run-task-2.3.1.module>
* <https://plugins.gradle.org/m2/xyz/jpenilla/run-velocity/xyz.jpenilla.run-velocity.gradle.plugin/maven-metadata.xml>
* <https://plugins.gradle.org/m2/xyz/jpenilla/run-waterfall/xyz.jpenilla.run-waterfall.gradle.plugin/maven-metadata.xml>
* <https://raw.githubusercontent.com/jpenilla/run-paper/master/README.md>
* <https://raw.githubusercontent.com/jpenilla/run-paper/master/plugin/build.gradle.kts>
* <https://raw.githubusercontent.com/jpenilla/run-paper/master/gradle/libs.versions.toml>
* <https://raw.githubusercontent.com/jpenilla/run-paper/master/gradle/wrapper/gradle-wrapper.properties>
* <https://raw.githubusercontent.com/jpenilla/run-paper/master/settings.gradle.kts>
* <https://raw.githubusercontent.com/jpenilla/run-paper/master/.github/workflows/build.yml>
* <https://plugins.gradle.org/m2/io/papermc/paperweight/userdev/io.papermc.paperweight.userdev.gradle.plugin/maven-metadata.xml>
* <https://plugins.gradle.org/m2/io/papermc/paperweight/paperweight-userdev/maven-metadata.xml>
* <https://plugins.gradle.org/m2/io/papermc/paperweight/paperweight-userdev/2.0.0-beta.24/paperweight-userdev-2.0.0-beta.24.module>
