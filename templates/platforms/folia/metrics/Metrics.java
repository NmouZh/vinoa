package {{ packageName }}.{{ platform }}.metrics;

import org.bukkit.plugin.java.JavaPlugin;

/**
{% if is_lang_en %}
 * bStats bootstrap for this plugin.
 *
 * <p>{@link #BSTATS_PLUGIN_ID} is injected at generate time from {@code --bstats-id}; it is never
 * guessed. bStats is not a server plugin, so its classes have to reach the runtime classpath:
 * through the {@code libraries:} metadata field (Minecraft 1.16.5 and newer) or by shading. On
 * older targets, add the shading step before calling {@link #start(JavaPlugin)}.
{% elif cap_dual_lang %}
 * 本插件的 bStats 引导。
 *
 * <p>{@link #BSTATS_PLUGIN_ID} 由 {@code --bstats-id} 在生成期注入，绝不猜测。bStats 不是服务端
 * 插件，它的类必须进运行时 classpath：走 {@code libraries:} 元数据字段（Minecraft 1.16.5 及以后）
 * 或自己 shade。在更老的目标上请先补 shade 步骤再调用 {@link #start(JavaPlugin)}。
 * bStats bootstrap for this plugin. BSTATS_PLUGIN_ID is injected at generate time from
 * --bstats-id; it is never guessed. bStats is not a server plugin, so its classes have to reach
 * the runtime classpath through the libraries: metadata field (1.16.5+) or by shading.
{% else %}
 * 本插件的 bStats 引导。
 *
 * <p>{@link #BSTATS_PLUGIN_ID} 由 {@code --bstats-id} 在生成期注入，绝不猜测。bStats 不是服务端
 * 插件，它的类必须进运行时 classpath：走 {@code libraries:} 元数据字段（Minecraft 1.16.5 及以后）
 * 或自己 shade。在更老的目标上请先补 shade 步骤再调用 {@link #start(JavaPlugin)}。
{% endif %}
 */
public final class Metrics {

    /** The numeric plugin id bstats.org shows for this plugin. */
    public static final int BSTATS_PLUGIN_ID = {{ bstatsPluginId }};

    private Metrics() {
    }

    /**
{% if is_lang_en %}
     * Starts submitting anonymous usage statistics.
     *
     * @param plugin the plugin instance bStats reports for
     * @return the bStats instance, so charts can be added
{% elif cap_dual_lang %}
     * 开始上报匿名使用统计。
     *
     * @param plugin bStats 上报所用的插件实例
     * @return bStats 实例，便于继续添加图表
     * Starts submitting anonymous usage statistics.
     *
     * @param plugin the plugin instance bStats reports for
     * @return the bStats instance, so charts can be added
{% else %}
     * 开始上报匿名使用统计。
     *
     * @param plugin bStats 上报所用的插件实例
     * @return bStats 实例，便于继续添加图表
{% endif %}
     */
    public static org.bstats.bukkit.Metrics start(final JavaPlugin plugin) {
        return new org.bstats.bukkit.Metrics(plugin, BSTATS_PLUGIN_ID);
    }
}
