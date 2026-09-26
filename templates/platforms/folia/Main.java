package {{ packageName }}.{{ platform }};

import {{ packageName }}.ExampleService;
import {{ packageName }}.config.PluginConfig;
import {{ packageName }}.message.Messages;
import {{ packageName }}.{{ platform }}.command.ExampleCommand;
{% if cap_placeholderapi %}
import {{ packageName }}.{{ platform }}.hook.PlaceholderHook;
{% endif %}
{% if cap_bstats and cap_libraries %}
import {{ packageName }}.{{ platform }}.metrics.Metrics;
{% endif %}
{% if cap_update_check %}
import {{ packageName }}.update.UpdateChecker;
{% endif %}
import {{ packageName }}.{{ platform }}.listener.ExampleListener;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.Map;
{% if is_paper_metadata %}
import io.papermc.paper.plugin.lifecycle.event.types.LifecycleEvents;
{% endif %}
import org.bukkit.configuration.file.YamlConfiguration;
import org.bukkit.plugin.java.JavaPlugin;

/**
{% if is_lang_en %}
 * Plugin entry point.
 *
 * <p>Only bootstrap work happens here: load the configuration, build the shared service, then
 * register commands and listeners. Registration must match the metadata format chosen at
 * generate time, otherwise commands are either invisible or registered twice.
{% elif cap_dual_lang %}
 * 插件入口。
 *
 * <p>这里只做 bootstrap：读配置、构建共享服务、注册命令与监听器。注册方式必须与生成时
 * 选定的元数据格式配对，否则命令会不可见或被注册两次。
 * Plugin entry point. Only bootstrap work happens here: load the configuration, build the
 * shared service, then register commands and listeners. Registration must match the metadata
 * format chosen at generate time.
{% else %}
 * 插件入口。
 *
 * <p>这里只做 bootstrap：读配置、构建共享服务、注册命令与监听器。注册方式必须与生成时
 * 选定的元数据格式配对，否则命令会不可见或被注册两次。
{% endif %}
 */
public final class {{ pluginName }} extends JavaPlugin {

    // Folia note: the global scheduler is unavailable here. Anything that touches the world or
    // an entity must go through Bukkit.getRegionScheduler() or entity.getScheduler(); only
    // runTaskAsynchronously() may be used as a global task. The examples below need no
    // scheduling, so they run unchanged on Folia.
    private ExampleService service;

    @Override
    public void onEnable() {
        saveDefaultConfig();
        this.service = new ExampleService(PluginConfig.from(getConfig().getValues(true)), loadMessages());
        getServer().getPluginManager().registerEvents(new ExampleListener(service), this);
{% if cap_bstats and cap_libraries %}
        // bStats is not a server plugin: its classes arrive through the libraries: metadata field.
        Metrics.start(this);
{% endif %}
{% if cap_placeholderapi %}
        // Soft dependency: only register the expansion when the API is really on the classpath.
        try {
            Class.forName("me.clip.placeholderapi.PlaceholderAPI");
            new PlaceholderHook(service).register();
        } catch (ClassNotFoundException absent) {
            getLogger().info("PlaceholderAPI is not installed; the expansion stays unregistered.");
        }
{% endif %}
{% if is_paper_metadata %}
        // Paper plugin format: commands are NOT declared in paper-plugin.yml. Registering them
        // through the lifecycle API is the only path, and using plugin.yml alongside it would
        // double-register.
        getLifecycleManager().registerEventHandler(LifecycleEvents.COMMANDS, event ->
                event.registrar().register("{{ commandName }}", new ExampleCommand(service)));
{% elif cap_paper_brigadier %}
        // plugin.yml carries the commands: section; the Brigadier API executes them.
        registerCommand("{{ commandName }}", new ExampleCommand(service));
{% else %}
        // plugin.yml carries the commands: section; this Paper line predates the Brigadier API,
        // so the Bukkit executor path is used.
        final org.bukkit.command.PluginCommand command = getCommand("{{ commandName }}");
        if (command != null) {
            final ExampleCommand executor = new ExampleCommand(service);
            command.setExecutor(executor);
            command.setTabCompleter(executor);
        }
{% endif %}
    }

    private Messages loadMessages() {
        final Map<String, String> templates = new HashMap<String, String>();
{% if is_lang_en %}
        final String locale = "en";
{% else %}
        final String locale = "zh_CN";
{% endif %}
        final InputStream stream = getResource("lang/" + locale + ".yml");
        if (stream == null) {
            return Messages.from(templates);
        }
        final YamlConfiguration yaml =
                YamlConfiguration.loadConfiguration(new InputStreamReader(stream, StandardCharsets.UTF_8));
        for (final String key : yaml.getKeys(false)) {
            templates.put(key, yaml.getString(key, ""));
        }
        return Messages.from(templates);
    }
}
