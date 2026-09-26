package {{ packageName }}.{{ platform }};

import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.Scanner;
import org.junit.jupiter.api.Test;

/**
{% if is_lang_en %}
 * Integration test: asserts that the packaged descriptor really is on the runtime classpath and
 * still points at this module's entry class. It is not part of `check`; run
 * {@code ./gradlew integrationTest} (or a CI step) explicitly.
{% elif cap_dual_lang %}
 * 集成测试：断言打包后的描述符真的在运行时 classpath 上、且仍指向本模块的入口类。
 * 它不属于 `check`；请显式跑 {@code ./gradlew integrationTest}（或加一个 CI 步骤）。
 * Integration test: asserts that the packaged descriptor really is on the runtime classpath and
 * still points at this module's entry class. It is not part of `check`.
{% else %}
 * 集成测试：断言打包后的描述符真的在运行时 classpath 上、且仍指向本模块的入口类。
 * 它不属于 `check`；请显式跑 {@code ./gradlew integrationTest}（或加一个 CI 步骤）。
{% endif %}
 */
class {{ pluginName }}IntegrationTest {

    private static final String DESCRIPTOR =
            "{% if is_paper_metadata %}paper-plugin.yml{% else %}plugin.yml{% endif %}";

    private static String descriptor() throws IOException {
        // Keep the interpolated class name on its own line: the project name is
        // user-supplied, so folding it into a longer expression would let the
        // generated line overflow the checkstyle LineLength limit.
        final ClassLoader loader = {{ pluginName }}IntegrationTest.class.getClassLoader();
        try (InputStream in = loader.getResourceAsStream(DESCRIPTOR)) {
            assertNotNull(in, DESCRIPTOR + " must be packaged on the classpath");
            final Scanner scanner = new Scanner(in, StandardCharsets.UTF_8.name());
            scanner.useDelimiter("\\A");
            return scanner.hasNext() ? scanner.next() : "";
        }
    }

    @Test
    void descriptorNamesThisPluginAndItsEntryClass() throws IOException {
        final String text = descriptor();
        assertTrue(text.contains("name: {{ pluginName }}"), "descriptor must name the plugin");
        assertTrue(
                text.contains("main: {{ mainClass }}"),
                "descriptor must point at the entry class");
    }

    @Test
    void descriptorCarriesTheBuildTimeVersion() throws IOException {
        assertTrue(descriptor().contains("version: \""), "version must stay a quoted scalar");
    }
}
