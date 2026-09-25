package {{ packageName }};

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import {{ packageName }}.config.PluginConfig;
import {{ packageName }}.message.Messages;
import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.Test;

/**
{% if is_lang_en %}
 * Unit tests for the shared logic. No server API is involved, so this test runs everywhere.
{% elif cap_dual_lang %}
 * 共享逻辑的单元测试。不涉及任何服务端 API，因此在任何环境都能跑。
 * Unit tests for the shared logic. No server API is involved, so this test runs everywhere.
{% else %}
 * 共享逻辑的单元测试。不涉及任何服务端 API，因此在任何环境都能跑。
{% endif %}
 */
class ExampleServiceTest {

    private static ExampleService newService() {
        final Map<String, Object> config = new HashMap<String, Object>();
        config.put("welcome-message", "&aHello, {player}!");
        config.put("broadcast-prefix", "[Test]");
        config.put("debug", Boolean.TRUE);

        final Map<String, String> messages = new HashMap<String, String>();
        messages.put("welcome", "{message}");
        messages.put("broadcast", "{prefix} {message}");
        messages.put("command-usage", "Usage: /{label} <message>");

        return new ExampleService(PluginConfig.from(config), Messages.from(messages));
    }

    @Test
    void welcomeMessageSubstitutesThePlayerName() {
        assertEquals("&aHello, Alex!", newService().welcomeMessage("Alex"));
    }

    @Test
    void broadcastAppliesTheConfiguredPrefix() {
        assertEquals("[Test] hello everyone", newService().broadcast("hello everyone"));
    }

    @Test
    void usageUsesTheInvokedLabel() {
        assertEquals("Usage: /example <message>", newService().usage("example"));
    }

    @Test
    void debugFlagComesFromTheConfiguration() {
        assertTrue(newService().debugEnabled());
    }
}
