# Canonical Shape of a Modern Paper/Bukkit Plugin Project (2026)

**Research date:** 2026-09-25.
**Scope:** what a scaffolding CLI should generate for a new Paper plugin, and every version-sensitive knob it must parameterize.
**Method / evidence rules:** every claim is traced to a primary source — `docs.papermc.io`, the PaperMC docs source repository (`PaperMC/docs`, the exact Markdown the site is built from), the PaperMC/Paper server source, the PaperMC Fill API (`fill.papermc.io`), Mojang's version manifest, the Gradle Plugin Portal/Maven metadata, or the owning Gradle plugin repositories. Blog posts, tutorials, and Stack Overflow are not used as sources.

> **Environment caveats that affect citations.**
> - `raw.githubusercontent.com` was intermittently unreachable from this workspace; GitHub content was therefore fetched through the authenticated `api.github.com` contents API, which is the same content. `docs.papermc.io` is a client-rendered Astro/Starlight site, so claims from it were verified by fetching **both** the rendered page and the source `.md`/`.mdx` in `PaperMC/docs` (the source file is the stronger citation and is what is linked below where a specific line matters).
> - `spigotmc.org` (including its wiki) returned HTTP 403 (Cloudflare) from this network, so Spigot-side claims are sourced from Paper's docs and the Spigot API artifact instead. This is noted in "Confidence / unverified".

---

## 0. TL;DR — the canonical shape

| Decision | Recommendation for 2026 | Why |
|---|---|---|
| Metadata file | **`plugin.yml`** for a default scaffold; offer `paper-plugin.yml` as an explicit opt-in | Paper plugins are still flagged **Experimental** in the official docs; the official project-setup guide's own IDE table says the Paper manifest "is not recommended as it is still in development" |
| Required fields | `name`, `version`, `main` (`api-version` strongly advised; see §1) | `PluginDescriptionFile` javadoc: only these three are required; a plugin missing any is rejected |
| Minecraft / Paper version | **26.3** is the newest Minecraft release; **Paper 26.2** is the newest version with a non-`ALPHA` Paper build, and is what the live docs currently recommend targeting | `piston-meta` `latest.release = 26.3`; Fill API shows 26.3 has only `ALPHA` builds |
| API version string | Identical to the MC/Paper version identifier (`26.2`, `26.1.2`, `1.21.11`) | `Paper/.github/build-data` sets `apiVersion` = MC version; docs' `api-version` field description |
| Java | **Java 25** toolchain (Paper 26.1+ requires Java 25) | Paper docs requirements table; `Paper/build.gradle.kts` `JavaLanguageVersion.of(25)`, `options.release = 25` |
| Gradle wrapper | **9.8.0** (current stable, 2026-09-24) | `services.gradle.org/versions/current`; Paper itself pins `gradle-9.8.0-bin.zip` |
| Build plugin (default) | `java-library` + `xyz.jpenilla.run-paper` **3.1.0**; no shading needed | Paper API is `compileOnly`; docs offer `libraries:` in metadata to avoid shading |
| Build plugin (NMS) | add `io.papermc.paperweight.userdev` **2.0.0-beta.24** | Only supported way to access server internals |
| Dir layout | `src/main/java/<pkg>/`, `src/main/resources/{plugin.yml\|paper-plugin.yml}` | Paper docs `FileTree` in plugin-yml + project-setup |
| Official template repo | **None exists.** Use the IDE generator or hand-roll | Enumerating all 69 public `PaperMC` repos and the GitHub org search for `template` returns no plugin template |

---

## 1. Plugin metadata: `plugin.yml` vs `paper-plugin.yml`

### 1.1 `plugin.yml` (Bukkit/Spigot format — the stable default)

**Location:** root of the jar, i.e. `src/main/resources/plugin.yml`.
Paper: "The `plugin.yml` file is located in the `resources` directory of your project."
Source: <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/getting-started/plugin-yml.mdx>

**Required fields — exactly three.** From `PluginDescriptionFile`'s own class javadoc in the Paper API (inherited from Bukkit):

> "Every (almost* every) method corresponds with a specific entry in the plugin.yml. These are the **required** entries for every plugin.yml: `name`, `version`, `main`. Failing to include any of these items will throw an exception and cause the server to ignore your plugin."

Source: <https://github.com/PaperMC/Paper/blob/main/paper-api/src/main/java/org/bukkit/plugin/PluginDescriptionFile.java> (javadoc lines ~48–57)

**Full field list** (from the same javadoc table plus the docs field reference):

| Field | Required | Notes |
|---|---|---|
| `name` | **yes** | Displayed in plugin list and logs; overridden in logs if `prefix` is set |
| `version` | **yes** | Shown in plugin info/logs |
| `main` | **yes** | Fully-qualified `JavaPlugin` subclass |
| `api-version` | no (but see below) | Valid: `1.13` → latest Paper release. Minor version supported from `1.20.5` onward. **If omitted, Paper loads the plugin as a *legacy* plugin and prints a console warning** |
| `description` | no | Plugin-info command |
| `author` / `authors` | no | Single value or list |
| `contributors` | no | List |
| `website` | no | Plugin-info command |
| `load` | no | `STARTUP` or `POSTWORLD`; defaults to `POSTWORLD` |
| `prefix` | no | Replaces plugin name in log lines |
| `libraries` | no | GAV list, resolved from Maven Central + added to the classpath — **explicitly "removes the need to shade and relocate the libraries"** |
| `permissions` | no | Map of nodes → `{description, default, children}`; `default` may be `op`/`notop`/`true`/`false` |
| `default-permission` | no | Fallback `default` for permission nodes lacking one; defaults to `op` |
| `depend`, `softdepend`, `loadbefore`, `provides` | no | Plugin-name lists; `depend` is load-blocking |
| `commands` | no | Map of command name → `{description, usage, aliases, permission, permission-message}` |
| `awareness` | no | From the javadoc table (e.g. `FOLIA`) |
| `paper-plugin-loader`, `paper-skip-libraries` | no | Paper-specific additions to `plugin.yml`; docs mark both **Experimental** |

Sources: <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/getting-started/plugin-yml.mdx>, <https://github.com/PaperMC/Paper/blob/main/paper-api/src/main/java/org/bukkit/plugin/PluginDescriptionFile.java>

**Minimal working `plugin.yml`** (docs' own example):

```yaml
name: Example-Plugin
version: 1.0.0
main: com.example.paperplugin.ExamplePlugin
description: An example plugin
author: PaperMC
website: https://papermc.io
api-version: '26.2'
```

Note the docs example uses `api-version: '{LATEST_PAPER_RELEASE}'` — the value is substituted at docs build time from the PaperMC Fill API (`src/utils/versions.ts`), which today resolves to `26.2`. Full `plugin.yml` with a command and a permission:

```yaml
name: Example-Plugin
version: 1.0.0
main: com.example.paperplugin.ExamplePlugin
api-version: '26.2'

commands:
  example:
    description: "An example command"
    usage: "/example <arg>"
    aliases: [ex]
    permission: example.command
    permission-message: "You do not have permission to use this command"

permissions:
  example.command:
    description: "Allows using /example"
    default: op
```

### 1.2 `paper-plugin.yml` (Paper plugin format — richer, still Experimental)

**Location:** root of the jar, i.e. `src/main/resources/paper-plugin.yml`. Docs: "Similarly to Bukkit plugins, you have to introduce a `paper-plugin.yml` file into your JAR resources folder. This will not act as a drop-in replacement for `plugin.yml`."
Source: <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/getting-started/paper-plugins.md>

**The authoritative schema is the deserialization class.** `PaperPluginMeta` is `@ConfigSerializable` and parsed with Configurate; fields marked `@Required` are mandatory:

| Field | Required | Default / type |
|---|---|---|
| `name` | **yes** | — |
| `main` | **yes** | — |
| `version` | **yes** | — |
| `api-version` | **yes** | `ApiVersion`; **minimum accepted is `1.19`** — anything older throws `SerializationException`: `"<version> is too old for a paper plugin!"` |
| `bootstrapper` | no | FQCN implementing `PluginBootstrap` |
| `loader` | no | FQCN implementing `PluginLoader` |
| `provides` | no | `List<String>` |
| `description` | no | `String` |
| `authors` | no | `List<String>` (a scalar `author:` key is also accepted and **appended** to `authors` during `create()`) |
| `contributors` | no | `List<String>` |
| `website` | no | `String` |
| `prefix` | no | `String` |
| `load` | no | `PluginLoadOrder`, defaults to `POSTWORLD` |
| `defaultPerm` | no | `PermissionDefault`, defaults to `OP` |
| `permissions` | no | `List<Permission>` |
| `dependencies` | no | Map keyed by lifecycle enum: **`BOOTSTRAP`** and **`SERVER`** |

Dependency entry shape (`DependencyConfiguration` record `load`, `required`, `joinClasspath`):

| Key | Values | Default |
|---|---|---|
| `load` | `BEFORE` \| `AFTER` \| `OMIT` | `OMIT` (undefined ordering) |
| `required` | boolean | `true` |
| `join-classpath` | boolean | `true` |

Sources: <https://github.com/PaperMC/Paper/blob/main/paper-server/src/main/java/io/papermc/paper/plugin/provider/configuration/PaperPluginMeta.java>, <https://github.com/PaperMC/Paper/blob/main/paper-server/src/main/java/io/papermc/paper/plugin/provider/configuration/type/DependencyConfiguration.java>, <https://github.com/PaperMC/Paper/blob/main/paper-server/src/main/java/io/papermc/paper/plugin/provider/configuration/type/PluginDependencyLifeCycle.java>

> **Important detail the prose docs understate:** `api-version` is `@Required` for `paper-plugin.yml`, whereas for `plugin.yml` it is optional (omission ⇒ legacy load + warning). A scaffolder that emits `paper-plugin.yml` **must** always write `api-version`.

**Minimal working `paper-plugin.yml`:**

```yaml
name: Example-Plugin
version: 1.0.0
main: com.example.paperplugin.ExamplePlugin
api-version: '26.2'
description: An example Paper plugin
authors: [PaperMC]
```

**`paper-plugin.yml` with dependencies** (from the docs):

```yaml
name: Example-Plugin
version: '1.0'
main: com.example.paperplugin.ExamplePlugin
api-version: '26.2'
dependencies:
  server:
    ProtocolLib:
      load: BEFORE
      required: true
      join-classpath: true
```

### 1.3 Which one does a server require? Can they coexist?

- **Bukkit/Spigot/Paper all read `plugin.yml`.** `paper-plugin.yml` is only understood by Paper.
- **Both files may coexist in one jar.** Docs: "It should be noted that you still have the ability to include both `paper-plugin.yml` and `plugin.yml` in the same JAR." Source: <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/getting-started/paper-plugins.md>
- **When both are present, Paper loads the Paper plugin and ignores `plugin.yml`.** The server picks the first matching type from an ordered list, and `PAPER` is listed before `SPIGOT`:
  ```java
  private static final List<PluginFileType<?, ?>> VALUES = List.of(PAPER, SPIGOT);
  ```
  If neither entry exists the jar is rejected: `"does not contain a paper-plugin.yml or plugin.yml!"`. Source: <https://github.com/PaperMC/Paper/blob/main/paper-server/src/main/java/io/papermc/paper/plugin/provider/type/PluginFileType.java>
  Consequence for a scaffolder: shipping both is a legitimate dual-target strategy (Paper gets the modern path, Spigot falls back), but the two files' `commands` semantics differ — see §4.

### 1.4 Which does Paper recommend for **new** plugins?

Paper documents `paper-plugin.yml` as **Experimental** (sidebar badge `text: Experimental`, `variant: danger`; a `:::danger[Experimental]` admonition appears on both the Paper-plugins page and the `plugin.yml` field entries for `paper-plugin-loader` / `paper-skip-libraries`). The official project-setup guide's IDE field table describes the Paper manifest as:

> "**Paper Manifest** — Whether you want to use the new Paper plugins or not. **For now this is not recommended as it is still in development.**"

Sources: <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/getting-started/paper-plugins.md>, <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/getting-started/project-setup.mdx>

**So the canonical default for a 2026 scaffolder is `plugin.yml`**, with `paper-plugin.yml` offered as an opt-in for users who need bootstrappers/loaders, strict classloading isolation, or per-plugin dependency load ordering. (Note the trade-off: only `paper-plugin.yml` gives you `load: BEFORE/AFTER/OMIT` + `join-classpath` and the bootstrapper, which is required to register Brigadier commands early enough for datapacks — see §4.)

---

## 2. Java version requirements

**Current Paper requires Java 25.** The Paper docs requirements table maps every Paper line to its Java version:

| Paper Version | Recommended Java Version |
|---|---|
| 1.7.10 to 1.11 | Java 8 |
| 1.12 to 1.16.4 | Java 11 |
| 1.16.5 | Java 16 |
| 1.17 to 1.19 | Java 17 |
| 1.20 to 1.21.11 | Java 21 |
| **26.1+** | **Java 25** |

Source: <https://docs.papermc.io/paper/getting-started/> (rendered page, "Requirements" section).

Corroborated in the Paper server's own build:

```kotlin
extensions.configure<JavaPluginExtension> {
    toolchain {
        languageVersion = JavaLanguageVersion.of(25)
    }
}
// ...
tasks.withType<JavaCompile>().configureEach {
    options.release = 25
}
```

Source: <https://github.com/PaperMC/Paper/blob/main/build.gradle.kts>

**Legacy Spigot/Bukkit minimum.** From the same table, the lowest line Paper supports is 1.7.10, which runs on **Java 8** — that is the practical floor for a plugin that must load on legacy Bukkit/Spigot servers. For anything modern, the floor is set by the newest server you must support (Java 21 for 1.20–1.21.11; Java 25 for 26.1+).

Independent corroboration of the API's own bytecode baseline: the published `spigot-api` 26.3 artifact `org/bukkit/*.class` files are **class file major version 61 = Java 17** (so the *API surface* is compiled to Java 17 bytecode even though *servers* now require Java 25 to run). Artifact observed: <https://hub.spigotmc.org/nexus/content/repositories/snapshots/org/spigotmc/spigot-api/26.3-R0.1-SNAPSHOT/>.

**Scaffolder rule:** parameterize the toolchain as Java 25 for Paper 26.1+; Java 21 for 1.20–1.21.11; Java 17 for 1.17–1.19; Java 8 if you truly need 1.7.10–1.11 legacy support.

---

## 3. Build: canonical Gradle Kotlin DSL

The Paper docs' guide is explicit that Gradle + Kotlin DSL is the supported path: "The Paper team uses Gradle as its build system… this guide will only cover Gradle using the **Kotlin DSL**." Source: <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/getting-started/project-setup.mdx>

### 3.1 Versions to use (observed 2026-09-25)

| Component | Version | Source |
|---|---|---|
| Gradle wrapper | **9.8.0** | <https://services.gradle.org/versions/current> (buildTime `20260924134000`); Paper itself pins `gradle-9.8.0-bin.zip` in <https://github.com/PaperMC/Paper/blob/main/gradle/wrapper/gradle-wrapper.properties> |
| Paper API artifact | `io.papermc.paper:paper-api:26.2.build.+` | docs build script; artifact versions confirmed in the PaperMC Maven metadata <https://repo.papermc.io/repository/maven-public/io/papermc/paper/paper-api/maven-metadata.xml> |
| `xyz.jpenilla.run-paper` | **3.1.0** (released 2026-08-08) | <https://plugins.gradle.org/m2/xyz/jpenilla/run-paper/xyz.jpenilla.run-paper.gradle.plugin/maven-metadata.xml>; README <https://github.com/jpenilla/run-task> |
| `com.gradleup.shadow` | **9.6.1** (2026-07-22) | <https://repo1.maven.org/maven2/com/gradleup/shadow/shadow-gradle-plugin/maven-metadata.xml>; marker on the portal at `.../com/gradleup/shadow/com.gradleup.shadow.gradle.plugin/maven-metadata.xml` |
| `io.papermc.paperweight.userdev` | **2.0.0-beta.24** (2026-09-25) | <https://plugins.gradle.org/m2/io/papermc/paperweight/userdev/io.papermc.paperweight.userdev.gradle.plugin/maven-metadata.xml>; the docs page shows `2.0.0-beta.23` |

⚠️ **Shadow coordinates changed.** The plugin id is now **`com.gradleup.shadow`** (GradleUp), with versions in the `9.x` line — the old `com.github.johnrengelman.shadow` 8.x id is the superseded lineage. `com.gradleup.shadow` 9.0.0 is the first stable release of the renamed plugin; `9.6.1` is current.

### 3.2 Canonical `build.gradle.kts` (default scaffold — no NMS, no shading)

```kotlin
plugins {
    `java-library`
    id("xyz.jpenilla.run-paper") version "3.1.0"
}

group = "com.example"
version = "1.0.0-SNAPSHOT"
description = "An example Paper plugin"

repositories {
    mavenCentral()
    maven("https://repo.papermc.io/repository/maven-public/") {
        name = "papermc"
    }
}

dependencies {
    compileOnly("io.papermc.paper:paper-api:26.2.build.+")
}

java {
    toolchain.languageVersion = JavaLanguageVersion.of(25)
}

tasks {
    compileJava {
        options.encoding = Charsets.UTF_8.name()
        options.release = 25
    }
    processResources {
        val props = mapOf("version" to project.version)
        inputs.properties(props)
        filesMatching("plugin.yml") {
            expand(props)
        }
    }
    runServer {
        // Required configuration for run-paper: the server version to download.
        minecraftVersion("26.2")
    }
}
```

The `repositories`/`dependencies`/`java` block is verbatim the docs' recommendation:

```kotlin
repositories {
  maven(url = "https://repo.papermc.io/repository/maven-public/") {
    name = "papermc"
  }
}

dependencies {
  compileOnly("io.papermc.paper:paper-api:26.2.build.+")
}

java {
  toolchain.languageVersion.set(JavaLanguageVersion.of(25))
}
```

Sources: <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/getting-started/project-setup.mdx>, <https://docs.papermc.io/paper/dev/project-setup/>

`runServer { minecraftVersion("…") }` is the documented run-paper configuration — "This is the only required configuration besides applying the plugin. Your plugin's jar (or shadowJar if present) will be used automatically." Source: <https://github.com/jpenilla/run-task> (README).

The `processResources`/`expand` pattern for injecting the version into `plugin.yml` is exactly what Paper's own test plugin does:

```kotlin
tasks.processResources {
    val props = mapOf(
        "version" to project.version,
        "apiversion" to "\"${rootProject.providers.gradleProperty("apiVersion").get()}\"",
    )
    inputs.properties(props)
    filesMatching("paper-plugin.yml") {
        expand(props)
    }
}
```

Source: <https://github.com/PaperMC/Paper/blob/main/test-plugin/build.gradle.kts>

### 3.3 `settings.gradle.kts`, toolchain auto-provisioning, and the wrapper

Paper's own test plugin enables Foojay so Gradle can download a JDK 25 when the machine only has, say, JDK 17:

```kotlin
// settings.gradle.kts
plugins {
    id("org.gradle.toolchains.foojay-resolver-convention") version "1.0.0"
}
rootProject.name = "example-plugin"
```

Source: <https://github.com/PaperMC/Paper/blob/main/test-plugin/settings.gradle.kts> (that repo's `settings.gradle.kts`); the toolchain comment ("This allows gradle to auto-provision JDK 25 on systems that only have JDK 17 installed for example") is in <https://github.com/PaperMC/paperweight-test-plugin/blob/master/build.gradle.kts>

`gradle/wrapper/gradle-wrapper.properties` (generated by `gradle wrapper --gradle-version 9.8.0`):

```properties
distributionBase=GRADLE_USER_HOME
distributionPath=wrapper/dists
distributionUrl=https\://services.gradle.org/distributions/gradle-9.8.0-bin.zip
networkTimeout=10000
retries=0
retryBackOffMs=500
validateDistributionUrl=true
zipStoreBase=GRADLE_USER_HOME
zipStorePath=wrapper/dists
```

(Keys and layout copied from Paper's own wrapper file: <https://github.com/PaperMC/Paper/blob/main/gradle/wrapper/gradle-wrapper.properties>. The docs' build-tooling guidance is "Please make sure you are using the latest stable version of Gradle", which `services.gradle.org/versions/current` reports as 9.8.0.)

### 3.4 Shading and relocation — when, and with what

**Do not shade by default.** Two official mechanisms make it unnecessary:

1. `plugin.yml`'s **`libraries`** field: "This is a list of libraries that your plugin depends on. These libraries will be downloaded from the Maven Central repository and added to the classpath. **This removes the need to shade and relocate the libraries.**" Source: <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/getting-started/plugin-yml.mdx>
2. `paper-plugin.yml`'s **`loader`** + `PluginLoader#classloader`, adding `JarLibrary` or a `MavenLibraryResolver`. Source: <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/getting-started/paper-plugins.md>

If you *do* bundle (e.g. to support Spigot, which has neither mechanism), use `com.gradleup.shadow` 9.6.1:

```kotlin
plugins {
    id("com.gradleup.shadow") version "9.6.1"
}

tasks.shadowJar {
    // The plugin produces a normal jar alongside the shaded one;
    // run-paper picks up shadowJar automatically when present.
    archiveClassifier = ""
    // Relocate any bundled library that could clash with the server's own copy.
    relocate("com.example.libs", "com.example.plugin.libs")
    minimize()
}
```

`run-paper` detects `shadowJar` automatically (README, linked above). paperweight also detects Shadow: "If you have the shadow Gradle plugin applied in your build script, `paperweight-userdev` will detect that and use the shaded JAR as the input for the `reobfJar` task." Source: <https://docs.papermc.io/paper/dev/userdev/>

### 3.5 paperweight-userdev (only if the plugin touches server internals)

```kotlin
plugins {
    id("io.papermc.paperweight.userdev") version "2.0.0-beta.24"
}

dependencies {
    // The dev bundle includes the Paper API — remove the compileOnly paper-api dependency.
    paperweight.paperDevBundle("26.2.build.+")
}
```

The docs stress: "You should remove any dependency on the Paper API, as the dev bundle includes that." Plugin IDs and versions: <https://docs.papermc.io/paper/dev/userdev/> and the portal metadata above.

**26.1 removed reobfuscation entirely:** "From Minecraft version 26.1 onwards, Paper no longer supports obfuscated plugins due to Mojang themselves removing obfuscation of their server JAR. As Minecraft now ships unobfuscated, there are no 26.1 Spigot mappings to obfuscate to. For this reason, the reobfuscation function no longer works for dev bundles for 26.1+." Source: <https://docs.papermc.io/paper/dev/userdev/> — a scaffolder must not emit `reobfJar` wiring for MC ≥ 26.1.

---

## 4. Directory layout, main class, command, and event listener

### 4.1 Layout

```
example-plugin/
├── build.gradle.kts
├── settings.gradle.kts
├── gradle/
│   ├── libs.versions.toml          # optional
│   └── wrapper/
│       ├── gradle-wrapper.jar
│       └── gradle-wrapper.properties
├── gradlew
├── gradlew.bat
└── src/
    └── main/
        ├── java/
        │   └── com/example/paperplugin/
        │       ├── ExamplePlugin.java
        │       ├── ExampleListener.java
        │       └── ExampleCommand.java
        └── resources/
            └── plugin.yml            # or paper-plugin.yml
```

The `src/main/java` + `src/main/resources` split is the documented structure; the docs' `FileTree` shows exactly `src/ main/ java/ resources/` with the metadata file under `resources/`. Sources: <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/getting-started/plugin-yml.mdx>, <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/getting-started/project-setup.mdx>

Docs' own packaging/naming guidance for the scaffolder to encode:
- "When naming your packages, you should use your domain name in reverse order… If you do not have a domain name, you could use something like your GitHub username… `io.github.torvalds`."
- "It is considered good practice to **not** name this class `Main`. Instead, choose a more descriptive name, like your plugin's name."
- "do not call your plugin's constructor directly"; don't put logic in the constructor — use `onLoad`/`onEnable`.

Sources: project-setup.mdx (above), <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/getting-started/how-do-plugins-work.md>

### 4.2 Minimal main class (`JavaPlugin` + `onEnable`)

The docs' own example, which also doubles as the listener-registration pattern:

```java
package com.example.paperplugin;

import org.bukkit.event.EventHandler;
import org.bukkit.event.Listener;
import org.bukkit.event.player.PlayerJoinEvent;
import org.bukkit.plugin.java.JavaPlugin;

public final class ExamplePlugin extends JavaPlugin implements Listener {
    @Override
    public void onEnable() {
        this.getServer().getPluginManager().registerEvents(this, this);
    }

    @EventHandler
    public void onPlayerJoin(PlayerJoinEvent event) {
        event.getPlayer().sendPlainMessage("Hello, " + event.getPlayer().getName() + "!");
    }
}
```

Source: <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/getting-started/project-setup.mdx>

Lifecycle contract for `onEnable`/`onLoad`/`onDisable`: `onLoad` runs before most of the Bukkit API is available; `onEnable` runs before the server ticks and is the right place to register listeners, commands, open DB connections and start threads; `onDisable` is for cleanup. Source: <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/getting-started/how-do-plugins-work.md>

### 4.3 Event listener (declaration + registration)

```java
package com.example.paperplugin;

import org.bukkit.event.EventHandler;
import org.bukkit.event.EventPriority;
import org.bukkit.event.Listener;
import org.bukkit.event.player.PlayerJoinEvent;

public final class ExampleListener implements Listener {

    @EventHandler(priority = EventPriority.NORMAL, ignoreCancelled = true)
    public void onPlayerJoin(PlayerJoinEvent event) {
        event.getPlayer().sendRichMessage("<yellow>Welcome!");
    }
}
```

Registration, from `onEnable`:

```java
this.getServer().getPluginManager().registerEvents(new ExampleListener(), this);
```

**`plugin.yml` declaration needed for a listener: none.** Event listeners require no metadata entry — only registration. Sources: <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/api/event-api/event-listeners.md>, <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/getting-started/how-do-plugins-work.md>

Priorities are `LOWEST, LOW, NORMAL, HIGH, HIGHEST, MONITOR`; higher priority runs **later**; `MONITOR` must not mutate state. Source: event-listeners.md (above).

### 4.4 Sample command — the declaration differs by metadata format

**A. With `plugin.yml` (classic Bukkit path).** The command must be declared in `plugin.yml`:

```yaml
commands:
  example:
    description: "An example command"
    usage: "/example <arg>"
    aliases: [ex]
    permission: example.command
    permission-message: "You do not have permission to use this command"
```

Fields: `description`, `usage` (shown by `/help`), `aliases`, `permission` (players only see commands they may use), `permission-message`. Source: <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/getting-started/plugin-yml.mdx>

Implement it either as a `CommandExecutor`/`TabCompleter`, or — the modern Paper way — via Brigadier's `BasicCommand`, registered with `JavaPlugin#registerCommand`:

```java
package com.example.paperplugin;

import io.papermc.paper.command.brigadier.BasicCommand;
import io.papermc.paper.command.brigadier.CommandSourceStack;
import java.util.Collection;
import java.util.List;
import org.bukkit.Bukkit;
import org.bukkit.entity.Player;

public final class ExampleCommand implements BasicCommand {

    @Override
    public void execute(CommandSourceStack source, String[] args) {
        if (args.length == 0) {
            source.getSender().sendRichMessage("<red>Usage: /example <message>");
            return;
        }
        Bukkit.broadcast(net.kyori.adventure.text.Component.text(String.join(" ", args)));
    }

    @Override
    public String permission() {
        return "example.command";
    }

    @Override
    public Collection<String> suggest(CommandSourceStack source, String[] args) {
        return Bukkit.getOnlinePlayers().stream().map(Player::getName).toList();
    }
}
```

Registered in `onEnable`:

```java
@Override
public void onEnable() {
    this.registerCommand("example", new ExampleCommand());
}
```

`BasicCommand`'s overridable members are `execute(CommandSourceStack, String[])` (required) plus `suggest`, `canUse`, and `permission()` (optional). Overriding `canUse` makes `permission()` inert. `JavaPlugin#registerCommand(...)` is the documented registration call. Source: <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/api/command-api/misc/basic-command.md>

**B. With `paper-plugin.yml` (Paper path).** Docs, verbatim: "Paper plugins do **not** use the `commands` field to register commands. This means that you do not need to include all of your commands in the `paper-plugin.yml` file. Instead, you can register commands using the Brigadier Command API." Source: <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/getting-started/paper-plugins.md>

Registration goes through the Lifecycle API, either in the main class:

```java
@Override
public void onEnable() {
    this.getLifecycleManager().registerEventHandler(LifecycleEvents.COMMANDS, event -> {
        event.registrar().register("example", new ExampleCommand());
    });
}
```

or in a `PluginBootstrap` (which additionally makes commands usable by datapack functions earlier in startup, and requires `paper-plugin.yml`):

```java
public final class ExamplePluginBootstrap implements PluginBootstrap {
    @Override
    public void bootstrap(BootstrapContext context) {
        context.getLifecycleManager().registerEventHandler(LifecycleEvents.COMMANDS, event -> {
            event.registrar().register("example", new ExampleCommand());
        });
    }
}
```

Sources: <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/api/command-api/basics/registration.md>, <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/api/lifecycle/lifecycle.md>

> **The trap a scaffolder must handle:** the *same* logical command has two mutually exclusive declaration styles. Emitting `commands:` in `plugin.yml` **and** a `LifecycleEvents.COMMANDS` handler in `onEnable` double-registers; emitting only a Brigadier registration with `plugin.yml` and no `commands:` entry means Bukkit's command map never learns about it (and `/help` won't list it). Pick one path per metadata format.

### 4.5 Permissions

`plugin.yml` declares permission nodes so the server registers them and `/help`/permission listings know them:

```yaml
permissions:
  example.command:
    description: "Allows using /example"
    default: op
    children:
      example.command.extra: true
```

`default` accepts `op`/`notop`/`true`/`false`, falls back to `default-permission` (itself defaulting to `op`), and children inherit when set `true`. `paper-plugin.yml` uses the flattened defaults `defaultPerm` + `permissions`. Sources: <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/getting-started/plugin-yml.mdx>, <https://github.com/PaperMC/Paper/blob/main/paper-server/src/main/java/io/papermc/paper/plugin/provider/configuration/type/PermissionConfiguration.java>

---

## 5. Official template repository / generator

**There is no official Paper plugin template repository or scaffolding CLI.** This was verified two ways on 2026-09-25:

1. Enumerating every public repository in the `PaperMC` GitHub organisation (69 repos, pages 1–2 of `/orgs/PaperMC/repos?per_page=100`) yields **no** repository whose name or description is a plugin template/starter. The closest are:
   - `PaperMC/paperweight-test-plugin` — "test plugin for paperweight-userdev" (<https://github.com/PaperMC/paperweight-test-plugin>), last pushed 2026-04-16, default branch `master`, not archived. It is a **userdev test fixture**, not a general plugin template: its build pulls `paperweight.paperDevBundle("26.1.2.build.+")` and its metadata is generated by the third-party `xyz.jpenilla.resource-factory-bukkit-convention` plugin.
   - `PaperMC/paperweight-examples` — "examples" for paperweight, default branch `v2-fork`, last pushed 2026-07-14 (<https://github.com/PaperMC/paperweight-examples>).
2. GitHub search `org:PaperMC+template` returns `total_count: 0`.

**What Paper points users at instead**, in order of officialness:

| Option | What it is | Source |
|---|---|---|
| **Minecraft Development IntelliJ plugin** | Paper's recommended point-and-click project generator: `File > New > Project… > Minecraft`, with fields for Platform (`Paper`), Minecraft Version, Plugin Name, Main Class, Build System, Group ID, Artifact ID, JDK ("This can be anything from Java 25 and above") | <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/getting-started/project-setup.mdx> — and <https://plugins.jetbrains.com/plugin/8327-minecraft-development> |
| **Manual `New Project > Gradle - Kotlin DSL`** | The documented fallback; you create `src/main/{java,resources}` yourself | same |
| **Paper's own `test-plugin/` module** | The de-facto reference implementation of a Paper plugin inside the server repo: `test-plugin/src/main/resources/paper-plugin.yml` + `io.papermc.testplugin.TestPlugin`, `TestPluginBootstrap`, `TestPluginLoader` | <https://github.com/PaperMC/Paper/tree/main/test-plugin> |
| **`PaperMC/paperweight-test-plugin`** | Runnable userdev example | <https://github.com/PaperMC/paperweight-test-plugin> |
| **`xyz.jpenilla.resource-factory-bukkit-convention`** (third-party) | Generates `plugin.yml`/`paper-plugin.yml` from Gradle config — used *by* Paper's own test plugin, but not a PaperMC project | <https://github.com/PaperMC/paperweight-test-plugin/blob/master/build.gradle.kts> |

Paper's own `test-plugin/paper-plugin.yml` is worth reproducing in full, because it is the only first-party `paper-plugin.yml` shipped with the server:

```yaml
name: Paper-Test-Plugin
version: ${version}
main: io.papermc.testplugin.TestPlugin
description: Paper Test Plugin
author: PaperMC
api-version: ${apiversion}
load: STARTUP
bootstrapper: io.papermc.testplugin.TestPluginBootstrap
loader: io.papermc.testplugin.TestPluginLoader
defaultPerm: FALSE
permissions:
dependencies:
```

Source: <https://github.com/PaperMC/Paper/blob/main/test-plugin/src/main/resources/paper-plugin.yml> — note it uses `${...}` placeholders expanded by `processResources`, and an empty `permissions:`/`dependencies:` key is accepted.

**Implication for the CLI:** the value-add is real. A scaffolder earns its place by generating exactly what Paper does *not* ship: a runnable Gradle Kotlin DSL project with a correct toolchain, correct wrapper, correct metadata for the target format, and correct command/listener wiring for that format.

---

## 6. What a template-rendering CLI must parameterize

| Parameter | Type / values | Derivation | Why it's version-sensitive |
|---|---|---|---|
| `mcVersion` | e.g. `26.3`, `26.2`, `1.21.11` | Mojang manifest `latest.release` or user choice | Drives everything below |
| `apiVersion` | **Same string as `mcVersion`** (`26.2`, `26.1.2`, `1.21.11`) | MC version identifier | `build-data/.../apiVersion` is set to the MC version; `plugin.yml` accepts `1.13`–latest, `paper-plugin.yml` accepts `≥ 1.19` and **requires** the key |
| `paperApiVersion` | `{apiVersion}.build.+` | PaperMC Fill + Maven metadata | Version string format changed at 26.1: pre-26.1 used `{VERSION}-R0.1-SNAPSHOT` (no way to pin a build); 26.1+ uses `{VERSION}.build.{N}-{channel}` |
| `javaVersion` | `25` for 26.1+; `21` for 1.20–1.21.11; `17` for 1.17–1.19; `16` for 1.16.5; `11` for 1.12–1.16.4; `8` for 1.7.10–1.11 | Paper requirements table | Feeds `java { toolchain }`, `options.release`, and the run-paper/paperweight expectations |
| `gradleVersion` | `9.8.0` | `services.gradle.org/versions/current` | Wrapper pin; paperweight requires a recent stable Gradle |
| `metadataFormat` | `plugin.yml` \| `paper-plugin.yml` | User choice; default `plugin.yml` | Changes required fields, dependency schema, and command registration style |
| `runPaperVersion` | `3.1.0` | Gradle Plugin Portal | Also needs `runServer { minecraftVersion(...) }` — itself MC-version driven |
| `shadowVersion` | `9.6.1` (only if bundling) | Maven Central / portal | Coordinates changed to `com.gradleup.shadow` |
| `paperweightVersion` | `2.0.0-beta.24` (only if NMS) | Gradle Plugin Portal | Emit **no** `reobfJar` wiring for MC ≥ 26.1 (reobfuscation removed) |
| `pluginName` / `mainClass` / `packageName` / `group` / `artifactId` | strings | User input | `main` must match the generated class FQCN; docs require reverse-domain packages and warn against naming the class `Main` |
| `usesBootstrap` / `usesLoader` | bool | Only legal with `paper-plugin.yml` | Adds `bootstrapper`/`loader` keys and optional extra classes |
| `commandRegistrationStyle` | `plugin-yml-commands` \| `brigadier-lifecycle` | Implied by `metadataFormat` | The two are mutually exclusive; see §4.4 |
| `mcVersionTarget` for run-paper | same as `mcVersion` | — | `runServer.minecraftVersion("26.2")` |

**Resolving "latest Paper" is non-trivial and the CLI must replicate the docs' algorithm.** The Paper docs' own build resolves it in `src/utils/versions.ts`:

```ts
const findLatest = async (project: Project): Promise<string> => {
  const versions = Object.values(project.versions).flat();
  // find the newest version with at least one non-alpha build
  for (const version of versions) {
    const builds = await fetchBuilds(project, version);
    if (builds.some((b) => b.channel !== "ALPHA")) {
      return version;
    }
  }
  return versions[0];
};
```

At research time this is why the docs render `26.2` rather than the newest MC release `26.3`: the Fill API reports all 39 `26.3` builds as `ALPHA`, while `26.2` has 43 `STABLE` + 24 `BETA` builds. Source: <https://github.com/PaperMC/docs/blob/main/src/utils/versions.ts>; live data: <https://fill.papermc.io/v3/projects/paper> and <https://fill.papermc.io/v3/projects/paper/versions/26.3/builds>.

A scraper that blindly takes the newest version key from `fill.papermc.io/v3/projects/paper` will emit `api-version: '26.3'` and `paper-api:26.3.build.+`, which currently resolves only to `ALPHA` builds — a real footgun for a scaffolding CLI.

Other version-sensitive notes:
- **Version-string format broke at 26.1.** Docs tip: "If you want to reference a specific build, you can do so by replacing the + with the build identifier (i.e. `{VERSION}.build.25-stable` for the 25th stable build of a version). **Before `26.1` (`1.21.11` and below), the version string format used was `{VERSION}-R0.1-SNAPSHOT`, with no way to reference a specific build.**" Source: project-setup.mdx.
- **Minecraft itself switched to date-based versioning** (`26.1`, `26.2`, `26.3`), so any CLI logic that assumes `1.x.y` will break. Confirmed from Mojang's manifest: releases run `… 1.21.11, 26.1, 26.1.1, 26.1.2, 26.2, 26.3`. Source: <https://piston-meta.mojang.com/mc/game/version_manifest_v2.json>.
- **Paper's own `mcVersion`/`apiVersion` lives in `gradle.properties`** (`mcVersion=26.3`, `apiVersion=26.3`) — useful if the CLI ever validates against upstream. Source: <https://github.com/PaperMC/Paper/blob/main/gradle.properties>.

---

## Confidence / unverified

- **`paper-plugin.yml`'s exact unknown-key behaviour was not verified.** Configurate's `ObjectMapper` may or may not reject unrecognised keys; I did not find explicit `implicitInitialization`/strictness configuration in `PaperPluginMeta`. A scaffolder should not emit speculative keys. *Unverified.*
- **Paper's refusal to load a `plugin.yml` whose `api-version` exceeds the server** is stated in the docs ("Servers with a version lower than the version specified here will refuse to load the plugin") but I did not read the enforcement code path. Treated as documented behaviour, not source-verified.
- **Spigot's own stated Java requirements could not be fetched** — `spigotmc.org` (including the wiki) and `spigotmc.org/wiki/buildtools` return HTTP 403 (Cloudflare) from this network. The Java-per-version table used in §2 is therefore **Paper's**; it is authoritative for Paper and is the best available primary substitute for Spigot/Bukkit legacy floors. Note also that `hub.spigotmc.org` *was* reachable, and the `spigot-api` 26.3 artifact exists and compiles to Java 17 bytecode (observed directly), which is evidence Spigot is still actively published for 26.3 — but Spigot's own minimum *runtime* JDK is not independently confirmed here.
- **The Gradle Plugin Portal's JSON API rejected every version string I tried** (`8.0`, `8.10`, `8.14`, `9.0`, `9.0.0`, `9.6`, `9.8.0` all returned `INVALID_GRADLE_VERSION`). Plugin versions in §3.1 are therefore taken from the portal's own `m2` repository metadata (`plugins.gradle.org/m2/...maven-metadata.xml`), which publishes `<latest>`/`<release>`/`<lastUpdated>` and is equally primary.
- **`paperweight-userdev` docs page vs. portal disagree by one version** (`2.0.0-beta.23` in the docs body vs. `2.0.0-beta.24` as the portal's `<latest>`/`<release>`). The portal is the artifact registry and is newer; the CLI should prefer resolving the latest at render time rather than hard-coding either.
- **`xyz.jpenilla.run-paper` 3.1.0's compatibility range with Gradle 9.8.0 was not explicitly verified**; the README documents 3.x usage generically and Paper's own test plugin uses 3.0.2 with Gradle 9.4.1. Verify by running `./gradlew runServer` in a generated project before shipping the template.
- **`docs.papermc.io` renders DB-sourced versions at build time.** The site's rendered `plugin.yml` page shows `api-version: '26.2'` today because of the `findLatest` algorithm in §6; that value will change without any docs commit. A CLI must not scrape the rendered page for versions — query `fill.papermc.io` instead.

---

## Sources

**PaperMC official documentation (source Markdown in `PaperMC/docs`, `main` branch — the files the site is built from):**
- <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/getting-started/plugin-yml.mdx> — `plugin.yml` field schema, requirements, commands, permissions, libraries
- <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/getting-started/paper-plugins.md> — `paper-plugin.yml`, bootstrapper, loader, dependencies, differences (Experimental)
- <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/getting-started/project-setup.mdx> — Gradle Kotlin DSL setup, toolchain, `src` layout, main class, IDE generator table
- <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/getting-started/how-do-plugins-work.md> — plugin lifecycle, commands, permissions, scheduling
- <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/api/event-api/event-listeners.md> — `Listener`, `@EventHandler`, registration, priorities
- <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/api/command-api/basics/registration.md> — Brigadier registration via `LifecycleEventManager`
- <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/api/command-api/misc/basic-command.md> — `BasicCommand`, `JavaPlugin#registerCommand`
- <https://github.com/PaperMC/docs/blob/main/src/content/docs/paper/dev/api/lifecycle/lifecycle.md> — `LifecycleEvents.COMMANDS`
- <https://github.com/PaperMC/docs/blob/main/src/utils/versions.ts> — the `findLatest` "newest non-ALPHA build" algorithm

**PaperMC official documentation (rendered site):**
- <https://docs.papermc.io/paper/getting-started/> — Java version per Paper version table
- <https://docs.papermc.io/paper/dev/project-setup/> — resolved `paper-api` version, toolchain 25
- <https://docs.papermc.io/paper/dev/plugin-yml/> · <https://docs.papermc.io/paper/dev/getting-started/paper-plugins/> · <https://docs.papermc.io/paper/dev/userdev/>

**Paper server + test plugin source (`PaperMC/Paper`, `main`):**
- <https://github.com/PaperMC/Paper/blob/main/paper-api/src/main/java/org/bukkit/plugin/PluginDescriptionFile.java> — required `plugin.yml` entries + full field table
- <https://github.com/PaperMC/Paper/blob/main/paper-server/src/main/java/io/papermc/paper/plugin/provider/configuration/PaperPluginMeta.java> — authoritative `paper-plugin.yml` schema, `@Required` fields, `MINIMUM = 1.19`
- <https://github.com/PaperMC/Paper/blob/main/paper-server/src/main/java/io/papermc/paper/plugin/provider/configuration/type/DependencyConfiguration.java> — `load`/`required`/`join-classpath` + defaults
- <https://github.com/PaperMC/Paper/blob/main/paper-server/src/main/java/io/papermc/paper/plugin/provider/configuration/type/PluginDependencyLifeCycle.java> — `BOOTSTRAP`/`SERVER`
- <https://github.com/PaperMC/Paper/blob/main/paper-server/src/main/java/io/papermc/paper/plugin/provider/configuration/type/PermissionConfiguration.java> — `defaultPerm`/`permissions`
- <https://github.com/PaperMC/Paper/blob/main/paper-server/src/main/java/io/papermc/paper/plugin/provider/type/PluginFileType.java> — `PAPER` before `SPIGOT`: Paper wins when both metadata files exist
- <https://github.com/PaperMC/Paper/blob/main/build.gradle.kts> — `JavaLanguageVersion.of(25)`, `options.release = 25`
- <https://github.com/PaperMC/Paper/blob/main/gradle.properties> — `mcVersion=26.3`, `apiVersion=26.3`
- <https://github.com/PaperMC/Paper/blob/main/gradle/wrapper/gradle-wrapper.properties> — Gradle 9.8.0 pin
- <https://github.com/PaperMC/Paper/blob/main/test-plugin/build.gradle.kts> — `processResources`/`expand` pattern
- <https://github.com/PaperMC/Paper/blob/main/test-plugin/src/main/resources/paper-plugin.yml> — first-party `paper-plugin.yml` example

**Version registries / APIs (read 2026-09-25):**
- <https://piston-meta.mojang.com/mc/game/version_manifest_v2.json> — `latest.release = 26.3`, `latest.snapshot = 26.4-snapshot-1`
- <https://fill.papermc.io/v3/projects/paper> · <https://fill.papermc.io/v3/projects/paper/versions/26.3/builds> — Paper versions and build channels
- <https://services.gradle.org/versions/current> — Gradle 9.8.0
- <https://repo.papermc.io/repository/maven-public/io/papermc/paper/paper-api/maven-metadata.xml> — `paper-api` version strings
- <https://plugins.gradle.org/m2/xyz/jpenilla/run-paper/xyz.jpenilla.run-paper.gradle.plugin/maven-metadata.xml> — run-paper 3.1.0
- <https://plugins.gradle.org/m2/io/papermc/paperweight/userdev/io.papermc.paperweight.userdev.gradle.plugin/maven-metadata.xml> — paperweight-userdev 2.0.0-beta.24
- <https://plugins.gradle.org/m2/com/gradleup/shadow/com.gradleup.shadow.gradle.plugin/maven-metadata.xml> · <https://repo1.maven.org/maven2/com/gradleup/shadow/shadow-gradle-plugin/maven-metadata.xml> — shadow 9.6.1
- <https://hub.spigotmc.org/nexus/content/repositories/snapshots/org/spigotmc/spigot-api/maven-metadata.xml> · <https://hub.spigotmc.org/nexus/content/repositories/snapshots/org/spigotmc/spigot-api/26.3-R0.1-SNAPSHOT/> — Spigot API still published for 26.3; class file major version 61 (Java 17) observed directly in `spigot-api-26.3-R0.1-20260923.104027-6.jar`

**Gradle plugin repositories:**
- <https://github.com/jpenilla/run-task> — run-paper README, `minecraftVersion(...)` usage
- <https://github.com/PaperMC/paperweight> — paperweight (default branch `main`)
- <https://github.com/PaperMC/paperweight-test-plugin> — userdev example plugin; `build.gradle.kts`, `settings.gradle.kts`, `gradle-wrapper.properties` (default branch `master`)
- <https://github.com/PaperMC/paperweight-examples> — paperweight examples (default branch `v2-fork`)
- <https://github.com/PaperMC/Paper/tree/main/test-plugin> — first-party test plugin module

**Repository/org enumeration used to establish that no official template exists:**
- <https://api.github.com/orgs/PaperMC/repos?per_page=100&page=1> · <https://api.github.com/orgs/PaperMC/repos?per_page=100&page=2> (69 repos)
- <https://github.com/search?q=org%3APaperMC+template&type=repositories> — `total_count: 0`
