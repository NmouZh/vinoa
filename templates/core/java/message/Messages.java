package {{ packageName }}.message;

import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Map;

/**
{% if is_lang_en %}
 * Localised message lookup with {placeholder} substitution.
 *
 * <p>Missing keys are a hard error: a plugin that silently prints an empty line hides its own
 * packaging mistake.
{% elif cap_dual_lang %}
 * 带 {placeholder} 替换的本地化消息取值器。
 *
 * <p>缺 key 是硬错误：静默输出空串会把打包错误藏起来。
 * Localised message lookup with {placeholder} substitution. Missing keys are a hard error:
 * a plugin that silently prints an empty line hides its own packaging mistake.
{% else %}
 * 带 {placeholder} 替换的本地化消息取值器。
 *
 * <p>缺 key 是硬错误：静默输出空串会把打包错误藏起来。
{% endif %}
 */
public final class Messages {

    private final Map<String, String> templates;

    private Messages(final Map<String, String> templates) {
        this.templates = Collections.unmodifiableMap(new LinkedHashMap<String, String>(templates));
    }

    /**
{% if is_lang_en %}
     * @param templates message key to message template
     * @return a lookup backed by the given templates
{% elif cap_dual_lang %}
     * @param templates 消息 key 到消息模板的映射
     * @return 以该映射为后端的取值器
     * @param templates message key to message template
     * @return a lookup backed by the given templates
{% else %}
     * @param templates 消息 key 到消息模板的映射
     * @return 以该映射为后端的取值器
{% endif %}
     */
    public static Messages from(final Map<String, String> templates) {
        return new Messages(templates);
    }

    /**
{% if is_lang_en %}
     * @param key the message key
     * @return the raw template
     * @throws IllegalStateException when the key is missing
{% elif cap_dual_lang %}
     * @param key 消息 key
     * @return 原始模板
     * @throws IllegalStateException key 缺失时抛出
     * @param key the message key
     * @return the raw template
     * @throws IllegalStateException when the key is missing
{% else %}
     * @param key 消息 key
     * @return 原始模板
     * @throws IllegalStateException key 缺失时抛出
{% endif %}
     */
    public String raw(final String key) {
        final String value = templates.get(key);
        if (value == null) {
            throw new IllegalStateException("Missing message key: " + key);
        }
        return value;
    }

    /**
{% if is_lang_en %}
     * Replaces every {name} placeholder found in the template.
     *
     * @param key the message key
     * @param placeholders placeholder name to replacement text
     * @return the rendered message
{% elif cap_dual_lang %}
     * 替换模板中的每个 {name} 占位符。
     *
     * @param key 消息 key
     * @param placeholders 占位符名到替换文本
     * @return 渲染后的消息
     * Replaces every {name} placeholder found in the template.
     *
     * @param key the message key
     * @param placeholders placeholder name to replacement text
     * @return the rendered message
{% else %}
     * 替换模板中的每个 {name} 占位符。
     *
     * @param key 消息 key
     * @param placeholders 占位符名到替换文本
     * @return 渲染后的消息
{% endif %}
     */
    public String format(final String key, final Map<String, String> placeholders) {
        String result = raw(key);
        for (final Map.Entry<String, String> entry : placeholders.entrySet()) {
            result = result.replace("{" + entry.getKey() + "}", entry.getValue());
        }
        return result;
    }
}
