package {{ packageName }};

import {{ packageName }}.config.PluginConfig;
import {{ packageName }}.message.Messages;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Map;

/**
{% if is_lang_en %}
 * Example business logic shared by every platform adapter.
 *
 * <p>This class only knows about the abstractions in {@code core}; it never touches a server API.
{% elif cap_dual_lang %}
 * 各平台适配层共享的示例业务逻辑。
 *
 * <p>本类只依赖 {@code core} 里的抽象，完全不接触任何服务端 API。
 * Example business logic shared by every platform adapter. This class only knows about the
 * abstractions in {@code core}; it never touches a server API.
{% else %}
 * 各平台适配层共享的示例业务逻辑。
 *
 * <p>本类只依赖 {@code core} 里的抽象，完全不接触任何服务端 API。
{% endif %}
 */
public final class ExampleService {

    private static final String KEY_WELCOME = "welcome";
    private static final String KEY_BROADCAST = "broadcast";
    private static final String KEY_USAGE = "command-usage";

    private final PluginConfig config;
    private final Messages messages;

    /**
{% if is_lang_en %}
     * @param config the loaded configuration
     * @param messages the loaded language pack
{% elif cap_dual_lang %}
     * @param config 已加载的配置
     * @param messages 已加载的语言包
     * @param config the loaded configuration
     * @param messages the loaded language pack
{% else %}
     * @param config 已加载的配置
     * @param messages 已加载的语言包
{% endif %}
     */
    public ExampleService(final PluginConfig config, final Messages messages) {
        this.config = config;
        this.messages = messages;
    }

    /**
{% if is_lang_en %}
     * Builds the join message for one player.
     *
     * @param playerName the player name substituted into {player}
     * @return the rendered, colour-coded message
{% elif cap_dual_lang %}
     * 生成某个玩家的加入消息。
     *
     * @param playerName 替换 {player} 的玩家名
     * @return 渲染后的带色消息
     * Builds the join message for one player.
     *
     * @param playerName the player name substituted into {player}
     * @return the rendered, colour-coded message
{% else %}
     * 生成某个玩家的加入消息。
     *
     * @param playerName 替换 {player} 的玩家名
     * @return 渲染后的带色消息
{% endif %}
     */
    public String welcomeMessage(final String playerName) {
        final Map<String, String> placeholders = new LinkedHashMap<String, String>();
        placeholders.put("message", config.welcomeMessage().replace("{player}", playerName));
        return messages.format(KEY_WELCOME, placeholders);
    }

    /**
{% if is_lang_en %}
     * Builds the broadcast line for one message.
     *
     * @param message the message body
     * @return the prefix plus the message body
{% elif cap_dual_lang %}
     * 生成一条广播消息。
     *
     * @param message 消息正文
     * @return 前缀加消息正文
     * Builds the broadcast line for one message.
     *
     * @param message the message body
     * @return the prefix plus the message body
{% else %}
     * 生成一条广播消息。
     *
     * @param message 消息正文
     * @return 前缀加消息正文
{% endif %}
     */
    public String broadcast(final String message) {
        final Map<String, String> placeholders = new LinkedHashMap<String, String>();
        placeholders.put("prefix", config.broadcastPrefix());
        placeholders.put("message", message);
        return messages.format(KEY_BROADCAST, placeholders);
    }

    /**
{% if is_lang_en %}
     * Builds the usage line for the label the sender typed.
     *
     * @param label the invoked command label or alias
     * @return the usage message
{% elif cap_dual_lang %}
     * 生成玩家实际输入别名对应的用法提示。
     *
     * @param label 实际调用的命令名或别名
     * @return 用法提示
     * Builds the usage line for the label the sender typed.
     *
     * @param label the invoked command label or alias
     * @return the usage message
{% else %}
     * 生成玩家实际输入别名对应的用法提示。
     *
     * @param label 实际调用的命令名或别名
     * @return 用法提示
{% endif %}
     */
    public String usage(final String label) {
        return messages.format(KEY_USAGE, Collections.singletonMap("label", label));
    }

    /**
{% if is_lang_en %}
     * @return whether the plugin should log verbosely
{% elif cap_dual_lang %}
     * @return 插件是否输出详细日志
     * @return whether the plugin should log verbosely
{% else %}
     * @return 插件是否输出详细日志
{% endif %}
     */
    public boolean debugEnabled() {
        return config.debug();
    }
}
