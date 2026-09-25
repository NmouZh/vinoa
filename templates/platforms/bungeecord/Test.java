package {{ packageName }}.{{ platform }};

import static org.junit.jupiter.api.Assertions.assertEquals;

import {{ packageName }}.ExampleService;
import {{ packageName }}.config.PluginConfig;
import {{ packageName }}.message.Messages;
import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.Test;

/**
{% if is_lang_en %}
 * Smoke test: proves that this platform module can reach the shared logic.
{% elif cap_dual_lang %}
 * 冒烟测试：证明本平台模块能正常依赖共享逻辑。
 * Smoke test: proves that this platform module can reach the shared logic.
{% else %}
 * 冒烟测试：证明本平台模块能正常依赖共享逻辑。
{% endif %}
 */
class {{ pluginName }}Test {

    @Test
    void sharedServiceIsReachableFromThisModule() {
        final Map<String, Object> config = new HashMap<String, Object>();
        config.put("welcome-message", "hi {player}");
        final Map<String, String> messages = new HashMap<String, String>();
        messages.put("welcome", "{message}");
        final ExampleService service = new ExampleService(PluginConfig.from(config), Messages.from(messages));
        assertEquals("hi Sam", service.welcomeMessage("Sam"));
    }
}
