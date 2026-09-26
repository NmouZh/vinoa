package {{ packageName }}.{{ platform }};

import com.google.inject.Inject;
import com.velocitypowered.api.command.CommandManager;
import com.velocitypowered.api.event.Subscribe;
import com.velocitypowered.api.event.proxy.ProxyInitializeEvent;
import com.velocitypowered.api.plugin.Plugin;
import com.velocitypowered.api.proxy.ProxyServer;
import {{ packageName }}.ExampleService;
import {{ packageName }}.config.PluginConfig;
import {{ packageName }}.message.Messages;
import {{ packageName }}.{{ platform }}.command.ExampleCommand;
import {{ packageName }}.{{ platform }}.listener.ExampleListener;
import java.util.HashMap;
import java.util.Map;
import org.slf4j.Logger;

/**
{% if is_lang_en %}
 * Proxy entry point.
 *
 * <p>Velocity builds the plugin descriptor from this annotation (there is no resources/
 * directory). Nothing may be registered in the constructor: the proxy is only ready once
 * ProxyInitializeEvent fires.
{% elif cap_dual_lang %}
 * 代理端入口。
 *
 * <p>Velocity 用这个注解生成插件描述符（本模块没有 resources/ 目录）。构造器里不能注册
 * 任何东西：代理端要等 ProxyInitializeEvent 才就绪。
 * Proxy entry point. Velocity builds the plugin descriptor from this annotation (there is no
 * resources/ directory). Nothing may be registered in the constructor: the proxy is only ready
 * once ProxyInitializeEvent fires.
{% else %}
 * 代理端入口。
 *
 * <p>Velocity 用这个注解生成插件描述符（本模块没有 resources/ 目录）。构造器里不能注册
 * 任何东西：代理端要等 ProxyInitializeEvent 才就绪。
{% endif %}
 */
@Plugin(
        id = "{{ pluginId }}",
        name = "{{ pluginName }}",
        version = "{{ projectVersion }}",
        description = "{{ pluginDescription }}"{% if has_authors %},
        authors = { {% for a in authors %}"{{ a }}", {% endfor %}}{% endif %})
public final class {{ pluginName }} {

    private final ProxyServer proxy;
    private final Logger logger;
    private final CommandManager commandManager;

    @Inject
    public {{ pluginName }}(
            final ProxyServer proxy, final Logger logger, final CommandManager commandManager) {
        this.proxy = proxy;
        this.logger = logger;
        this.commandManager = commandManager;
    }

    /**
{% if is_lang_en %}
     * Registers the example command and the example listener.
     *
     * @param event the proxy initialisation event
{% elif cap_dual_lang %}
     * 注册示例命令与示例监听器。
     *
     * @param event 代理端初始化事件
     * Registers the example command and the example listener.
     *
     * @param event the proxy initialisation event
{% else %}
     * 注册示例命令与示例监听器。
     *
     * @param event 代理端初始化事件
{% endif %}
     */
    @Subscribe
    public void onProxyInitialize(final ProxyInitializeEvent event) {
        // A proxy has no server configuration file of its own here, so the defaults are inlined.
        final Map<String, Object> config = new HashMap<String, Object>();
        config.put("welcome-message", "&aWelcome, {player}!");
        config.put("broadcast-prefix", "[{{ pluginName }}]");
        config.put("debug", Boolean.FALSE);

        final Map<String, String> templates = new HashMap<String, String>();
        templates.put("welcome", "{message}");
        templates.put("broadcast", "{prefix} {message}");
        templates.put("command-usage", "&eUsage: /{label} <message>");

        final ExampleService service = new ExampleService(PluginConfig.from(config), Messages.from(templates));
        commandManager.register(
                commandManager.metaBuilder("{{ commandName }}").plugin(this).build(),
                new ExampleCommand(service));
        proxy.getEventManager().register(this, new ExampleListener(service));
        logger.info("{{ pluginName }} enabled for Minecraft {{ mcVersion }}");
    }
}
