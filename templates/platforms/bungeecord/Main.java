package {{ packageName }}.{{ platform }};

import {{ packageName }}.ExampleService;
import {{ packageName }}.config.PluginConfig;
import {{ packageName }}.message.Messages;
import {{ packageName }}.{{ platform }}.command.ExampleCommand;
import {{ packageName }}.{{ platform }}.listener.ExampleListener;
import java.util.HashMap;
import java.util.Map;
import net.md_5.bungee.api.plugin.Plugin;

/**
{% if is_lang_en %}
 * Proxy entry point.
 *
 * <p>BungeeCord ignores a commands: section in plugin.yml, so the command is registered here.
{% elif cap_dual_lang %}
 * 代理端入口。
 *
 * <p>BungeeCord 会忽略 plugin.yml 里的 commands: 段，因此命令在这里注册。
 * Proxy entry point. BungeeCord ignores a commands: section in plugin.yml, so the command is
 * registered here.
{% else %}
 * 代理端入口。
 *
 * <p>BungeeCord 会忽略 plugin.yml 里的 commands: 段，因此命令在这里注册。
{% endif %}
 */
public final class {{ pluginName }} extends Plugin {

    @Override
    public void onEnable() {
        // A proxy has no bundled configuration here yet: keep the defaults in code until the
        // plugin writes its own config.yml into the data folder.
        final Map<String, Object> config = new HashMap<String, Object>();
        config.put("welcome-message", "&aWelcome, {player}!");
        config.put("broadcast-prefix", "[{{ pluginName }}]");
        config.put("debug", Boolean.FALSE);

        final Map<String, String> templates = new HashMap<String, String>();
        templates.put("welcome", "{message}");
        templates.put("broadcast", "{prefix} {message}");
        templates.put("command-usage", "&eUsage: /{label} <message>");

        final ExampleService service = new ExampleService(PluginConfig.from(config), Messages.from(templates));
        getProxy().getPluginManager().registerCommand(this, new ExampleCommand(service));
        getProxy().getPluginManager().registerListener(this, new ExampleListener(service));
        getLogger().info("{{ pluginName }} enabled for Minecraft {{ mcVersion }}");
    }
}
