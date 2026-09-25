package {{ packageName }}.{{ platform }}.hook;

import {{ packageName }}.ExampleService;
import me.clip.placeholderapi.expansion.PlaceholderExpansion;
import org.bukkit.OfflinePlayer;
import org.bukkit.plugin.Plugin;

/**
{% if is_lang_en %}
 * Example PlaceholderAPI expansion exposing one placeholder.
 *
 * <p>PlaceholderAPI is a soft dependency: the plugin only registers this expansion after a
 * runtime probe confirms the API is present, so a server without PlaceholderAPI still loads.
 * The generated placeholder is {{ pluginId }}_welcome, wrapped in percent signs.
{% elif cap_dual_lang %}
 * 示例 PlaceholderAPI 扩展，暴露一个占位符。
 *
 * <p>PlaceholderAPI 是软依赖：只有在运行时探测确认 API 存在后才注册本扩展，因此没装
 * PlaceholderAPI 的服务端照样能加载。生成的占位符是 {{ pluginId }}_welcome，两侧用百分号包裹。
 * Example PlaceholderAPI expansion exposing one placeholder. PlaceholderAPI is a soft
 * dependency: the expansion is only registered after a runtime probe confirms the API exists.
{% else %}
 * 示例 PlaceholderAPI 扩展，暴露一个占位符。
 *
 * <p>PlaceholderAPI 是软依赖：只有在运行时探测确认 API 存在后才注册本扩展，因此没装
 * PlaceholderAPI 的服务端照样能加载。生成的占位符是 {{ pluginId }}_welcome，两侧用百分号包裹。
{% endif %}
 */
public final class PlaceholderHook extends PlaceholderExpansion {

    private final Plugin plugin;
    private final ExampleService service;

    /**
{% if is_lang_en %}
     * @param plugin the owning plugin
     * @param service the shared service that renders the placeholder value
{% elif cap_dual_lang %}
     * @param plugin 所属插件
     * @param service 渲染占位符取值的共享服务
     * @param plugin the owning plugin
     * @param service the shared service that renders the placeholder value
{% else %}
     * @param plugin 所属插件
     * @param service 渲染占位符取值的共享服务
{% endif %}
     */
    public PlaceholderHook(final Plugin plugin, final ExampleService service) {
        this.plugin = plugin;
        this.service = service;
    }

    @Override
    public String getIdentifier() {
        return "{{ pluginId }}";
    }

    @Override
    public String getAuthor() {
        return "{% if has_authors %}{{ authors[0] }}{% else %}{{ pluginName }}{% endif %}";
    }

    @Override
    public String getVersion() {
        return "{{ projectVersion }}";
    }

    @Override
    public boolean persist() {
        return true;
    }

    @Override
    public String onRequest(final OfflinePlayer player, final String params) {
        if ("welcome".equals(params)) {
            return player == null ? "" : service.welcomeMessage(player.getName());
        }
        return null;
    }

    /**
{% if is_lang_en %}
     * @return the plugin this expansion belongs to
{% elif cap_dual_lang %}
     * @return 本扩展所属的插件
     * @return the plugin this expansion belongs to
{% else %}
     * @return 本扩展所属的插件
{% endif %}
     */
    public Plugin plugin() {
        return plugin;
    }
}
