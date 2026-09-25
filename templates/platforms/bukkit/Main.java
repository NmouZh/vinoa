package {{ packageName }}.{{ platform }};

import {{ packageName }}.ExampleService;
import {{ packageName }}.config.PluginConfig;
import {{ packageName }}.message.Messages;
import {{ packageName }}.{{ platform }}.command.ExampleCommand;
import {{ packageName }}.{{ platform }}.listener.ExampleListener;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.Map;
import org.bukkit.command.PluginCommand;
import org.bukkit.configuration.file.YamlConfiguration;
import org.bukkit.plugin.java.JavaPlugin;

/**
{% if is_lang_en %}
 * Plugin entry point.
 *
 * <p>The command is declared in plugin.yml, so registration here is only
 * {@code getCommand(...).setExecutor(...)}: declaring it in the metadata and registering a
 * second handler elsewhere would double-register it.
{% elif cap_dual_lang %}
 * 插件入口。
 *
 * <p>命令已在 plugin.yml 里声明，因此这里只做 {@code getCommand(...).setExecutor(...)}：
 * 元数据声明之外再注册第二个处理器会导致重复注册。
 * Plugin entry point. The command is declared in plugin.yml, so registration here is only
 * getCommand(...).setExecutor(...): declaring it in the metadata and registering a second
 * handler elsewhere would double-register it.
{% else %}
 * 插件入口。
 *
 * <p>命令已在 plugin.yml 里声明，因此这里只做 {@code getCommand(...).setExecutor(...)}：
 * 元数据声明之外再注册第二个处理器会导致重复注册。
{% endif %}
 */
public final class {{ pluginName }} extends JavaPlugin {

    private ExampleService service;

    @Override
    public void onEnable() {
        saveDefaultConfig();
        this.service = new ExampleService(PluginConfig.from(getConfig().getValues(true)), loadMessages());

        final PluginCommand command = getCommand("{{ commandName }}");
        if (command == null) {
            getLogger().warning("plugin.yml does not declare the command {{ commandName }}");
        } else {
            final ExampleCommand executor = new ExampleCommand(service);
            command.setExecutor(executor);
            command.setTabCompleter(executor);
        }
        getServer().getPluginManager().registerEvents(new ExampleListener(service), this);
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
