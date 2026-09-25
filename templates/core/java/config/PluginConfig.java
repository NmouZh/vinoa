package {{ packageName }}.config;

import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Map;

/**
{% if is_lang_en %}
 * Read-only view of the plugin configuration.
 *
 * <p>Each platform adapter turns its own configuration source into this interface, so that the
 * shared logic never depends on a server API.
{% elif cap_dual_lang %}
 * 插件配置的只读视图。
 *
 * <p>各平台适配层把自己的配置源转换成这个接口，共享逻辑因此不依赖任何服务端 API。
 * Read-only view of the plugin configuration. Each platform adapter turns its own
 * configuration source into this interface, so that the shared logic never depends on a
 * server API.
{% else %}
 * 插件配置的只读视图。
 *
 * <p>各平台适配层把自己的配置源转换成这个接口，共享逻辑因此不依赖任何服务端 API。
{% endif %}
 */
public interface PluginConfig {

    /**
{% if is_lang_en %}
     * @return the message shown to a joining player; {player} is replaced by the player name
{% elif cap_dual_lang %}
     * @return 玩家加入时显示的消息；{player} 会替换成玩家名
     * @return the message shown to a joining player; {player} is replaced by the player name
{% else %}
     * @return 玩家加入时显示的消息；{player} 会替换成玩家名
{% endif %}
     */
    String welcomeMessage();

    /**
{% if is_lang_en %}
     * @return the prefix prepended to broadcast messages
{% elif cap_dual_lang %}
     * @return 广播消息使用的前缀
     * @return the prefix prepended to broadcast messages
{% else %}
     * @return 广播消息使用的前缀
{% endif %}
     */
    String broadcastPrefix();

    /**
{% if is_lang_en %}
     * @return whether verbose logging is enabled
{% elif cap_dual_lang %}
     * @return 是否打开详细日志
     * @return whether verbose logging is enabled
{% else %}
     * @return 是否打开详细日志
{% endif %}
     */
    boolean debug();

    /**
{% if is_lang_en %}
     * Adapts a flattened configuration map (for example a YAML loader result) to this interface.
     *
     * @param values flattened keys such as {@code welcome-message}
     * @return a configuration view backed by the given map
{% elif cap_dual_lang %}
     * 把一个扁平化的配置表（例如 YAML 加载结果）适配成本接口。
     *
     * @param values 扁平键，例如 {@code welcome-message}
     * @return 以该表为后端的配置视图
     * Adapts a flattened configuration map (for example a YAML loader result) to this interface.
     *
     * @param values flattened keys such as {@code welcome-message}
     * @return a configuration view backed by the given map
{% else %}
     * 把一个扁平化的配置表（例如 YAML 加载结果）适配成本接口。
     *
     * @param values 扁平键，例如 {@code welcome-message}
     * @return 以该表为后端的配置视图
{% endif %}
     */
    static PluginConfig from(final Map<String, Object> values) {
        final Map<String, Object> copy =
                Collections.unmodifiableMap(new LinkedHashMap<String, Object>(values));
        return new PluginConfig() {
            @Override
            public String welcomeMessage() {
                return asString(copy.get("welcome-message"), "&aWelcome, {player}!");
            }

            @Override
            public String broadcastPrefix() {
                return asString(copy.get("broadcast-prefix"), "[{{ pluginName }}]");
            }

            @Override
            public boolean debug() {
                final Object raw = copy.get("debug");
                if (raw instanceof Boolean) {
                    return ((Boolean) raw).booleanValue();
                }
                return Boolean.parseBoolean(String.valueOf(raw));
            }
        };
    }

    /**
{% if is_lang_en %}
     * Converts a raw configuration value to text.
     *
     * @param value the raw value, possibly {@code null}
     * @param fallback the value returned when {@code value} is {@code null}
     * @return the text form of {@code value}, or {@code fallback}
{% elif cap_dual_lang %}
     * 把原始配置值转成文本。
     *
     * @param value 原始值，可能为 {@code null}
     * @param fallback {@code value} 为 {@code null} 时返回的默认值
     * @return {@code value} 的文本形式，或 {@code fallback}
     * Converts a raw configuration value to text.
     *
     * @param value the raw value, possibly {@code null}
     * @param fallback the value returned when {@code value} is {@code null}
     * @return the text form of {@code value}, or {@code fallback}
{% else %}
     * 把原始配置值转成文本。
     *
     * @param value 原始值，可能为 {@code null}
     * @param fallback {@code value} 为 {@code null} 时返回的默认值
     * @return {@code value} 的文本形式，或 {@code fallback}
{% endif %}
     */
    static String asString(final Object value, final String fallback) {
        return value == null ? fallback : String.valueOf(value);
    }
}
