# Canonical Shape of a Modern Velocity / BungeeCord Proxy Plugin Project (2026)

**Research date:** 2026-09-25.
**Ticket:** vinoa wayfinder #6 — 代理端插件工程形态（Velocity / BungeeCord）.
**Scope:** what a scaffolding CLI should generate for a new Minecraft *proxy* plugin, on Velocity and/or BungeeCord, and every version-sensitive knob it must parameterize.
**Method / evidence rules:** every claim is traced to a primary source — `docs.papermc.io` (the rendered page), the PaperMC artifact repository (`repo.papermc.io`) for published coordinates and bytecode facts, the PaperMC Fill API (`fill.papermc.io`) for support/Java matrices, the owning source code in `PaperMC/Velocity` and `SpigotMC/BungeeCord`, and the Gradle Plugin Portal / `services.gradle.org`. Blog posts and tutorials are not used. Claims that were checked by compiling a throwaway plugin are marked **[verified locally]**.

> **Environment caveats that affect citations.**
> - `github.com` HTML pages and `api.github.com` were intermittently unreachable or rate-limited from this workspace. Content was therefore fetched from `raw.githubusercontent.com` and from **published jars on `repo.papermc.io`** (`*-sources.jar`, `*.jar`, `*.pom`, `*.module`), which are the exact bytes shipped to users — a *stronger* primary source than a repo tree. Where a source link is to `github.com/.../blob/...`, the identical file was read from the sources jar first.
> - `spigotmc.org` (including its wiki) is not usable from this network; BungeeCord claims are sourced from the BungeeCord **source code and published API jar** instead. This is flagged in "Confidence / unverified".
> - The Velocity repository's default development branch is `dev/3.0.0`; its `master` branch is the retired 1.x line (Gradle 5.4 wrapper, no `paper-plugin.yml` handling). All Velocity source links below are `dev/3.0.0` unless stated. The `velocity-api 4.x` artifact itself is published from the release branch, so line numbers may drift slightly between the branch and the artifact — both were cross-checked where it mattered.

---

## 0. TL;DR — the canonical shape

### 0.1 Version matrix (all verified 2026-09-25)

| Item | Current value | Source |
|---|---|---|
| Velocity (proxy) latest stable | **4.2.0** (build 30, released 2026-09-14, channel `STABLE`) | Fill API `/v3/projects/velocity/versions` |
| Velocity newest published line | **4.2.1-SNAPSHOT** (status `SUPPORTED`) | Fill API `/v3/projects/velocity` |
| `velocity-api` Maven release | **4.2.0** (`<release>`); newest version is **4.2.1-SNAPSHOT** | `repo.papermc.io` maven-metadata |
| Velocity Java minimum | **Java 25** for 4.0.x+ (all `4.x` builds report `java.minimum = 25`) | docs FAQ + Fill API |
| `velocity-api` bytecode target | **class file major 69 = Java 25**; Gradle module metadata `org.gradle.jvm.version = 25` | `velocity-api-4.2.0.jar`, `velocity-api-4.2.0.module` |
| Gradle current stable | **9.8.0** (built 2026-09-24) | `services.gradle.org/versions/current` |
| Velocity's own Gradle wrapper | 9.3.0 (`dev/3.0.0`) — not the recommendation | `gradle-wrapper.properties` |
| Velocity's own build JDK | 21 toolchain (`dev/3.0.0` root build) — the *published* 4.x API is Java 25 | `build.gradle.kts` @ `dev/3.0.0` |
| `xyz.jpenilla.run-velocity` | **3.1.0** (latest; 2026-08-08) | Gradle Plugin Portal metadata |
| BungeeCord API latest snapshot | **26.1-R0.1-SNAPSHOT** (build 43, 2026-09-15 21:08:55 UTC) | `bungeecord-api` maven-metadata |
| BungeeCord API latest release | **1.21-R0.4** | `bungeecord-api` maven-metadata |
| BungeeCord Java minimum | **Java 17** on current snapshots (`maven.compiler.release = 17`; published classes are major **61** = Java 17). The `1.21-R0.4` release was still Java 8 (`release = 8`, classes major 52) | root `pom.xml`, published jars, parent poms |
| BungeeCord maintained in 2026? | **Yes, but legacy.** Snapshots shipped 2026-09-15; README copyright "2012-2026 SpigotMC Pty. Ltd." PaperMC docs: "strongly recommends using Velocity over Waterfall and BungeeCord… All future development by PaperMC is done on Velocity"; the comparison table marks BungeeCord "Actively developed ❓" and Waterfall ❌ | `SpigotMC/BungeeCord` README; docs `comparisons-to-other-proxies` |

### 0.2 Minimum-file skeleton (Velocity only — the default a scaffold should emit)

```
<project>/
  settings.gradle.kts              # rootProject.name
  build.gradle.kts                 # see §2.2
  gradle/wrapper/gradle-wrapper.properties   # gradle-9.8.0-bin.zip
  gradle/wrapper/gradle-wrapper.jar
  gradlew, gradlew.bat
  src/main/java/<pkg>/<Name>Plugin.java      # @Plugin + @Subscribe ProxyInitializeEvent
  README.md, .gitignore, LICENSE
```

There is **no required resource file**: the descriptor is *generated*. Compiling the main class with the annotation processor on the processor path produces `build/classes/java/main/velocity-plugin.json`, which Gradle's `jar` task copies verbatim to the **root of the jar**. **[verified locally]** (compiled a throwaway class against `velocity-api` 3.5.1 with `-processorpath velocity-api:guava:gson`; the processor emitted exactly this at CLASS_OUTPUT root, i.e. jar root):

```json
{"id":"demo","name":"Demo","version":"1.0.0","description":"demo","url":"https://example.org","authors":["Me"],"dependencies":[{"id":"wonderplugin","optional":true}],"main":"com.example.demo.Test"}
```

### 0.3 Decision summary

| Decision | Recommendation for 2026 | Why |
|---|---|---|
| Velocity metadata | **`@Plugin` annotation + annotation processor**; do **not** hand-write `velocity-plugin.json` | The processor is shipped and auto-registered via `META-INF/services`; the descriptor format is derived from the annotation, so drift is impossible. Hand-writing is only a fallback for non-Java build pipelines (§1.3) |
| Descriptor location | **jar root**, `velocity-plugin.json` | `JavaPluginLoader` scans jar entries by that exact name; a jar containing only `plugin.yml`/`bungee.yml`/`paper-plugin.yml` is **rejected** with "Velocity does not support plugins from these platforms" (§4) |
| Java | **Java 25 toolchain for Velocity 4.x**; 21 for the deprecated Velocity 3.5.x | Fill API + FAQ + published bytecode |
| Gradle | **Kotlin DSL**, wrapper **9.8.0**, `java` plugin + `toolchain.languageVersion = 25` | Kotlin DSL is the docs' first tab; Gradle 9.8.0 is current stable |
| Velocity dependency scope | `compileOnly` + `annotationProcessor` (never `implementation`, never shaded) | The API is provided by the proxy; docs' own snippet |
| Dual-platform | **One jar with two entry points and two descriptors is valid and is the practical way to do it** — but it is *not* a Velocity/Bungee plugin in the Bungee sense, and it triples your surface | §4 |
| BungeeCord as a target in 2026 | Support it for reach, but make Velocity the default and say out loud that BungeeCord is legacy | PaperMC's own recommendation |
| Plugin id | Must match `[a-z][a-z0-9-_]{0,63}` and is validated by the processor and by the loader | `SerializedPluginDescription.ID_PATTERN` |

---

## 1. Velocity plugin descriptor and main class

### 1.1 `@Plugin` — the full annotation schema

Source: `com.velocitypowered.api.plugin.Plugin` (published in `velocity-api-4.2.0-sources.jar`; mirrored at <https://github.com/PaperMC/Velocity/blob/dev/3.0.0/api/src/main/java/com/velocitypowered/api/plugin/Plugin.java>).

| Element | Type | Required | Notes |
|---|---|---|---|
| `id()` | `String` | **yes** | Must match `[a-z][a-z0-9-_]{0,63}` (annotated `@Pattern`) |
| `name()` | `String` | no (default `""`) | Human-readable display name |
| `version()` | `String` | no (default `""`) | Docs recommend semver |
| `description()` | `String` | no (default `""`) | |
| `url()` | `String` | no (default `""`) | |
| `authors()` | `String[]` | no (default `""`) | Empty strings are filtered out |
| `dependencies()` | `Dependency[]` | no | `@Dependency(id=..., optional=...)` |
| `provides()` | `String[]` | no | IDs this plugin provides; each must match the id pattern |

`@Plugin` is `@Retention(RUNTIME) @Target(TYPE)`. `@Dependency` is `@Retention(RUNTIME) @Target({})` (i.e. only usable nested inside `@Plugin`) and has `id()` (required, id-pattern) and `optional()` (default `false`).

The maximum-length/pattern rule is defined once, in `SerializedPluginDescription`:

```java
public static final String ID_PATTERN_STRING = "[a-z][a-z0-9-_]{0,63}";
```

Source: <https://github.com/PaperMC/Velocity/blob/dev/3.0.0/api/src/main/java/com/velocitypowered/api/plugin/ap/SerializedPluginDescription.java>. The processor emits a hard compile error for a bad id, a bad dependency id, or a bad `provides` id, and only a *warning* if more than one `@Plugin` class is found ("Velocity does not yet currently support multiple plugins") — see `PluginAnnotationProcessor#process`.

### 1.2 `velocity-plugin.json` — the full field schema

The serialized form is `SerializedPluginDescription`; `// @Nullable is used here to make GSON skip these in the serialized file`, which is why a minimal descriptor is only `id` and `main`.

| JSON field | Required in file | Type | From annotation |
|---|---|---|---|
| `id` | **yes** | string | `id` |
| `main` | **yes** | string | *not from the annotation* — the processor fills in the annotated class's **fully qualified name** |
| `name`, `version`, `description`, `url` | no | string, omitted when empty | same-named elements |
| `authors` | no | array of string | `authors` |
| `dependencies` | no | array of `{"id": string, "optional": boolean}` | `dependencies` |
| `provides` | no | array of string | `provides` |

No other keys exist. There is no `api-version`, no `load-order`, no `website`, no `commands`, no `permissions` in a Velocity descriptor.

Where it lives: `PluginAnnotationProcessor` calls

```java
environment.getFiler().createResource(StandardLocation.CLASS_OUTPUT, "", "velocity-plugin.json");
```

so it is written to the **root of the compiled classes output**, and therefore to the **root of the jar** after `jar`/`shadowJar`. Source: <https://github.com/PaperMC/Velocity/blob/dev/3.0.0/api/src/main/java/com/velocitypowered/api/plugin/ap/PluginAnnotationProcessor.java>. Loader confirmation: `JavaPluginLoader#getSerializedPluginInfo` switches on `entry.getName()` and matches `case "velocity-plugin.json"` at the jar root, parsing it with `VelocityServer.GENERAL_GSON`. Source: <https://github.com/PaperMC/Velocity/blob/dev/3.0.0/proxy/src/main/java/com/velocitypowered/proxy/plugin/loader/java/JavaPluginLoader.java>.

### 1.3 Which is preferred: annotation or handwritten descriptor?

**The annotation.** Rationale from primary sources:

- PaperMC's own project-setup guide tells you to create a class that "should have the `@Plugin` annotation" and never mentions `velocity-plugin.json` as something you author: <https://docs.papermc.io/velocity/dev/creating-your-first-plugin/>.
- The processor is self-registering (`@AutoService(Processor.class)` → `META-INF/services/javax.annotation.processing.Processor` **is present in the published jar**, and `META-INF/gradle/incremental.annotation.processors` too), so `annotationProcessor(...)` is enough.
- The processor is the *only* documented producer of the file; there is no official hand-authoring guide.
- If the processor does not run, the plugin simply fails to load with `InvalidPluginException("Did not find a valid velocity-plugin.json.")`.

Hand-writing is a legitimate fallback when the build is not javac (e.g. a non-JVM build front-end), because the JSON schema is fixed and small — but a scaffold should not mix both, to avoid two producers.

### 1.4 Minimal main class (Velocity 4.x)

From the official basics page, and consistent with the API sources (<https://docs.papermc.io/velocity/dev/api-basics/>):

```java
package com.example.velocityplugin;

import com.google.inject.Inject;
import com.velocitypowered.api.event.Subscribe;
import com.velocitypowered.api.event.proxy.ProxyInitializeEvent;
import com.velocitypowered.api.plugin.Plugin;
import com.velocitypowered.api.proxy.ProxyServer;
import org.slf4j.Logger;

@Plugin(id = "myfirstplugin", name = "My First Plugin", version = "0.1.0-SNAPSHOT",
    url = "https://example.org", description = "I did it!", authors = {"Me"})
public class VelocityTest {

  private final ProxyServer server;
  private final Logger logger;

  @Inject
  public VelocityTest(ProxyServer server, Logger logger) {
    this.server = server;
    this.logger = logger;
  }

  @Subscribe
  public void onProxyInitialization(ProxyInitializeEvent event) {
    // register commands / listeners here — NOT in the constructor
  }
}
```

Two things a template must get right:

1. **Do not do API work in the constructor.** Docs: plugin loading is two-phase; "you can't register an event listener in your constructor, because you need to have a valid plugin registration" — wait for `ProxyInitializeEvent`. Source: <https://docs.papermc.io/velocity/dev/api-basics/>.
2. **The main class is auto-registered as a listener.** Same page. Extra listeners need `server.getEventManager().register(this, listener)`.

Injectable constructor parameters used by templates: `ProxyServer`, `org.slf4j.Logger`, `@DataDirectory java.nio.file.Path`, and `com.velocitypowered.api.command.CommandManager` (the command docs say it can be injected directly: <https://docs.papermc.io/velocity/dev/command-api/>).

### 1.5 The class does not need to extend or implement anything

`@Plugin` may be placed on any class; the loader only requires a no-arg-constructible/injectable main class. There is no `Plugin` base class or `onEnable` in the Velocity API (that is the Bungee/Bukkit pattern).

---

## 2. Velocity build setup

### 2.1 Required Java and Gradle

- Java: **25**. docs: "Velocity 4.0.x and above requires at least Java 25" (<https://docs.papermc.io/velocity/faq/>), "Make sure your Project JDK is Java 25 or later" and "This can be anything from Java 25 and above" (<https://docs.papermc.io/velocity/dev/creating-your-first-plugin/>). Fill API independently reports `java.minimum: 25` for 4.0.0 through 4.2.1-SNAPSHOT and `21` for the 3.5.x line (<https://fill.papermc.io/v3/projects/velocity/versions>). The published API jar is Java 25 bytecode.
- Gradle wrapper: **9.8.0** is the current stable release (<https://services.gradle.org/versions/current>, built 2026-09-24). Note Velocity's own repo still pins `gradle-9.3.0-bin.zip` on `dev/3.0.0` — checkout state, not a recommendation. A scaffold should pin the current stable and allow override.

### 2.2 Canonical `build.gradle.kts`

The repository and dependency lines are verbatim from the official Gradle (Kotlin) tab, except that the docs default to the `-SNAPSHOT` version while a scaffold should default to the **stable release** (`4.2.0`) with a documented opt-in to snapshots:

```kotlin
plugins {
    java
}

java {
    toolchain.languageVersion = JavaLanguageVersion.of(25)
}

repositories {
    mavenCentral()
    maven(url = "https://repo.papermc.io/repository/maven-public/") {
        name = "papermc"
    }
}

dependencies {
    compileOnly("com.velocitypowered:velocity-api:4.2.0")
    annotationProcessor("com.velocitypowered:velocity-api:4.2.0")
}

tasks.withType<JavaCompile>().configureEach {
    options.encoding = "UTF-8"
    options.release = 25
}
```

Source for the repository URL, the `velocity-api` coordinate, and the `compileOnly` + `annotationProcessor` pairing: <https://docs.papermc.io/velocity/dev/creating-your-first-plugin/> (the docs currently show `4.2.1-SNAPSHOT`). The same page's Maven tabs use `<scope>provided</scope>` plus either a `type=processor` dependency or `maven-compiler-plugin` `<annotationProcessorPaths>`.

**JDK 23+ caveat (Maven only):** the docs carry an explicit caution that with JDK 23 or later the annotation processor must be declared explicitly; Gradle's `annotationProcessor` configuration already does that.

**Descriptor into the jar.** Nothing extra is needed for the normal case: the processor writes `velocity-plugin.json` into the classes output, and `jar` picks it up. `processResources` is *not* used. If a project also uses `shadowJar`, the file must still end up at the jar root exactly once (the processor's file is inside the shadowed `jar` output already).

**Running it locally.** The docs point at the Run-Task plugin: "you can use the Run-Task Gradle plugin. It will automatically download a Velocity server and run it for you." The Velocity variant is `xyz.jpenilla.run-velocity`, latest **3.1.0** (<https://plugins.gradle.org/m2/xyz/jpenilla/run-velocity/xyz.jpenilla.run-velocity.gradle.plugin/maven-metadata.xml>).

**Alternative generator.** IntelliJ's *Minecraft Development* plugin can generate a Velocity project ("Platform Type: Plugin, Platform: Velocity, Velocity Version: …, Main Class … should have the `@Plugin` annotation"); Paper "recommends using Gradle". Source: <https://docs.papermc.io/velocity/dev/creating-your-first-plugin/>. **There is no official PaperMC Velocity plugin template repository** — a GitHub org search for `org:PaperMC template` returns 0 results (2026-09-25), matching the finding in the Paper-side research note.

### 2.3 Third-party dependency handling (what templates must warn about)

Docs, "External dependencies": "Please remember to relocate any dependencies you shade. Failure to relocate will lead to dependency conflicts with other plugins." Velocity does not resolve plugin libraries for you (contrast: Paper's `libraries:` field). `PluginManager#addToClasspath` exists for the attach-from-directory workflow. Source: <https://docs.papermc.io/velocity/dev/dependency-management/>.

---

## 3. BungeeCord

### 3.1 Descriptor: `plugin.yml`, with `bungee.yml` as an accepted alias

`PluginManager#detectPlugins` looks for **`bungee.yml` first, then `plugin.yml`**, and fails the jar if neither exists:

```java
JarEntry pdf = jar.getJarEntry( "bungee.yml" );
if ( pdf == null ) { pdf = jar.getJarEntry( "plugin.yml" ); }
Preconditions.checkNotNull( pdf, "Plugin must have a plugin.yml or bungee.yml" );
```

Then it does `yaml.loadAs(in, PluginDescription.class)` and asserts `name` and `main` are non-null. Source (published `bungeecord-api` sources): <https://github.com/SpigotMC/BungeeCord/blob/master/api/src/main/java/net/md_5/bungee/api/plugin/PluginManager.java> (also in `bungeecord-api-26.1-R0.1-…-sources.jar`).

`PluginDescription` *is* the schema — its javadoc says "POJO representing the plugin.yml file":

| YAML key | POJO field | Required | Notes |
|---|---|---|---|
| `name` | `name` | **yes** (checked non-null) | identity in the dependency graph |
| `main` | `main` | **yes** (checked non-null) | "Needs to extend `net.md_5.bungee.api.plugin.Plugin`" |
| `version` | `version` | effectively yes | printed in the enable log line |
| `author` | `author` | no | single string only (no `authors` list) |
| `depends` | `depends` (Set) | no | hard dependency |
| `softdepends` | `softDepends` (Set) | no | soft dependency; `softdepends` is the YAML spelling of the field `softDepends` |
| `description` | `description` | no | |
| `libraries` | `libraries` (List) | no | runtime-resolved library GAVs — this is BungeeCord's answer to shading |

Notable absences: there is **no `website` field** in the POJO in any version checked, no `commands`/`permissions` block (commands are registered from code), and no `api-version`. Source (latest snapshot): `net/md_5/bungee/api/plugin/PluginDescription.java` in `bungeecord-api-26.1-R0.1-20260915.210855-43-sources.jar` — **byte-identical** to the `1.21-R0.4` copy, i.e. the descriptor schema has been frozen since 1.21.

Minimal `plugin.yml`:

```yaml
name: Example-Plugin
main: com.example.bungeeplugin.ExamplePlugin
version: 1.0.0
author: Me
description: An example BungeeCord plugin
```

### 3.2 Main class pattern

`net.md_5.bungee.api.plugin.Plugin` is a concrete base class with `onLoad()` and `onEnable()` no-op hooks and `onDisable()`, plus `getProxy()`, `getDescription()`, `getDataFolder()`, `getLogger()`, `getResourceAsStream(name)`:

```java
package com.example.bungeeplugin;

import net.md_5.bungee.api.plugin.Plugin;

public final class ExamplePlugin extends Plugin {

  @Override
  public void onEnable() {
    getProxy().getPluginManager().registerCommand(this, new ExampleCommand());
    getProxy().getPluginManager().registerListener(this, new ExampleListener());
  }
}
```

Sources: `Plugin.java` and `PluginManager.java` in the published `bungeecord-api` sources jar; <https://github.com/SpigotMC/BungeeCord/blob/master/api/src/main/java/net/md_5/bungee/api/plugin/Plugin.java>. The base-class constructor does `Preconditions.checkState(classLoader instanceof PluginClassloader, ...)`, so a plugin main class must genuinely be loaded from the plugin jar.

### 3.3 Build / dependency coordinates

```
repository: https://repo.papermc.io/repository/maven-public/     (id "papermc")
groupId:    net.md-5
artifactId: bungeecord-api
scope:      compileOnly / provided
version:    1.21-R0.4               (latest release)
         or 26.1-R0.1-SNAPSHOT     (latest line; currently build 43, 2026-09-15)
```

Repo/coordinates verified from `bungeecord-api` maven-metadata and the artifact tree. There is **no BungeeCord BOM** (`net/md-5/bungeecord-bom/maven-metadata.xml` → 404), and `bungeecord-api` brings the sibling modules (`bungeecord-chat`, `-config`, `-event`, `-protocol`, `-dialog`, `-maven-resolver`, plus Netty) as transitive `compile` dependencies. The snapshot version embeds a `-SNAPSHOT` literal in the coordinate; the resolved timestamped build is e.g. `26.1-R0.1-20260915.210855-43`. **There is no official BungeeCord Gradle plugin** — a plain `java` project is the canonical shape; `plugin.yml` goes in `src/main/resources/`.

### 3.4 Java version

- Current line: **Java 17**. `SpigotMC/BungeeCord` root `pom.xml` sets `<maven.compiler.release>17</maven.compiler.release>` (<https://github.com/SpigotMC/BungeeCord/blob/master/pom.xml>), and the published `26.1-R0.1-…-43` classes are class-file major **61** (Java 17).
- Last release `1.21-R0.4`: **Java 8**. Its parent pom declares `maven.compiler.source/target 1.8` / `release 8`, and its classes are major **52**.
- Practical consequence for a scaffold: a single jar that also targets Velocity 4.x must be built with **Java 25**, which is fine for BungeeCord (newer bytecode, Java 17+ runtime); the reverse (building at 17 for Bungee and shipping to Velocity) is impossible.

### 3.5 Is BungeeCord still maintained / recommended in 2026?

Evidence:

- **Actively shipped.** The newest published snapshot is build 43 of `26.1-R0.1-SNAPSHOT`, `lastUpdated 20260915210855` — twelve days before this research. README footer: "(c) 2012-2026 SpigotMC Pty. Ltd." (<https://github.com/SpigotMC/BungeeCord/blob/master/README.md>).
- **Not recommended by PaperMC.** Velocity docs, "Comparing with other proxies": "**Danger** — The Paper team strongly recommends using Velocity over Waterfall and BungeeCord. Waterfall has reached end of life. All future development by PaperMC is done on Velocity." The feature table marks BungeeCord "Actively developed ❓" versus Velocity ✅, and "Velocity plugins ❌" for BungeeCord; "BungeeCord plugins ✅". Same page documents that the Velocity API cannot run BungeeCord plugins (the third-party `Snap` shim is "not maintained by, or affiliated with, the Velocity project"). Source: <https://docs.papermc.io/velocity/comparisons-to-other-proxies/>.
- **Waterfall, the Paper fork of BungeeCord, is deprecated**: the docs landing page now opens with "We recommend you transition to Velocity." (<https://docs.papermc.io/waterfall/>).

So: **maintained, legacy, reach-only**. A 2026 scaffolding CLI should default to Velocity, and offer BungeeCord as an explicit secondary target with a notice.

---

## 4. One project/jar for both platforms?

### 4.1 What the loaders will and will not accept

The single most important fact: **Velocity rejects BungeeCord/Bukkit/Paper-only jars.** In `JavaPluginLoader#getSerializedPluginInfo`:

```java
case "velocity-plugin.json" -> { ...return Optional.of(GSON.fromJson(reader, SerializedPluginDescription.class)); }
case "paper-plugin.yml", "plugin.yml", "bungee.yml" -> foundBungeeBukkitPluginFile = true;
...
if (foundBungeeBukkitPluginFile) {
  throw new InvalidPluginException("The plugin file " + source.getFileName() + " appears to be a "
      + "Paper, Bukkit or BungeeCord plugin. Velocity does not support plugins from these "
      + "platforms.");
}
```

Source: <https://github.com/PaperMC/Velocity/blob/dev/3.0.0/proxy/src/main/java/com/velocitypowered/proxy/plugin/loader/java/JavaPluginLoader.java>. Note the branch: the throw is only reached when **no** `velocity-plugin.json` was found, so a jar that contains **both** `velocity-plugin.json` and `plugin.yml` is fine for Velocity. Separately, `loadCandidate` throws `InvalidPluginException("Did not find a valid velocity-plugin.json.")` when neither is present.

BungeeCord's own discovery is name-based on `bungee.yml`/`plugin.yml` only — it does not care about `velocity-plugin.json`, and it loads exactly one main class (`main` in the YAML).

### 4.2 The workable dual-platform layout

```
src/main/java/<pkg>/
  BungeeBootstrap.java      extends net.md_5.bungee.api.plugin.Plugin, onEnable()
  VelocityBootstrap.java    @Plugin(...), @Subscribe on ProxyInitializeEvent
  common/…                  platform-neutral logic, no proxy types in its signatures
src/main/resources/
  plugin.yml                main: <pkg>.BungeeBootstrap
                            # velocity-plugin.json is NOT hand-written — see §1.2/§1.3
```

Both processors/descriptors can coexist because they are read by different loaders from different keys. **Two main classes are required** — Velocity's `main` is the annotated class and BungeeCord's `main` is the YAML value; nothing in either API allows one class to be both `@Plugin`-annotated *and* extend BungeeCord's `Plugin` in a way that both loaders accept cleanly (the Bungee base constructor hard-asserts a `PluginClassloader`, and the Velocity side expects Guice-constructible types).

Shading concerns specific to this combination:

- Keep **both** APIs `compileOnly`; never bundle them.
- Velocity ships Guava, Gson, Netty, Adventure, SLF4J and Guice on its classpath; BungeeCord ships Guava, Gson, Netty and its own `bungeecord-chat`. Any third-party library must be **relocated**, per the Velocity docs' explicit caution (<https://docs.papermc.io/velocity/dev/dependency-management/>).
- Prefer the platform's own library story over shading where the platform has one: Paper/Velocity do not resolve `libraries`, but **BungeeCord does** (`libraries:` in `plugin.yml` → `LibraryLoader`), and Velocity offers `PluginManager#addToClasspath`.
- Shared logic must not reference either proxy API in its public surface, or one platform's absence of that class makes the other platform's class loading fragile under lazy linkage.
- Command names and permission nodes should be chosen once for the jar; both platforms treat duplicate command aliases as an error (Velocity `CommandManager#register` throws `IllegalArgumentException` if an alias is taken; BungeeCord overwrites the map entry silently).

### 4.3 What the common practice actually is

Primary sources support this read:

- **The proxy APIs are mutually exclusive by design.** Velocity docs: the API "does not support plugins made for BungeeCord/Waterfall" and only the unaffiliated `Snap` shim offers experimental compatibility.
- **No official dual-platform template or example exists.** PaperMC has no Velocity template repo; the BungeeCord repo has no plugin template; GitHub name search for Velocity plugin templates returns only third-party projects (chiefly Maven/Gradle starters), none of them PaperMC-owned.
- **The ecosystem norm is therefore: target one proxy, publish two artifacts**, or use a third-party multi-platform abstraction (e.g. an external cross-platform plugin framework) — not a hand-rolled dual-bootstrap jar. A dual-bootstrap jar is *technically* valid (per §4.1) and some plugins do it, but it doubles the artifact's untested surface.

**Recommendation for the CLI:** generate a single-platform project; if dual-platform is requested, generate **two entry-point classes in one Gradle module** (as in §4.2) with `compileOnly` APIs and a shared `common` package, and emit a warning that this mode is unsupported by both proxy teams.

---

## 5. Sample Velocity command and event listener

### 5.1 Commands

Three registrable `Command` sub-interfaces: `BrigadierCommand` (raw Brigadier node), `SimpleCommand` (Bukkit/Bungee-style `execute`/`suggest`/`hasPermission`), and `RawCommand`. All register through `CommandManager`. Source: <https://docs.papermc.io/velocity/dev/command-api/>.

**The registration API's current form is `register(CommandMeta, Command)`; the alias-only overloads are `@Deprecated`** — `CommandManager` javadoc says "use `register(CommandMeta, Command)` instead with a plugin specified". Sources: rendered docs (registration section) and `com.velocitypowered.api.command.CommandManager` in `velocity-api-4.2.0-sources.jar`.

Canonical scaffold code (docs' `HelloWorldPlugin`, trimmed):

```java
@Plugin(id = "helloworld", name = "Hello World", version = "1.0.0")
public final class HelloWorldPlugin {

  private final ProxyServer proxy;

  @Inject
  public HelloWorldPlugin(ProxyServer proxy) { this.proxy = proxy; }

  @Subscribe
  public void onProxyInitialize(ProxyInitializeEvent event) {
    CommandManager commandManager = proxy.getCommandManager();

    CommandMeta meta = commandManager.metaBuilder("test")
        .aliases("otherAlias", "anotherAlias")
        .plugin(this)            // docs: "the plugin to which it belongs (RECOMMENDED)"
        .build();

    commandManager.register(meta, new TestCommand());   // SimpleCommand, RawCommand, or BrigadierCommand
  }
}
```

and the `SimpleCommand` implementation shape (docs' `TestCommand`):

```java
public final class TestCommand implements SimpleCommand {
  @Override
  public void execute(final Invocation invocation) {
    CommandSource source = invocation.source();
    String[] args = invocation.arguments();
    source.sendMessage(Component.text("Hello World!", NamedTextColor.AQUA));
  }
}
```

A `BrigadierCommand` is built with `BrigadierCommand.literalArgumentBuilder("test").requires(src -> src.hasPermission("test.permission")).executes(ctx -> Command.SINGLE_SUCCESS).then(BrigadierCommand.requiredArgumentBuilder("argument", StringArgumentType.word()).suggests(...))` and registered the same way. The docs' `BrigadierCommand` example also uses `VelocityBrigadierMessage.tooltip(Component)` for suggestion tooltips and notes `BrigadierCommand.FORWARD` to forward a command to the backend server.

### 5.2 Events

- Annotate a method with `com.velocitypowered.api.event.Subscribe`. The docs carry a specific warning: "the import is `com.velocitypowered.api.event.Subscribe` and not in `com.google.common.eventbus`". **A template should emit that exact import; getting it wrong is a classic silent failure.**
- Priority: `@Subscribe(priority = 10)`; **higher runs first**; default **0**.
- Registration: the main class is registered automatically; other objects via `server.getEventManager().register(this, listener)` (both parameters `Object`), or a functional listener via `register(this, PlayerChatEvent.class, handler)`.
- Async handling (Velocity 3.0+): annotation-based listeners may take a `Continuation` second parameter or return an `EventTask`; functional listeners implement `AwaitingEventExecutor`. Source: <https://docs.papermc.io/velocity/dev/event-api/>.

```java
public final class MyListener {
  @Subscribe(priority = 10)
  public void onPlayerChat(PlayerChatEvent event) { /* ... */ }
}
```

**BungeeCord, for contrast** (same jar, different package): listeners implement `net.md_5.bungee.api.plugin.Listener` and handlers use `net.md_5.bungee.event.@EventHandler(priority = EventPriority.NORMAL)` with `byte` priorities `LOWEST=-64, LOW=-32, NORMAL=0, HIGH=32, HIGHEST=64`. Sources: `EventHandler.java` / `EventPriority.java` in `bungeecord-event-26.1-R0.1-…-sources.jar`.

---

## 6. What a template-rendering CLI must parameterize

| Parameter | Values / derivation | Source of truth | Notes for the CLI |
|---|---|---|---|
| `proxy` | `velocity` \| `bungeecord` \| (`both`) | ticket scope | `both` ⇒ two main classes + two descriptors (see §4) |
| `proxyVersion` | Velocity `4.2.0` (stable) / `4.2.1-SNAPSHOT`; BungeeCord `1.21-R0.4` / `26.1-R0.1-SNAPSHOT` | Fill API `/v3/projects/velocity/versions`; `bungeecord-api` maven-metadata | Snapshot coordinates for BungeeCord resolve to a timestamped build — never hardcode the timestamp |
| `apiCoordinates` | `com.velocitypowered:velocity-api:<v>`; `net.md-5:bungeecord-api:<v>` | maven-metadata | Velocity version == proxy version; BungeeCord artifact version == the proxy build string |
| **`javaVersion`** | **25** for Velocity; **21** if you deliberately pin the deprecated Velocity 3.5.x; **17** for current BungeeCord (25 required if the same jar also carries Velocity 4.x) | FAQ; Fill API `java.minimum`; BungeeCord `pom.xml` + bytecode major 61 | **The CLI must couple this to the API version.** Java 25 toolchain + `velocity-api` 3.5.x does not work: the 3.5.x API is Java 21-era and the docs' Java-25 guidance applies to 4.x |
| `gradleVersion` | `9.8.0` | `services.gradle.org/versions/current` | Write it into `gradle/wrapper/gradle-wrapper.properties`; expose an override. Do not copy Velocity's own pinned 9.3.0 |
| `velocityApiScope` | always `compileOnly` + `annotationProcessor` | docs project-setup | Never `implementation`; no shading of the API |
| `runVelocityPluginVersion` | `xyz.jpenilla.run-velocity` **3.1.0** | Gradle Plugin Portal metadata | Optional dev-run convenience |
| `pluginId` | must satisfy `[a-z][a-z0-9-_]{0,63}` | `SerializedPluginDescription` | Velocity only; BungeeCord derives identity from `name`. Validate and fail early, lowercase first |
| `pluginName` / `pluginVersion` | free strings | — | Velocity: display/semver. BungeeCord: `name` **must be unique among installed plugins** |
| `mainPackage` / `mainClass` | free | — | Velocity `main` is generated from the FQCN; BungeeCord `main` is written into `plugin.yml` |
| `bungeeDescriptorFileName` | `plugin.yml` (default) or `bungee.yml` | `PluginManager#detectPlugins` | BungeeCord prefers `bungee.yml` when both exist — pick exactly one to avoid surprises |
| `commandAlias` / `permissions` | free | — | Avoid clashing with built-ins on both platforms; Velocity requires `CommandMeta` + plugin reference |

**Anti-parameterization (things that must NOT be templated):** the `velocity-plugin.json` contents (generated), the `@Subscribe` import, and the id regex. Those are correctness, not configuration.

---

## Confidence / unverified

- **High confidence (read from published bytes or owning source):** Velocity annotation/descriptor schema and generation path; Velocity id pattern; Velocity jar-root descriptor lookup and the Bungee/Bukkit-jar rejection; version/Java matrices from Fill API and maven-metadata; Gradle 9.8.0; BungeeCord `PluginDescription` fields, discovery order, `Plugin` hooks, `plugin.yml`/`bungee.yml`, Java 17 target and the Java 8 target of `1.21-R0.4`.
- **Medium:** the exact `commands`/`permissions` support *inside* BungeeCord's `plugin.yml` — the POJO has no such fields, and `PluginManager` registers commands only from code, so a `commands:` block appears to be ignored (not read). Treated as "no".
- **Unverified / not sourced:** anything on `spigotmc.org` (resource pages, wiki, the BungeeCord "discussion thread") was unreachable from this network (Cloudflare), so no claim in this note depends on it. The BungeeCord Jenkins CI (`ci.md-5.net`) also returned a Cloudflare 301; maintenance activity is therefore evidenced by Maven publication timestamps and the repository README rather than CI history.
- **Line-number drift:** Velocity source links point at `dev/3.0.0`. The `velocity-api` 4.2.0 artifact's sources were read directly and match the branch for every file cited; the proxy loader (`JavaPluginLoader`) is not published as a sources artifact and was read from `dev/3.0.0` only.
- **Best available practice for cross-platform:** the "two bootstraps, one jar" pattern in §4.2 is derived from the loaders' actual behavior (read directly). Its popularity in the wild could not be measured from primary sources and is reported only as an inference from the absence of any official dual-platform template.

---

## Sources

Official docs
- Velocity — Creating your first plugin (repo, coordinates, Java 25, `compileOnly` + `annotationProcessor`, Maven/JDK-23 caution, MinecraftDev generator, Run-Task plugin): <https://docs.papermc.io/velocity/dev/creating-your-first-plugin/>
- Velocity — Plugin basics (`@Plugin` usage, two-phase loading, `ProxyInitializeEvent`, `@DataDirectory`): <https://docs.papermc.io/velocity/dev/api-basics/>
- Velocity — Dependency management (`@Dependency`, shading/relocation caution, `addToClasspath`): <https://docs.papermc.io/velocity/dev/dependency-management/>
- Velocity — Command API (`CommandManager#metaBuilder`, `register(CommandMeta, Command)`, `BrigadierCommand`, `SimpleCommand`): <https://docs.papermc.io/velocity/dev/command-api/>
- Velocity — Working with events (`@Subscribe`, priorities, registration, async): <https://docs.papermc.io/velocity/dev/event-api/>
- Velocity — FAQ ("Velocity 4.0.x and above requires at least Java 25"): <https://docs.papermc.io/velocity/faq/>
- Velocity — Getting started ("Velocity requires at least Java 25"): <https://docs.papermc.io/velocity/getting-started/>
- Velocity — Comparing with other proxies (recommendation, feature table, Snap note): <https://docs.papermc.io/velocity/comparisons-to-other-proxies/>
- Velocity — docs dev index (edit links proving the source paths): <https://docs.papermc.io/velocity/dev/getting-started/>
- Waterfall — landing page (transition-to-Velocity notice): <https://docs.papermc.io/waterfall/>

Velocity source (PaperMC/Velocity, branch `dev/3.0.0`)
- `api/.../plugin/Plugin.java` (`@Plugin` schema): <https://github.com/PaperMC/Velocity/blob/dev/3.0.0/api/src/main/java/com/velocitypowered/api/plugin/Plugin.java>
- `api/.../plugin/Dependency.java` (`@Dependency` schema): <https://github.com/PaperMC/Velocity/blob/dev/3.0.0/api/src/main/java/com/velocitypowered/api/plugin/Dependency.java>
- `api/.../plugin/PluginDescription.java` (`ID_PATTERN`): <https://github.com/PaperMC/Velocity/blob/dev/3.0.0/api/src/main/java/com/velocitypowered/api/plugin/PluginDescription.java>
- `api/.../plugin/ap/SerializedPluginDescription.java` (JSON schema, `ID_PATTERN_STRING`): <https://github.com/PaperMC/Velocity/blob/dev/3.0.0/api/src/main/java/com/velocitypowered/api/plugin/ap/SerializedPluginDescription.java>
- `api/.../plugin/ap/PluginAnnotationProcessor.java` (`createResource(CLASS_OUTPUT, "", "velocity-plugin.json")`): <https://github.com/PaperMC/Velocity/blob/dev/3.0.0/api/src/main/java/com/velocitypowered/api/plugin/ap/PluginAnnotationProcessor.java>
- `proxy/.../plugin/loader/java/JavaPluginLoader.java` (jar-root lookup + Bungee/Bukkit rejection + `InvalidPluginException`): <https://github.com/PaperMC/Velocity/blob/dev/3.0.0/proxy/src/main/java/com/velocitypowered/proxy/plugin/loader/java/JavaPluginLoader.java>
- `proxy/.../plugin/VelocityPluginManager.java` (candidate/load flow): <https://github.com/PaperMC/Velocity/blob/dev/3.0.0/proxy/src/main/java/com/velocitypowered/proxy/plugin/VelocityPluginManager.java>
- `build.gradle.kts` and `gradle/wrapper/gradle-wrapper.properties` (repo's own JDK 21 / Gradle 9.3.0 state): <https://github.com/PaperMC/Velocity/blob/dev/3.0.0/build.gradle.kts>, <https://github.com/PaperMC/Velocity/blob/dev/3.0.0/gradle/wrapper/gradle-wrapper.properties>
- retired `master` branch (1.x line, Gradle 5.4 wrapper): <https://github.com/PaperMC/Velocity/blob/master/gradle/wrapper/gradle-wrapper.properties>

BungeeCord source (SpigotMC/BungeeCord, `master`, cross-checked against published sources jars)
- `api/.../plugin/PluginDescription.java` (plugin.yml schema): <https://github.com/SpigotMC/BungeeCord/blob/master/api/src/main/java/net/md_5/bungee/api/plugin/PluginDescription.java>
- `api/.../plugin/PluginManager.java` (`bungee.yml`/`plugin.yml` discovery, `registerCommand`, `registerListener`, dependency graph): <https://github.com/SpigotMC/BungeeCord/blob/master/api/src/main/java/net/md_5/bungee/api/plugin/PluginManager.java>
- `api/.../plugin/Plugin.java` (`onLoad`/`onEnable`/`onDisable`, `PluginClassloader` state check): <https://github.com/SpigotMC/BungeeCord/blob/master/api/src/main/java/net/md_5/bungee/api/plugin/Plugin.java>
- `api/.../plugin/Command.java` (`Command`, `TabExecutor`): <https://github.com/SpigotMC/BungeeCord/blob/master/api/src/main/java/net/md_5/bungee/api/plugin/Command.java>
- `event/EventHandler.java` and `event/EventPriority.java`: <https://github.com/SpigotMC/BungeeCord/blob/master/event/src/main/java/net/md_5/bungee/event/EventHandler.java>
- root `pom.xml` (`maven.compiler.release` 17 on current line): <https://github.com/SpigotMC/BungeeCord/blob/master/pom.xml>
- `README.md` (SpigotMC maintenance, information thread, Jenkins binaries): <https://github.com/SpigotMC/BungeeCord/blob/master/README.md>

Artifact repositories and APIs
- `com.velocitypowered:velocity-api` maven-metadata (release 4.2.0, latest 4.2.1-SNAPSHOT): <https://repo.papermc.io/repository/maven-public/com/velocitypowered/velocity-api/maven-metadata.xml>
- `velocity-api-4.2.0.pom` (adventure-bom 5.2.0; gson 2.14.0; guava 33.7.1-jre; snakeyaml 2.7; slf4j 2.0.19; guice 7.0.0): <https://repo.papermc.io/repository/maven-public/com/velocitypowered/velocity-api/4.2.0/velocity-api-4.2.0.pom>
- `velocity-api-4.2.0.module` (`org.gradle.jvm.version: 25`): <https://repo.papermc.io/repository/maven-public/com/velocitypowered/velocity-api/4.2.0/velocity-api-4.2.0.module>
- `velocity-api-4.2.0.jar` / `-sources.jar` (`META-INF/services/javax.annotation.processing.Processor`, class-file major 69, all cited API sources): <https://repo.papermc.io/repository/maven-public/com/velocitypowered/velocity-api/4.2.0/velocity-api-4.2.0.jar>, <https://repo.papermc.io/repository/maven-public/com/velocitypowered/velocity-api/4.2.0/velocity-api-4.2.0-sources.jar>
- `net.md-5:bungeecord-api` maven-metadata (release 1.21-R0.4, latest 26.1-R0.1-SNAPSHOT): <https://repo.papermc.io/repository/maven-public/net/md-5/bungeecord-api/maven-metadata.xml>
- `bungeecord-api` 26.1 snapshot maven-metadata (build 43, 2026-09-15): <https://repo.papermc.io/repository/maven-public/net/md-5/bungeecord-api/26.1-R0.1-SNAPSHOT/maven-metadata.xml>
- `bungeecord-api-26.1-R0.1-20260915.210855-43.pom` (module dependencies): <https://repo.papermc.io/repository/maven-public/net/md-5/bungeecord-api/26.1-R0.1-SNAPSHOT/bungeecord-api-26.1-R0.1-20260915.210855-43.pom>
- `bungeecord-api-1.21-R0.4` jar + sources jar (frozen `PluginDescription`; Java 8 bytecode): <https://repo.papermc.io/repository/maven-public/net/md-5/bungeecord-api/1.21-R0.4/bungeecord-api-1.21-R0.4.jar>
- `bungeecord-event-26.1-R0.1-…-sources.jar` (`@EventHandler`, `EventPriority`): <https://repo.papermc.io/repository/maven-public/net/md-5/bungeecord-event/26.1-R0.1-SNAPSHOT/bungeecord-event-26.1-R0.1-20260915.210855-43-sources.jar>
- `bungeecord-parent` poms (`release` 8 on 1.21-R0.4): <https://repo.papermc.io/repository/maven-public/net/md-5/bungeecord-parent/1.21-R0.4/bungeecord-parent-1.21-R0.4.pom>
- PaperMC Fill API v3 — Velocity version/support/Java matrix: <https://fill.papermc.io/v3/projects/velocity/versions>
- PaperMC Fill API v3 — Velocity 4.2.0 stable build 30 (release commit, download): <https://fill.papermc.io/v3/projects/velocity/versions/4.2.0/builds/30>
- Gradle current version: <https://services.gradle.org/versions/current>
- Gradle Plugin Portal — `xyz.jpenilla.run-velocity` metadata (3.1.0): <https://plugins.gradle.org/m2/xyz/jpenilla/run-velocity/xyz.jpenilla.run-velocity.gradle.plugin/maven-metadata.xml>

Local verification
- `javac -cp velocity-api-3.5.1.jar -processorpath velocity-api-3.5.1.jar:guava-33.5.0-jre.jar:gson-2.14.0.jar` on a throwaway `@Plugin` class emitted `velocity-plugin.json` at the classes-output root with exactly the field set in §1.2 (run 2026-09-25; JDK 21 was used because the workspace has no JDK 25, so `velocity-api` 4.2.0 — Java 25 bytecode — could not be read by `javac` here; the 3.5.1 processor is the same class in an older build).

Related note in this directory
- `docs/research/paper-plugin-project-shape.md` — server-side (Paper/Bukkit) counterpart; shared Gradle/Java/Gradle-version conclusions are not duplicated here except where the proxy target changes them.
