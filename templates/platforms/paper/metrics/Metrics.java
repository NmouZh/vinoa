package {{ packageName }}.{{ platform }}.metrics;

import org.bukkit.plugin.java.JavaPlugin;

/**
{% if is_lang_en %}
 * bStats wiring for this plugin.
 *
 * <p>{@link #BSTATS_PLUGIN_ID} is injected at generate time from {@code --bstats-id}; it is never
 * guessed. bStats is not a server plugin, so its classes have to reach the runtime classpath:
 * either through the {@code libraries:} metadata field (Minecraft 1.16.5 and newer) or by
 * shading. On older targets, add the shading step before wiring this class up.
{% elif cap_dual_lang %}
 * 本插件的 bStats 接线。
 *
 * <p>{@link #BSTATS_PLUGIN_ID} 由 {@code --bstats-id} 在生成期注入，绝不猜测。bStats 不是服务端
 * 插件，它的类必须进运行时 classpath：要么走 {@code libraries:} 元数据字段（Minecraft 1.16.5
 * 及以后），要么自己 shade。在更老的目标上请先补 shade 步骤再接线本类。
 * bStats wiring for this plugin. BSTATS_PLUGIN_ID is injected at generate time from --bstats-id;
 * it is never guessed. bStats is not a server plugin, so its classes have to reach the runtime
 * classpath: either through the libraries: metadata field (1.16.5 and newer) or by shading.
{% else %}
 * 本插件的 bStats 接线。
 *
 * <p>{@link #BSTATS_PLUGIN_ID} 由 {@code --bstats-id} 在生成期注入，绝不猜测。bStats 不是服务端
 * 插件，它的类必须进运行时 classpath：要么走 {@code libraries:} 元数据字段（Minecraft 1.16.5
 * 及以后），要么自己 shade。在更老的目标上请先补 shade 步骤再接线本类。
{% endif %}
 */
public final class Metrics {

    /** The numeric plugin id bstats.org shows for this plugin. */
    public static final int BSTATS_PLUGIN_ID = {{ bstatsPluginId }};

    private final org.bstats.bukkit.Metrics delegate;

    /**
{% if is_lang_en %}
     * Starts submitting anonymous usage statistics.
     *
     * @param plugin the plugin instance bStats reports for
{% elif cap_dual_lang %}
     * 开始上报匿名使用统计。
     *
     * @param plugin bStats 上报所用的插件实例
     * Starts submitting anonymous usage statistics.
     *
     * @param plugin the plugin instance bStats reports for
{% else %}
     * 开始上报匿名使用统计。
     *
     * @param plugin bStats 上报所用的插件实例
{% endif %}
     */
    public Metrics(final JavaPlugin plugin) {
        this.delegate = new org.bstats.bukkit.Metrics(plugin, BSTATS_PLUGIN_ID);
    }

    /**
{% if is_lang_en %}
     * @return the underlying bStats instance, so charts can be added
{% elif cap_dual_lang %}
     * @return 底层的 bStats 实例，便于继续添加图表
     * @return the underlying bStats instance, so charts can be added
{% else %}
     * @return 底层的 bStats 实例，便于继续添加图表
{% endif %}
     */
    public org.bstats.bukkit.Metrics delegate() {
        return delegate;
    }
}
