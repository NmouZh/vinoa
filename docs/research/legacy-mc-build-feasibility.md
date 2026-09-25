# Legacy (MC 1.8.9–1.16.5) Build Feasibility for a Single vinoa Skeleton

**Research date:** 2026-09-25.
**Ticket:** vinoa wayfinder map #8 — "老版本（1.8.9–1.16.5）构建可行性".
**Question:** can ONE skeleton (Gradle Kotlin DSL, `gradle/libs.versions.toml`, common + platform modules) serve both the version floor (MC 1.8.9) and the ceiling (MC 26.2 / Java 25) via Java toolchains and capability switches, or are two template sets unavoidable?

**Method / evidence rules.** Every claim is traced to a primary source: Gradle's own user manual, the published artifacts themselves (`maven-metadata.xml`, POMs, JARs fetched from the owning repositories — bytecode major version read directly out of the class files), the APIs' documented schemas, Mojang's `piston-meta` version manifest, and PaperMC's Fill API. Blogs, tutorials, and Stack Overflow are not used. Where a claim was verified by *executing* Gradle locally, the command and its output are quoted.

> **Environment caveats that affect citations (read before trusting a "not verified" line).**
> - **Gradle 9.8 binary could not be downloaded** in this environment: `services.gradle.org`/`downloads.gradle.org` transfer of the ~140 MB distribution aborted after ~2 min (HTTP 307 then timeout/connection reset). All *empirical* Gradle runs therefore used **Gradle 9.7.1** (the distribution already present locally). Policy claims are cited from the **9.8.0** docs (`docs.gradle.org/current` renders as "version 9.8.0"); 9.7.1 and 9.8.0 share the same compatibility policy.
> - **No JDK 8 could be downloaded** (Adoptium's API responds but its binaries redirect to `github.com`, which is unreachable here — `HTTP 000`; the foojay redirect timed out). The Java-8 *toolchain* path could therefore not be executed end-to-end; only the `options.release = 8` / `sourceCompatibility` paths were executed. See §2.4.
> - **`spigotmc.org` and the HTML side of `hub.spigotmc.org` return HTTP 403** from this network, so Spigot's ToS/wiki/BuildTools docs could not be read. Spigot *artifact* claims come from the Nexus repositories and the artifacts themselves (those work fine).
> - **`github.com` is unreachable (HTTP 000) and `api.github.com` returns 403**; `raw.githubusercontent.com` works for some repos and 404s for others. Where source reading was needed, `raw.githubusercontent.com` was used; some files (notably all of `jpenilla/run-task`) could not be read, so those conclusions come from the published plugin JAR's constant pool.

---

## 0. TL;DR — verdict

**One skeleton with capability switches is feasible and is the right call — provided the "classic" module is a genuinely separate compilation unit with its own Java target, dependency, and metadata, rather than the same source compiled twice.** Two full template sets are *not* required. The reason is that every blocker found is a *per-module property* (toolchain, dependency coordinate, `plugin.yml` field, run task), not a property of the project or of Gradle.

The four hard, verified floors that the skeleton must encode:

| # | Floor | Evidence |
|---|---|---|
| 1 | **There is no `spigot-api` for MC 1.8.9.** The last 1.8.x API artifact is `1.8.8-R0.1-SNAPSHOT`. | `…/spigot-api/1.8.9-R0.1-SNAPSHOT/maven-metadata.xml` → **404**; `…/1.8.8-R0.1-SNAPSHOT/maven-metadata.xml` → **200** |
| 2 | **There is no Paper API below 1.17** under `io.papermc.paper:paper-api`; legacy Paper API is `com.destroystokyo.paper:paper-api`, and that has no 1.8.x at all. | `io.papermc.paper:paper-api` metadata's oldest version = `1.17-R0.1-SNAPSHOT`; `com.destroystokyo…/1.8.8-R0.1-SNAPSHOT` → 404 |
| 3 | **`paperweight-userdev` cannot touch 1.8.9–1.16.5.** The oldest dev-bundle is `1.17.1-R0.1-SNAPSHOT`. | `repo.papermc.io/…/io/papermc/paper/dev-bundle/maven-metadata.xml` (291 versions, oldest 1.17.1) |
| 4 | **Gradle 9.8 cannot *run* on Java 8** (JVM 17–27 required) — but it *can* compile **for** Java 8 and consume Java 6/8-era artifacts. | Gradle compat page: "A JVM version between 17 and 27 is required to execute Gradle"; **verified by running** a Java 8-targeted compile against `spigot-api:1.8.8` |

Recommended skeleton shape and the switch set are in §6.

---

## 1. Legacy `spigot-api` / `paper-api` coordinates and repositories

### 1.1 `org.spigotmc:spigot-api` — exact strings

Read from the owning repository's maven-metadata (all 82 published versions enumerated, unauthenticated).

| Target MC | Exact coordinate to use | Notes |
|---|---|---|
| **1.8.9** | `org.spigotmc:spigot-api:1.8.8-R0.1-SNAPSHOT` | **There is no `1.8.9` artifact.** The 1.8.x line in the repository ends at `1.8.8-R0.1-SNAPSHOT` (published 2016-02-21, build 43). MC 1.8.9 keeps the 1.8.8 API — there is no separate API version for it. |
| **1.12.2** | `org.spigotmc:spigot-api:1.12.2-R0.1-SNAPSHOT` | published 2018-07-12, build 156 |
| **1.16.5** | `org.spigotmc:spigot-api:1.16.5-R0.1-SNAPSHOT` | published 2021-06-11, build 99 |

Complete legacy 1.8.x–1.16.x list observed (useful for a version→artifact table):

```
1.8 1.8.3 1.8.4 1.8.5 1.8.6 1.8.7 1.8.8   ← no 1.8.9
1.9 1.9.2 1.9.4 1.10 1.10.2 1.11 1.11.1 1.11.2
1.12-pre2 … 1.12 1.12.1 1.12.2
1.13-pre7 1.13 1.13.1 1.13.2 1.14-pre5 1.14 1.14.1 1.14.2 1.14.3 1.14.4
1.15 1.15.1 1.15.2 1.16.1 1.16.2 1.16.3 1.16.4 1.16.5
```

The repository is still live in 2026 and still publishes for current MC: the newest entry is
`26.3-R0.1-SNAPSHOT` (`lastUpdated 20260923104027`).

**Bytecode of the legacy artifacts** (read from the real JARs — this is the authoritative statement of "what Java version is this API"):

| Artifact | `org/bukkit/Bukkit.class` major | Java target | Manifest |
|---|---|---|---|
| `spigot-api:1.8.8-R0.1-SNAPSHOT` | **50** | Java 6 | `Build-Jdk: 1.7.0_72`, `Created-By: Apache Maven 3.2.2` |
| `spigot-api:1.12.2-R0.1-SNAPSHOT` | **51** | Java 7 | — |
| `spigot-api:1.13-R0.1-SNAPSHOT` | **51** | Java 7 | — |
| `spigot-api:1.16.5-R0.1-SNAPSHOT` | **52** | Java 8 | `Automatic-Module-Name: org.bukkit` |

The 1.8.8 POM corroborates: `<maven.compiler.source>1.6</maven.compiler.source>`, `<maven.compiler.target>1.6</maven.compiler.target>`.

### 1.2 `paper-api` — the coordinate changed, and legacy coverage is thin

**`io.papermc.paper:paper-api` on `repo.papermc.io` starts at MC 1.17.** The oldest version in the group metadata is `1.17-R0.1-SNAPSHOT`. Consequently **`io.papermc.paper:paper-api:1.16.5-R0.1-SNAPSHOT` does not exist** (version-directory request → 404) — a coordinate that many older tutorials still use and that will silently fail a scaffolded build.

Legacy Paper API lives under the **old group** `com.destroystokyo.paper:paper-api`:

| Coordinate | Resolvable? | Observed build |
|---|---|---|
| `com.destroystokyo.paper:paper-api:1.16.5-R0.1-SNAPSHOT` | **yes** (200) | `1.16.5-R0.1-20211218.082619-371` |
| `com.destroystokyo.paper:paper-api:1.12.2-R0.1-SNAPSHOT` | **yes** (200) | `1.12.2-R0.1-20190714.184133-413` |
| `com.destroystokyo.paper:paper-api:1.8.8-R0.1-SNAPSHOT` | **no** (404) | — |

Caveat on cataloguing: the `com.destroystokyo.paper:paper-api` **group-level** metadata advertises only `1.16.5-R0.1-SNAPSHOT` as `<latest>`, yet the `1.12.2` version directory resolves and serves a timestamped build. I did not exhaustively enumerate that group, so treat "which legacy Paper API versions still resolve" as partially mapped. **For 1.8.x there is no Paper API at all**, under either group.

### 1.3 Which repositories serve them, and the trap that breaks legacy builds

| Repository URL | Serves | Verified |
|---|---|---|
| `https://hub.spigotmc.org/nexus/content/repositories/snapshots/` | `org.spigotmc:spigot-api` (all versions, incl. 1.8.8/1.16.5) | 200 |
| `https://hub.spigotmc.org/nexus/content/repositories/public/` | `org.spigotmc:spigot-api` **and** `net.md-5:bungeecord-chat` (incl. the old `1.8-SNAPSHOT` / `1.12-SNAPSHOT`) | 200 for both |
| `https://repo.papermc.io/repository/maven-public/` | `io.papermc.paper:paper-api`, `com.destroystokyo.paper:paper-api`, `net.md-5:bungeecord-chat`, **and it also proxies `org.spigotmc:spigot-api`** (1.8.8 metadata → 200) | 200 |
| `https://repo1.maven.org/maven2/` | `net.md-5:bungeecord-chat` **releases only** (`1.16-R0.4`, `1.20-R0.1`, `1.21-R0.4`, …); **no `-SNAPSHOT`** | 200 / snapshot 404 |

**The trap (found by actually running the build).** Declaring only the Spigot *snapshots* repo is **not sufficient for legacy**: `spigot-api:1.8.8-R0.1-SNAPSHOT` declares a compile dependency on `net.md-5:bungeecord-chat:1.8-SNAPSHOT`, which the `snapshots` repo does not contain. Observed failure:

```
> Could not resolve all files for configuration ':compileClasspath'.
   > Could not find net.md-5:bungeecord-chat:1.8-SNAPSHOT.
     Searched in the following locations:
       - …/snapshots/net/md-5/bungeecord-chat/1.8-SNAPSHOT/maven-metadata.xml
       - …/snapshots/net/md-5/bungeecord-chat/1.8-SNAPSHOT/bungeecord-chat-1.8-SNAPSHOT.pom
     Required by:
         org.spigotmc:spigot-api:1.8.8-R0.1-SNAPSHOT:20160221.082514-43
```

The old Sonatype OSS snapshot repositories that once hosted `net.md-5` are gone (`oss.sonatype.org` / `s01.oss.sonatype.org` snapshot paths → 404). The skeleton must therefore use **either** the Spigot `public` group repo **or** `repo.papermc.io`:

* With `https://hub.spigotmc.org/nexus/content/repositories/public/` as the only Spigot repo, the full legacy compile succeeded (see §2.4).
* With `https://repo.papermc.io/repository/maven-public/` as the only repository, `spigot-api:1.8.8` and its transitives resolved; the **only** resolution error in that run was the deliberately-included nonexistent `io.papermc.paper:paper-api:1.16.5-R0.1-SNAPSHOT`. `repo.papermc.io` is thus the best candidate for a **single repository URL covering both ends** (legacy Spigot **and** modern Paper) — which is a significant skeleton simplification.

Full transitive dependency lists (from the POMs) explain the fragility:

* `spigot-api:1.8.8` → `commons-lang:2.6`, `json-simple:1.1.1`, `guava:17.0`, `gson:2.2.4`, `ebean:2.8.1`, `snakeyaml:1.15`, **`net.md-5:bungeecord-chat:1.8-SNAPSHOT`**
* `spigot-api:1.12.2` → … **`net.md-5:bungeecord-chat:1.12-SNAPSHOT`**, `snakeyaml:1.19`
* `spigot-api:1.16.5` → … **`net.md-5:bungeecord-chat:1.16-R0.4`** (a release; resolves from Central), `snakeyaml:1.27`

So only the *pre-1.16* legacy coordinates depend on an unresolvable-by-default snapshot.

### 1.4 Authentication and ToS/licensing

* **No authentication is required for reads.** Every metadata, POM, and JAR fetch above succeeded with plain unauthenticated HTTPS; the Spigot Nexus read path needs no credentials in 2026.
* **`spigot-api` needs no BuildTools.** Decisive empirical point: I compiled a plugin against `spigot-api:1.8.8-R0.1-SNAPSHOT` without ever running BuildTools, downloading a Spigot server, or accepting any licence interactively. BuildTools builds the **server**, not the API artifact; the API artifact is published to a public Maven repository.
* **The `spigot-api` POM declares no license at all.** Neither `1.8.8-R0.1-SNAPSHOT` nor `1.16.5-R0.1-SNAPSHOT` contains a `<licenses>` element (grep count 0 in both POMs). So there is no license metadata in the artifact to comply with or to cite.
* **What I could NOT verify:** Spigot's actual ToS/licence wording, and BuildTools' redistribution constraints. `https://www.spigotmc.org/wiki/buildtools/`, `https://www.spigotmc.org/`, and `https://hub.spigotmc.org/` (HTML) all return **403** from this network. The commonly-cited position — that the *API* may be compiled against but the *server jar produced by BuildTools* may not be redistributed — is **not confirmed here from a primary source**. Treat any "you may redistribute Spigot" claim as unverified, and treat BuildTools as a build-time-only, user-invoked step rather than something vinoa ships or redistributes.

---

## 2. Can Gradle 9.8 compile a Java 8 target against a Java 8-era `paper-api`/`spigot-api`?

**Yes.** Short answer with the exact policy and the exact caveats below.

### 2.1 Gradle's own support policy (the authoritative sentences)

From Gradle's compatibility matrix (**version 9.8.0**):

> "Gradle runs on the Java Virtual Machine (JVM)… **A JVM version between 17 and 27 is required to execute Gradle.** JVM 28 and later versions are not yet supported."
> "The Gradle wrapper, Gradle client, Tooling API client, and TestKit client are compatible with JVM 8."
> "**JDK 6 and above can be used for compilation.** JVM 8 and above can be used for executing tests."
> "Any fully supported version of Java can be used for compilation or testing… **Support is achieved using toolchains** and applies to all tasks supporting toolchains."

And the per-Java-version table (`Support for running Gradle`):

| Java version | Support for toolchains | Support for running Gradle |
|---|---|---|
| **8** | N/A | **2.0 to 8.14.x** |
| 11 | N/A | 5.0 to 8.14.x |
| 17 | 7.3 | 7.3 and after |
| 21 | 8.4 | 8.5 and after |
| 25 | 9.1.0 | 9.1.0 and after |
| 26 | 9.4.0 | 9.4.0 and after |
| 27 | 9.8.0 | 9.8.0 and after |

**Reading of the table that matters here:** the "Support for running Gradle" column for Java 8 ends at **8.14.x** — i.e. Gradle 9.x **cannot be executed on** a Java 8 JVM. Nothing in the policy stops Gradle 9.8 from **compiling for** Java 8; the separate sentence "JDK 6 and above can be used for compilation… Support is achieved using toolchains" is the positive statement of that capability. (Note the `Support for toolchains` cell for Java 8 reads "N/A"; the same "N/A" appears for Java 8–14 in the Gradle 8.14.3 edition of the page, so it is a documentation quirk for the pre-toolchain-feature era, **not** a statement that Java 8 cannot be a toolchain. The prose sentence above it is the operative one, and Gradle's own current toolchains chapter demonstrates a Java 8 configuration: *"In the example below, we configure all java compilation tasks to use Java 8."*)

**Nothing in Gradle 9 removed the `sourceCompatibility`/`targetCompatibility` era APIs, and nothing removed Java 8 as a target.** The Gradle 9 upgrade guide says the opposite of a removal, describing default behaviour derived from them:

> "The java-base plugin uses the JavaCompile tasks it creates to determine the default source and target compatibility when `sourceCompatibility` / `targetCompatibility` or `release` are not set."

### 2.2 javac caveat (the real long-term risk)

Java 8 is the **current floor of javac itself**, and it now warns that the floor will move:

```
$ /usr/lib/jvm/java-25-openjdk/bin/javac --release 8 -d out A.java
warning: [options] source value 8 is obsolete and will be removed in a future release
warning: [options] target value 8 is obsolete and will be removed in a future release
warning: [options] To suppress warnings about obsolete options, use -Xlint:-options.
exit=0        → produced class file major version 52

$ /usr/lib/jvm/java-25-openjdk/bin/javac --release 7 …
error: release version 7 not supported
```

So on JDK 25 (which vinoa's ceiling needs for Paper 26.x), `--release 8` still works and still emits major 52, but it is officially on death row — the skeleton should keep the Java 8 target in one property so it can be raised centrally, and expect noise from `-source/-target` obsolescence warnings.

### 2.3 Empirical verification (what I actually ran)

Environment: local Gradle **9.7.1** on **JDK 21** (`Launcher JVM: 21.0.12.1`), `org.gradle.java.installations` untouched, JDK 17/21/25 installed, no JDK 8.

**Test 1 — legacy dependency + Java 8 bytecode target.** `java.toolchain = 21`, `options.release = 8`, `compileOnly("org.spigotmc:spigot-api:1.8.8-R0.1-SNAPSHOT")` (Java **6** bytecode!), source extending `JavaPlugin` and calling `Bukkit.getLogger()`:

```
BUILD SUCCESSFUL
./build/classes/java/main/com/example/T.class
  bytecode major 52 (52=Java8)
```

**Test 2 — the era APIs still work.** `sourceCompatibility = JavaVersion.VERSION_1_8`, `targetCompatibility = JavaVersion.VERSION_1_8`, toolchain 21, same legacy dependency:

```
BUILD SUCCESSFUL in 20s
  bytecode major 52 (52=Java8)
```
Only javac's "obsolete" warnings appeared; Gradle emitted **no** deprecation of `sourceCompatibility`/`targetCompatibility`.

**Test 3 — Gradle accepts a Java 8 *toolchain* request.** `java.toolchain { languageVersion = JavaLanguageVersion.of(8) }`:

```
> Cannot find a Java installation on your machine (…) matching:
  {languageVersion=8, vendor=any vendor, implementation=vendor-specific, nativeImageCapable=false}.
  Toolchain download repositories have not been configured.
```

This is the expected good-news failure: Gradle **understands and resolves** a Java 8 toolchain request; it only complains that no JDK 8 is installed and that auto-provisioning repositories aren't configured. On a machine with a JDK 8 — or with the foojay resolver convention plugin — this path works. It is the escape hatch when you need `javac` 8's own behaviour (or a Java 8 *launcher* for running the server), rather than merely Java 8 *bytecode*.

### 2.4 What was NOT verified

* **Gradle 9.8 itself was not executed** (distribution undownloadable here; §Environment caveats). 9.7.1 was executed instead; the policy quotes are from the 9.8.0 manual.
* **The Java 8 toolchain path was not run to completion** (no JDK 8 obtainable here). Everything up to resolution was verified (Test 3).
* Whether Gradle's *auto-provisioning* of JDK 8 works from this network is unknown (foojay/Adoptium binaries redirect to the unreachable `github.com`).

---

## 3. Quality tooling on a Java 8 target

Two different questions hide here, and conflating them produces wrong answers:
1. **What JVM does the tool require to RUN?** (This is what breaks builds.)
2. **Can it still analyse/compile Java 8 code?** (For all three tools: yes — Java 8 is a supported *language level*, not something dropped.)

Because Gradle 9.8 requires a JVM 17–27 to run, the tool JVMs are already ≥17, so the practical constraint is only relevant if you pin a *newer* tool than the Gradle JVM supports.

Bytecode majors were read directly out of the published JARs (`Main.class` for Checkstyle, `FindBugs2.class` for SpotBugs, `org/junit/jupiter/api/Test.class` for JUnit):

### 3.1 Checkstyle

| Version | Class major | Min JVM to run |
|---|---|---|
| 8.45.1 | 52 | Java 8 |
| **9.3** (last 9.x) | **52** | **Java 8 — last version runnable on Java 8** |
| **10.0** | **55** | **Java 11 — first version requiring Java 11** |
| 10.12.0, 10.21.0 (10.x ends at 10.26.1) | 55 | Java 11 |
| 11.0.0, 11.1.0 | 61 | Java 17 |
| 12.0.0, 12.3.1 (12.x ends at 12.3.1) | 61 | Java 17 |
| **13.0.0** | **65** | **Java 21 — first version requiring Java 21** |
| 14.1.0 (current) | 65 | Java 21 |

Corroborated by Checkstyle's own build config: its `pom.xml` sets `<java.version>21</java.version>` and `<maven.compiler.release>${java.version}</maven.compiler.release>`.

**Gradle integration (verified by running).** Gradle 9.7.1's built-in default Checkstyle version is **10.24.0** — the `checkstyle` configuration resolves `com.puppycrawl.tools:checkstyle:10.24.0` with no `toolVersion` set, and that string is also present in `gradle-code-quality-9.7.1.jar`. With that default, `checkstyleMain` ran successfully over a Java 8-targeted source set.

*Practical rule for vinoa:* leave `toolVersion` unset (default 10.24.0 = Java 11, fine on the JVM 17+ Gradle already needs) or pin ≤ **12.x**. Pinning **13.x/14.x requires a Java 21 runtime** for the Checkstyle worker — i.e. it couples "newest style checks" to "Gradle must run on 21", which is a real capability-switch cost given Gradle 9.8 permits JVM 17.

### 3.2 SpotBugs

| Version | Class major | Min JVM to run |
|---|---|---|
| 4.7.3 | 52 | Java 8 |
| **4.8.0 … 4.8.6 (4.8.x ends at 4.8.6)** | **52** | **Java 8 — 4.8.6 is the last version runnable on Java 8** |
| **4.9.0** | **55** | **Java 11 — first version requiring Java 11** |
| 4.9.8, 4.10.x (current 4.10.4) | 55 | Java 11 |

Official statement of the analysis-vs-runtime distinction, from SpotBugs' own README:

> "Building SpotBugs requires JDK 21 to run all the tests (**using SpotBugs requires JDK 11 or above, but it can analyze code compiled with older versions**)."

So SpotBugs 4.10.x is perfectly usable on a project whose output is major 52; it just needs a Java 11+ JVM to *run*, which Gradle 9.8 guarantees.

### 3.3 JUnit — 4 vs 5 vs 6 (the crispest floors)

| Line | Latest | Class major | Min JVM | Source of truth |
|---|---|---|---|---|
| **JUnit 4** | 4.13.2 | **49** (Java 5) | Java 8 works | Maven Central `<latest>`; bytecode |
| **JUnit 5 (Jupiter)** | **5.14.4** (last 5.x) | **52** | **Java 8** | JUnit's own docs, verbatim: *"Supported Java Versions — JUnit 5 requires Java 8 (or higher) at runtime."* |
| **JUnit 6** | 6.1.3 | **61** | **Java 17** | JUnit 6.0.0 release notes: *"Date of Release: September 30, 2025 — **Scope: Java 17 and Kotlin 2**."* |

Verification detail: I checked the whole boundary — `junit-jupiter-api` **5.13.4 → 52**, **5.14.1 → 52**, **5.14.4 → 52**, **6.0.1 → 61**, **6.1.3 → 61**. The 6.0.0 release notes also state *"Legacy documentation regarding Java 8 compatibility has been removed from the User Guide"*, which is exactly why the Java 8 statement must be cited from the **5.14.4** docs and not from `current`. **JUnit 6 cannot be used on a Java 8 target — it will not even load (major 61).**

Gradle side: Gradle's testing chapter still documents JUnit 4 support (a single `junit:junit` dependency + the default `useJUnit()`), JUnit Platform (`useJUnitPlatform()`), and JUnit Vintage for *"running JUnit 3 and JUnit 4 based tests on the platform"* / mixing with Jupiter. So Gradle 9.8 can run all three lines; the binding constraint is purely the JUnit artifact's own JVM floor. **What I did not verify:** the minimum JUnit Platform version each Gradle release supports, and whether Gradle 9.8's defaults have shifted.

---

## 4. `run-paper`, `paperweight`, and running a legacy test server

### 4.1 `xyz.jpenilla.run-paper` (Run-Paper) — can launch legacy Paper, but not 1.8.9

* **Latest: 3.1.0** (Gradle Plugin Portal metadata, released 2026-08-08; versions 1.0.0 → 3.1.0). The plugin marker depends on the implementation artifact **`xyz.jpenilla:run-task:3.1.0`**, which is served by the **Gradle Plugin Portal m2** (`plugins.gradle.org/m2/…`, 200) and is **not on Maven Central** (metadata → 404). Applying it needs `gradlePluginPortal()` in `pluginManagement`.
* **No MC-version allowlist was found in the plugin.** Inspecting the 3.1.0 JAR: it targets **`https://fill.papermc.io/v3/`** (the modern Fill API; no `api.papermc.io/v2` strings), exposes `minecraftVersion` (the only required config) plus `getMinecraftVersion`, and declares `minecraftVersionIsSameOrNewerThan` for comparisons. It also knows `getLegacyPluginLoading` and `--nogui`, and it supports `runVelocity`/`runWaterfall`.
* **Therefore:** since Fill has builds for **1.8.8, 1.12.2, 1.16.5** (§4.3), `runServer { minecraftVersion("1.8.8") }` is a supported configuration. **`minecraftVersion("1.8.9")` cannot work — Paper has no 1.8.9 at all** (`version_not_found`).
* **It can select the server's JVM independently of Gradle.** The JAR contains `xyz.jpenilla.runtask.util.findJavaLauncher` using Gradle's `JavaToolchainService`/`JavaToolchainSpec`, a serializable model `xyz.jpenilla.runtask.paperapi.JavaVersion(minimum=…)`, `RunPlugin.javaLauncherConvention`, and (on `RunServer`) an **`ignoreUnsupportedJvm`** escape hatch. This maps exactly onto Fill's per-version Java metadata (§4.3): run-paper reads the server's minimum Java and resolves a matching toolchain — so a legacy server needing Java 8 can be launched on a Java 8 JVM while Gradle itself runs on 21/25.
* **Not verified:** the exact JVM-check error text and the code path that binds `JavaVersion.minimum` to a toolchain. `github.com`/`api.github.com` are unreachable here, and `raw.githubusercontent.com/jpenilla/run-task/...` 404s for source files (only `master`-branch `README.md` could be read, which documents `minecraftVersion(...)` usage but no version policy).

### 4.2 `io.papermc.paperweight.userdev` — hard floor 1.17.1, useless for the legacy range

* **Latest: 2.0.0-beta.24** (Gradle Plugin Portal metadata; line goes 1.5.1 → 1.7.7 → 2.0.0-beta.24). Prior research in this repo already pins it at `2.0.0-beta.24` and notes it is the only supported route to server internals (NMS).
* **Oldest dev-bundle: `1.17.1-R0.1-SNAPSHOT`.** The `io.papermc.paper:dev-bundle` metadata lists **291 versions**, oldest `1.17.1-R0.1-SNAPSHOT`, then `1.18`, … There are **no dev bundles for 1.8.9–1.16.5**. Paperweight/userdev is therefore categorically unavailable for the legacy end; official docs additionally note that reobfuscation is only relevant "up to 1.21.5" and no longer works for 26.x dev bundles, and that a given dev bundle may not support the Java toolchain Gradle is configured with.

### 4.3 The realistic alternatives for a legacy test server

| Option | Does it cover 1.8.9 / 1.12.2 / 1.16.5? | Evidence |
|---|---|---|
| **Paper server jar via Fill (`fill.papermc.io/v3`)** | 1.8.8 ✅ (`paper-1.8.8-445.jar`, STABLE, 2021-12-20), 1.12.2 ✅ (1620 builds), 1.16.5 ✅ (794 builds). **1.8.9 ❌ `{"ok":false,"error":"version_not_found"}`.** Paper's version list starts at **1.7.10** and contains 1.8.8 but no 1.8.9. | Fill API responses |
| **Spigot BuildTools** | ✅ for all three (it builds whatever Spigot revision you ask for). **Still published in 2026:** `hub.spigotmc.org/jenkins/job/BuildTools/lastSuccessfulBuild/artifact/target/BuildTools.jar` → HTTP **206** on a ranged GET; the build job page → **200**. Note this is needed for **Spigot** only, and its licence/redistribution terms are unreadable here (§1.4). | Jenkins artifact |
| **Vanilla server jar from Mojang** | ✅ all three — `downloads.server` is present for 1.8.9, 1.12.2, 1.16.5 in the official version manifest | `piston-meta` manifest + per-version JSON |
| **`paperweight`** | ❌ everything below 1.17.1 | §4.2 |

**Java runtime per MC version — Mojang's own metadata (the strongest available primary source):**

| MC | `javaVersion.component` | `javaVersion.majorVersion` | Paper Fill `java.version.minimum` |
|---|---|---|---|
| **1.8.9** | `jre-legacy` | **8** | — (no Paper 1.8.9) |
| **1.12.2** | `jre-legacy` | **8** | **8** |
| **1.16.5** | `jre-legacy` | **8** | **8** |
| 1.17 | `java-runtime-alpha` | 16 | — |
| **26.2** | `java-runtime-epsilon` | **25** | **25** |
| 26.3 | `java-runtime-epsilon` | 25 | — |

The two sources agree, which is a nice cross-check: **the legacy end needs Java 8, the ceiling needs Java 25** — a 17-version-spanning `javaTarget` range inside one build, which is precisely what Java toolchains exist to express.

---

## 5. `plugin.yml` across versions — what a modern file does on a 1.8.9 server

### 5.1 `api-version` — appeared in the **1.13** API

Instrumented from the real artifact JARs (searching the constant pool of `org/bukkit/plugin/PluginDescriptionFile.class`):

| spigot-api | `"api-version"` in class | `"libraries"` in class | `LibraryLoader` class | `contributors` |
|---|---|---|---|---|
| 1.8.8-R0.1 | ✗ | ✗ | ✗ | ✗ |
| 1.12.2-R0.1 | ✗ | ✗ | ✗ | ✗ |
| **1.13-R0.1** | **✓** (`apiVersion`, `getAPIVersion`) | ✗ | ✗ | ✗ |
| 1.14.4 / 1.15.2 / 1.16.1 / 1.16.2 / 1.16.3 / 1.16.4 | ✓ | ✗ | ✗ | ✗ |
| **1.16.5-R0.1** | ✓ | — | **✓** (`org/bukkit/plugin/java/LibraryLoader.class`) | ✓ |

So: **`api-version` first appeared in the 1.13 API** (absent in 1.8.8 and 1.12.2), and **`libraries:` arrived with the 1.16.5 API** — the `LibraryLoader` class is present in 1.16.5 and absent in 1.16.4 and every earlier version checked.

Official semantics of `api-version` (Paper's own `plugin.yml` reference):

> "The version of the Paper API that your plugin is using. This doesn't include the minor version until 1.20.5. From 1.20.5 and onward, a minor version is supported. **Servers with a version lower than the version specified here will refuse to load the plugin.** The valid versions are **1.13** - {LATEST_PAPER_RELEASE}."
> "If this is not specified, the plugin will be loaded as a **legacy plugin** and a warning will be printed to the console."

Note the direction of the compatibility check: a *higher* `api-version` makes *older* servers refuse the plugin. So a value of `1.13` is the most permissive legal value for modern servers, and omitting the key entirely is the most permissive of all (at the cost of a console warning).

`libraries:` semantics (Paper docs):

> "This is a list of libraries that your plugin depends on. These libraries will be downloaded from the Maven Central repository and added to the classpath. This removes the need to shade and relocate the libraries."

The docs also reference the implementing class by name — `org.bukkit.plugin.java.LibraryLoader.centralURL` — matching the Spigot `LibraryLoader` found in the 1.16.5 artifact, so Spigot and Paper share this mechanism.

### 5.2 Unknown keys are ignored — on both old and new servers

This is the key finding for the "modern `plugin.yml` on a 1.8.9 server" question: **nothing throws.**

* **1.8.8 (`loadMap`)**: the method reads each key by name via `Map.get("name")`, `Map.get("version")`, `Map.get("main")`, `Map.get("commands")`, `Map.get("authors")`, `Map.get("author")`, `Map.get("description")`, `Map.get("website")`, `Map.get("prefix")`, `Map.get("depend")`, `Map.get("softdepend")`, `Map.get("loadbefore")`, `Map.get("database")`, `Map.get("load")`, `Map.get("default-permission")`, `Map.get("permissions")`, `Map.get("awareness")`, `Map.get("class-loader-of")`. Its **complete** set of string constants contains **no** `"api-version"`, **no** `"libraries"`, and **no** "unknown key" error message. The only `InvalidDescriptionException`s are for *required* keys (`name`/`version`/`main` not defined, or of wrong type) and for invalid *values* of known keys (`main may not be within the org.bukkit namespace`, `load is not a valid choice`, `default-permission is not a valid choice`, `\' contains invalid characters`). The two `Map.entrySet()` iterations inside `loadMap` are the nested **`commands`** map traversal (the bytecode loads the `"commands"` key immediately before each), not a keyset validation loop.
* **Current Paper (`main`)**: `loadMap` likewise reads known keys by name and ends with `map.get("api-version")`, `map.get("libraries")`, `map.containsKey("paper-plugin-loader")`, `map.containsKey("paper-skip-libraries")`, `map.get("permissions")`, `map.get("prefix")` — again with no rejection of unknown keys.

**Consequence for a single skeleton:** one `plugin.yml` shape can be dropped on 1.8.9 through 26.2 safely at the *parsing* level — `api-version` and `libraries:` are simply invisible to a 1.8.9 server, and any other unknown key is ignored. What actually breaks is behavioural, and the skeleton must be explicit about it:

* **`libraries:` silently does nothing below 1.16.5.** If you rely on `libraries:` to inject a dependency, it will not be on the classpath on old servers — those versions need shading/bundling instead, or the feature must be gated off. (This is the most likely real-world "modern plugin.yml on an old server" bug.)
* **`api-version` must be chosen deliberately.** `1.13` works from 1.13 up and is ignored below; anything higher makes older-but-modern servers refuse the plugin; omitting it loads as legacy with a warning.
* **`contributors` is likewise invisible before 1.16.5** (present in the 1.16.5 API, absent in 1.13–1.15).
* `paper-plugin.yml` remains a modern-only alternative — the prior research in this repo records `PaperPluginMeta.MINIMUM = 1.19` — so it is irrelevant to the legacy range.

**Not verified:** the SnakeYAML scalar-coercion gotcha (`version: 1.10` being parsed as a number and stringified as `1.1`, etc.). Bukkit's loader uses SnakeYAML, and quoting `version` is standard defensive practice, but I did not execute a 1.8.9 server to observe it, so treat that specific caution as unconfirmed.

---

## 6. Verdict and reasoning

**Verdict: ONE skeleton with capability switches, not two template sets — with the "classic" and "modern" platform modules compiled as separate source sets/compilation units inside that single Gradle build.**

Reasoning, floor by floor:

1. **The build tool is not the problem.** Gradle 9.8 runs on JVM 17–27 and can compile **for** Java 6+ (docs: "JDK 6 and above can be used for compilation"), verified end-to-end: Gradle on JDK 21 compiled against Java-6-bytecode `spigot-api:1.8.8` and emitted major 52. The same build can hold a `JavaLanguageVersion.of(25)` module for Paper 26.x. A toolchain *and* a dependency *per module* is exactly what one Gradle build expresses; nothing forces a second template set at the build level.
2. **The dependency differences are data, not structure.** `spigot-api:1.8.8` vs `spigot-api:1.16.5` vs `com.destroystokyo.paper:paper-api:1.16.5` vs `io.papermc.paper:paper-api:26.2` is a value in `libs.versions.toml` plus a repository choice. Crucially, **one repository URL (`repo.papermc.io/maven-public`) proxies legacy `org.spigotmc:spigot-api` *and* modern `paper-api` *and* the `net.md-5` snapshot that legacy Spigot needs** — so even the repository list need not fork. (If you prefer vendor-native repos, use Spigot **`public`**, never Spigot `snapshots` alone — the latter is a verified hard build failure for pre-1.16 legacy.)
3. **The version mapping is many-to-one, and must be a lookup table.** MC 1.8.9 → `spigot-api:1.8.8` (there *is* no 1.8.9 artifact, and no Paper at all); 1.12.2 → `spigot-api:1.12.2` or `com.destroystokyo.paper:paper-api:1.12.2`; 1.16.5 → `spigot-api:1.16.5` or `com.destroystokyo.paper:paper-api:1.16.5`. A skeleton that derives the artifact coordinate by string-interpolating the MC version (`"…:${mcVersion}-R0.1-SNAPSHOT"`) will produce a 404 for 1.8.9 — this is a concrete bug the dual-template approach would not have avoided, only moved.
4. **`plugin.yml` does not need two files.** Unknown keys are ignored on 1.8.9 *and* on modern Paper (verified in both code paths), so one metadata file loads everywhere; the version-sensitive behaviour is `api-version` (choose `1.13` or omit) and `libraries:` (must be treated as a ≥1.16.5-only capability).
5. **Test-server tooling is where a genuine gap exists, and it argues for a switch, not a second skeleton.** `run-paper` 3.1.0 can run 1.8.8/1.12.2/1.16.5 and can pick the *server's* JVM via Gradle toolchains; there is simply no Paper 1.8.9, so 1.8.9 needs a Spigot BuildTools/vanilla path. `paperweight` is unavailable below 1.17.1 — meaning **NMS/`userdev` capability is a modern-only switch** and must not be advertised on the classic module.

**The one place "two templates" is genuinely defensible** — and it is a module-level split, not a project-level one: **the Java source itself cannot be one.** Code compiled against `paper-api:26.2` cannot also run on 1.8.9, and code for 1.8.9 cannot call modern APIs. So the skeleton must produce *at least* two source sets (e.g. `common` + `platform-classic` / `platform-modern`), which a normal Gradle multi-module project already gives you. That is the "two templates" pressure, fully absorbed by the module structure vinoa already has.

**Concrete switch set the skeleton needs:**

| Switch | Values | Drives |
|---|---|---|
| `mcVersion` | `1.8.9`, `1.12.2`, `1.16.5`, …, `26.2` | Artifact lookup (never string-interpolated), run task, docs |
| `apiFlavor` | `spigot` \| `paper` | Coordinate + repository; Paper only ≥1.17 via `io.papermc.paper`, 1.16.5/1.12.2 via `com.destroystokyo.paper`, 1.8.x Spigot-only |
| `javaTarget` | `8` (≤1.16.5) \| `17` \| `21` \| `25` (26.x) | Toolchain + `options.release` per module (from Mojang `javaVersion.majorVersion` / Fill `java.version.minimum`) |
| `pluginYmlApiVersion` | omitted \| `1.13` | `api-version:` line |
| `librariesEnabled` | `true` (≥1.16.5) \| `false` | `libraries:` block + shading fallback |
| `runTask` | `run-paper` (≥1.8.8 Paper) \| BuildTools/vanilla (1.8.9, Spigot) | Test-server wiring; `paperweight` is gated to ≥1.17.1 |
| `qualityToolVersion` | default `10.24.0` \| ≤12.x \| 13+/14 (needs JVM 21) | Checkstyle only; SpotBugs/JUnit are unconstrained above 11/17 |

Two template sets would duplicate the Gradle scaffold, `libs.versions.toml`, common-module layout, and the metadata file — all of which are demonstrably shared — in exchange for solving a problem (source-level API incompatibility) that the module split already solves.

---

## 7. Confidence and explicitly unverified items

**High confidence (direct evidence, often executed):**
* absent 1.8.9 Spigot artifact; absent Paper API below 1.17 / below 1.16.5 for `com.destroystokyo`; bytecode versions of spigot-api 1.8.8/1.12.2/1.13/1.16.5; the `net.md-5` resolution failure and the two working repository alternatives; `api-version` first appearing in 1.13; `libraries`/`LibraryLoader` first appearing in 1.16.5; unknown-key tolerance in both 1.8.8 and current Paper; Gradle 9.8 Java-runtime policy and Java-8 compilation support; javac 25 `--release 8` behaviour; Checkstyle/SpotBugs/JUnit bytecode floors; Mojang `javaVersion` per MC version; Paper Fill's per-version `java.version.minimum`; oldest Paperweight dev-bundle 1.17.1; run-paper 3.1.0 coordinates and its Fill/toolchain/`ignoreUnsupportedJvm` machinery.

**Could NOT verify (stated as unverified in the body, never asserted):**
1. **No execution of Gradle 9.8** — the distribution could not be downloaded (network truncation). Empirical runs used 9.7.1; 9.8 policy claims are from the 9.8.0 manual.
2. **No end-to-end Java 8 *toolchain* run** — no JDK 8 obtainable (Adoptium→GitHub unreachable; foojay redirect timed out). Verified only that Gradle accepts and tries to resolve a `languageVersion=8` toolchain, failing solely on "no JDK 8 installed / no download repositories configured".
3. **Spigot ToS and BuildTools licensing/redistribution terms** — `spigotmc.org` and `hub.spigotmc.org` HTML return 403. Only established: the artifact is publicly readable without auth, and the POMs declare no `<licenses>`.
4. **Spigot wiki / BuildTools documentation content** — same 403.
5. **run-paper's source** — `github.com`/`api.github.com` unreachable; `raw.githubusercontent.com/jpenilla/run-task/...` 404s except `master/README.md`. The JVM-check error text, the version→Java mapping code, and any hidden minimum-version guard are inferred from JAR constant-pool strings + Fill API fields, not read from source.
6. **Full enumeration of the `com.destroystokyo.paper:paper-api` legacy catalogue** — the group metadata advertises only 1.16.5 while a 1.12.2 version directory resolves; other legacy versions may or may not exist.
7. **Paperweight's exact supported-version statement in prose** — inferred from the dev-bundle metadata (oldest 1.17.1) rather than quoted from the docs; the docs page fetched but its full text (including a version range sentence) could not be cleanly extracted.
8. **Minimum JUnit Platform version per Gradle release**, and whether Gradle 9.8 changed any JUnit default.
9. **SnakeYAML scalar coercion** for `plugin.yml` `version:` values.
10. Whether Gradle's **toolchain auto-provisioning of JDK 8** works from this network (vendor binaries redirect to the unreachable `github.com`).

---

## Sources

**Gradle (official)**
- <https://docs.gradle.org/current/userguide/compatibility.html> — "A JVM version between 17 and 27 is required to execute Gradle"; "JDK 6 and above can be used for compilation"; Java-compatibility table (Java 8 "running Gradle: 2.0 to 8.14.x"; 17 "7.3 and after"; 25 "9.1.0"; 27 "9.8.0"). Page renders as version 9.8.0.
- <https://docs.gradle.org/8.14.3/userguide/compatibility.html> — same table for comparison (confirms the "N/A" toolchains cell for Java 8–14 is longstanding).
- <https://docs.gradle.org/current/userguide/toolchains.html> — "In the example below, we configure all java compilation tasks to use Java 8"; `--release` vs `sourceCompatibility` guidance; auto-detection/provisioning.
- <https://docs.gradle.org/current/userguide/upgrading_version_8.html> — "The java-base plugin uses the JavaCompile tasks it creates to determine the default source and target compatibility when sourceCompatibility / targetCompatibility or release are not set."
- <https://docs.gradle.org/current/userguide/java_testing.html> — JUnit 4 support, `useJUnitPlatform()`, JUnit Vintage for JUnit 3/4 tests.
- <https://docs.gradle.org/current/userguide/java_plugin.html>
- <https://services.gradle.org/versions/current> — Gradle 9.8.0 current (`buildTime 20260924134000`), confirming 9.8.0 is the current stable and hence what `docs.gradle.org/current` documents.
- Local Gradle 9.7.1 distribution (`/usr/share/java/gradle`): `gradle-code-quality-9.7.1.jar` and `gradle-public-api-legacy-9.7.1.jar` contain the default Checkstyle version string `10.24.0`; `gradle-code-quality-workers-9.7.1.jar` also references `10.3.3`.

**Spigot / Bukkit artifacts (Nexus + the artifacts themselves)**
- <https://hub.spigotmc.org/nexus/content/repositories/snapshots/org/spigotmc/spigot-api/maven-metadata.xml> — all 82 `spigot-api` versions, 1.8 → 26.3; no `1.8.9`.
- <https://hub.spigotmc.org/nexus/content/repositories/snapshots/org/spigotmc/spigot-api/1.8.8-R0.1-SNAPSHOT/maven-metadata.xml> — 200; snapshot `1.8.8-R0.1-20160221.082514-43`.
- <https://hub.spigotmc.org/nexus/content/repositories/snapshots/org/spigotmc/spigot-api/1.8.9-R0.1-SNAPSHOT/maven-metadata.xml> — **404** (definitive: no 1.8.9 artifact).
- <https://hub.spigotmc.org/nexus/content/repositories/snapshots/org/spigotmc/spigot-api/1.12.2-R0.1-SNAPSHOT/maven-metadata.xml> — `1.12.2-R0.1-20180712.012057-156`.
- <https://hub.spigotmc.org/nexus/content/repositories/snapshots/org/spigotmc/spigot-api/1.16.5-R0.1-SNAPSHOT/maven-metadata.xml> — `1.16.5-R0.1-20210611.041013-99`.
- <https://hub.spigotmc.org/nexus/content/repositories/snapshots/org/spigotmc/spigot-api/1.8.8-R0.1-SNAPSHOT/spigot-api-1.8.8-R0.1-20160221.082514-43.pom> — `<maven.compiler.source>1.6`, `target 1.6`, dependency on `net.md-5:bungeecord-chat:1.8-SNAPSHOT`, **no `<licenses>`**.
- <https://hub.spigotmc.org/nexus/content/repositories/snapshots/org/spigotmc/spigot-api/1.16.5-R0.1-SNAPSHOT/spigot-api-1.16.5-R0.1-20210611.041013-99.pom> — deps incl. `net.md-5:bungeecord-chat:1.16-R0.4`; **no `<licenses>`**.
- `spigot-api-1.8.8-R0.1-20160221.082514-43.jar` — `org/bukkit/Bukkit.class` major **50**; `Build-Jdk: 1.7.0_72`.
- `spigot-api-1.12.2-R0.1-20180712.012057-156.jar`, `spigot-api-1.13-R0.1-20180826.040111-146.jar` — `PluginDescriptionFile.class` major **51**.
- `spigot-api-1.16.5-R0.1-20210611.041013-99.jar` — major **52**; contains `org/bukkit/plugin/java/LibraryLoader.class` (absent in 1.16.4/1.16.3/1.16.2/1.16.1/1.15.2/1.14.4/1.13).
- <https://hub.spigotmc.org/nexus/content/repositories/public/org/spigotmc/spigot-api/maven-metadata.xml> — the `public` group repo also serves spigot-api.
- <https://hub.spigotmc.org/nexus/content/repositories/public/net/md-5/bungeecord-chat/maven-metadata.xml> — `1.7-SNAPSHOT` … `1.18-R0.1-SNAPSHOT` (only place the old `1.8-SNAPSHOT` lives besides repo.papermc.io).
- <https://hub.spigotmc.org/nexus/content/repositories/public/net/md-5/bungeecord-chat/1.8-SNAPSHOT/maven-metadata.xml> — `1.8-20160221.214602-128`.
- <https://hub.spigotmc.org/nexus/content/repositories/snapshots/net/md-5/bungeecord-chat/maven-metadata.xml> — **404** (the cause of the legacy build failure).
- <https://hub.spigotmc.org/jenkins/job/BuildTools/lastSuccessfulBuild/artifact/target/BuildTools.jar> — HTTP **206** on a ranged GET (BuildTools still published, 2026).
- <https://hub.spigotmc.org/jenkins/job/BuildTools/> — HTTP 200.
- <https://www.spigotmc.org/wiki/buildtools/>, <https://www.spigotmc.org/>, <https://hub.spigotmc.org/> — HTTP **403** (documentation/ToS unreadable).

**PaperMC (repository, API, docs, source)**
- <https://repo.papermc.io/repository/maven-public/io/papermc/paper/paper-api/maven-metadata.xml> — oldest version `1.17-R0.1-SNAPSHOT` (no `1.8.9`/`1.12.2`/`1.16.5`).
- <https://repo.papermc.io/repository/maven-public/io/papermc/paper/paper-api/1.16.5-R0.1-SNAPSHOT/maven-metadata.xml> — **404** (the coordinate does not exist).
- <https://repo.papermc.io/repository/maven-public/com/destroystokyo/paper/paper-api/maven-metadata.xml> — only `1.16.5-R0.1-SNAPSHOT` advertised.
- <https://repo.papermc.io/repository/maven-public/com/destroystokyo/paper/paper-api/1.16.5-R0.1-SNAPSHOT/maven-metadata.xml> — `1.16.5-R0.1-20211218.082619-371`.
- <https://repo.papermc.io/repository/maven-public/com/destroystokyo/paper/paper-api/1.12.2-R0.1-SNAPSHOT/maven-metadata.xml> — `1.12.2-R0.1-20190714.184133-413`.
- <https://repo.papermc.io/repository/maven-public/com/destroystokyo/paper/paper-api/1.8.8-R0.1-SNAPSHOT/maven-metadata.xml> — **404** (no Paper API for 1.8).
- <https://repo.papermc.io/repository/maven-public/org/spigotmc/spigot-api/maven-metadata.xml> — repo.papermc.io proxies legacy Spigot API (1.8.x … 1.16.5 present).
- <https://repo.papermc.io/repository/maven-public/net/md-5/bungeecord-chat/1.8-SNAPSHOT/maven-metadata.xml> — 200 (`1.8-20160221.214602-128`).
- <https://repo.papermc.io/repository/maven-public/io/papermc/paper/dev-bundle/maven-metadata.xml> — 291 dev bundles, **oldest `1.17.1-R0.1-SNAPSHOT`**.
- <https://fill.papermc.io/v3/projects/paper> — Paper version catalogue (68 versions incl. 1.7.10 and 1.8.8, **no 1.8.9**).
- <https://fill.papermc.io/v3/projects/paper/versions/1.8.9/builds> — `{"ok":false,"error":"version_not_found"}`.
- <https://fill.papermc.io/v3/projects/paper/versions/1.8.8/builds> — `paper-1.8.8-445.jar`, STABLE, 2021-12-20.
- <https://fill.papermc.io/v3/projects/paper/versions/1.12.2/builds> — 1620 builds.
- <https://fill.papermc.io/v3/projects/paper/versions/1.16.5/builds> — 794 builds.
- <https://fill.papermc.io/v3/projects/paper/versions/1.8.8> · <https://fill.papermc.io/v3/projects/paper/versions/1.12.2> · <https://fill.papermc.io/v3/projects/paper/versions/1.16.5> · <https://fill.papermc.io/v3/projects/paper/versions/26.2> — per-version `java.version.minimum` = **8 / 8 / 8 / 25**, and `support.status` = UNSUPPORTED / UNSUPPORTED / UNSUPPORTED / SUPPORTED.
- <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/getting-started/plugin-yml.mdx> — `api-version` semantics ("Servers with a version lower than the version specified here will refuse to load the plugin"; valid 1.13–latest; omitted → legacy plugin + warning), `libraries` semantics (Maven Central, "removes the need to shade and relocate"), `org.bukkit.plugin.java.LibraryLoader.centralURL`.
- <https://raw.githubusercontent.com/PaperMC/docs/main/src/content/docs/paper/dev/getting-started/plugin-yml.mdx> — the exact fetched copy of the above (200).
- <https://raw.githubusercontent.com/PaperMC/Paper/main/paper-api/src/main/java/org/bukkit/plugin/PluginDescriptionFile.java> — current `loadMap`: reads `api-version`, `libraries`, `paper-plugin-loader`, `paper-skip-libraries`, `permissions`, `prefix` by name; **no unknown-key rejection**; `InvalidDescriptionException` only for known keys/required fields; javadoc table listing `api-version` (example `1.13`) and `libraries`.
- <https://docs.papermc.io/paper/dev/userdev/> — paperweight-userdev docs (reobfuscation notes, toolchain caveat, "Reobfuscation (up to 1.21.5)").

**Mojang (official version metadata)**
- <https://piston-meta.mojang.com/mc/game/version_manifest_v2.json> — `latest.release = 26.3`, `latest.snapshot = 26.4-snapshot-1`; per-version `url`s used below.
- Per-version JSONs from that manifest for **1.8.9**, **1.12.2**, **1.16.5** (`javaVersion.majorVersion = 8`, `jre-legacy`, `downloads.server` present), **1.17** (16, `java-runtime-alpha`), **26.2** / **26.3** (25, `java-runtime-epsilon`).

**JUnit**
- <https://docs.junit.org/5.14.4/user-guide/index.html> — **"Supported Java Versions — JUnit 5 requires Java 8 (or higher) at runtime."** (authoritative for the last Java-8 JUnit 5 line).
- <https://docs.junit.org/6.0.1/release-notes.html> — "Date of Release: September 30, 2025 — **Scope: Java 17 and Kotlin 2**"; "Legacy documentation regarding Java 8 compatibility has been removed from the User Guide."
- <https://docs.junit.org/6.1.3/release-notes.html> — current release notes (same statements).
- <https://repo1.maven.org/maven2/junit/junit/maven-metadata.xml> — JUnit 4 `<latest>/<release>` = **4.13.2**; `junit-4.13.2.jar` `org/junit/Test.class` major **49**.
- <https://repo1.maven.org/maven2/org/junit/jupiter/junit-jupiter/maven-metadata.xml> — 5.x ends at **5.14.4**; 6.x `6.0.0` … `6.1.3`.
- `junit-jupiter-api-5.13.4.jar` / `5.14.1` / `5.14.4` → major **52**; `6.0.1` / `6.1.3` → major **61**.
- <https://repo1.maven.org/maven2/org/junit/platform/junit-platform-launcher/maven-metadata.xml> — current 6.1.3.

**Checkstyle**
- <https://repo1.maven.org/maven2/com/puppycrawl/tools/checkstyle/maven-metadata.xml> — latest **14.1.0**; 9.x ends at **9.3**; 10.x `10.0` … `10.26.1`; 11.x ends 11.1.0; 12.x ends 12.3.1; 13.x to 13.11.0.
- Bytecode probes: `checkstyle-8.45.1` → 52; **`9.3` → 52**; **`10.0` → 55**; `10.12.0` / `10.21.0` → 55; `11.0.0` → 61; `12.0.0` / `12.3.1` → 61; **`13.0.0` → 65**; `14.1.0` → 65 (class `com/puppycrawl/tools/checkstyle/Main.class`).
- <https://raw.githubusercontent.com/checkstyle/checkstyle/master/pom.xml> — `<java.version>21</java.version>`, `<maven.compiler.release>${java.version}</maven.compiler.release>`.
- <https://checkstyle.org/releasenotes.html> · <https://checkstyle.org/running.html> · <https://checkstyle.org/index.html> — site pages (index confirms "Checkstyle 14.1.0 – checkstyle, Last Published: 2026-08-30").

**SpotBugs**
- <https://repo1.maven.org/maven2/com/github/spotbugs/spotbugs/maven-metadata.xml> — latest **4.10.4**; 4.8.x ends at **4.8.6**; 4.9.x from 4.9.0.
- Bytecode probes (`edu/umd/cs/findbugs/FindBugs2.class`): `4.7.3` → 52; `4.8.0` → 52; **`4.8.6` → 52**; **`4.9.0` → 55**; `4.9.8` → 55; `4.10.4` → 55.
- <https://raw.githubusercontent.com/spotbugs/spotbugs/master/README.md> — **"using SpotBugs requires JDK 11 or above, but it can analyze code compiled with older versions"**.
- <https://raw.githubusercontent.com/spotbugs/spotbugs/master/CHANGELOG.md> — Java 11/17 compatibility entries.
- <https://spotbugs.readthedocs.io/en/stable/> · `/running.html` · `/installing.html` · `/gradle.html` — docs index (no explicit minimum-Java sentence found on these pages).

**Gradle plugins (Run-Paper / Paperweight)**
- <https://plugins.gradle.org/m2/xyz/jpenilla/run-paper/xyz.jpenilla.run-paper.gradle.plugin/maven-metadata.xml> — latest **3.1.0**, `lastUpdated 20260808183317`.
- <https://plugins.gradle.org/m2/xyz/jpenilla/run-paper/xyz.jpenilla.run-paper.gradle.plugin/3.1.0/xyz.jpenilla.run-paper.gradle.plugin-3.1.0.pom> — depends on `xyz.jpenilla:run-task:3.1.0`.
- <https://plugins.gradle.org/m2/xyz/jpenilla/run-task/3.1.0/run-task-3.1.0.jar> — contains `https://fill.papermc.io/v3/`, `minecraftVersion`/`getMinecraftVersion`, `minecraftVersionIsSameOrNewerThan`, `xyz.jpenilla.runtask.util.findJavaLauncher` (`JavaToolchainService`/`JavaToolchainSpec`), `xyz.jpenilla.runtask.paperapi.JavaVersion(minimum=…)`, `RunPlugin.javaLauncherConvention`, `RunServer.ignoreUnsupportedJvm`, `getLegacyPluginLoading`; plugin ids `xyz.jpenilla.run-paper`, `xyz.jpenilla.run-velocity`, `xyz.jpenilla.run-waterfall`.
- <https://plugins.gradle.org/plugin/xyz.jpenilla.run-paper> — plugin portal page: description "Gradle plugin adding a task to run a Paper Minecraft server", owner Jason Penilla, version 3.1.0 created 2026-08-08, sources `github.com/jpenilla/run-paper`.
- <https://raw.githubusercontent.com/jpenilla/run-paper/master/README.md> · <https://raw.githubusercontent.com/jpenilla/run-task/master/README.md> — README: `plugins { id("xyz.jpenilla.run-paper") version "VERSION" }` and `tasks { runServer { minecraftVersion("1.21.8") } }`; "the only required configuration besides applying the plugin"; links to the wiki for detailed usage.
- <https://plugins.gradle.org/m2/io/papermc/paperweight/userdev/io.papermc.paperweight.userdev.gradle.plugin/maven-metadata.xml> — latest **2.0.0-beta.24**, `lastUpdated 20260925031623`; line 1.5.1 → 1.7.7 → 2.0.0-beta.1…24.
- <https://repo1.maven.org/maven2/xyz/jpenilla/run-paper/maven-metadata.xml> and `…/xyz/jpenilla/run-task/maven-metadata.xml` — **404** (not on Maven Central; use `gradlePluginPortal()`).

**Rust/vinoa context (internal, for cross-reference, not a research source)**
- Prior research in this repository: `docs/research/paper-plugin-project-shape.md` and `docs/research/rust-scaffold-cli-facts.md` (used only for continuity on `paper-plugin.yml`/`PaperPluginMeta.MINIMUM = 1.19`, Paper 26.x requiring Java 25, run-paper 3.1.0, and paperweight 2.0.0-beta.24 — all re-verified independently above where they bear on the verdict).
