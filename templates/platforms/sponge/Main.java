package {{ packageName }}.{{ platform }};

import {{ packageName }}.ExampleService;
import {{ packageName }}.config.PluginConfig;
import {{ packageName }}.message.Messages;
import {{ packageName }}.{{ platform }}.command.ExampleCommand;
import {{ packageName }}.{{ platform }}.listener.ExampleListener;
import com.google.inject.Inject;
import java.util.HashMap;
import java.util.Map;
import org.apache.logging.log4j.Logger;
import org.spongepowered.plugin.PluginContainer;
import org.spongepowered.plugin.builtin.jvm.Plugin;

/**
{% if is_lang_en %}
 * Sponge entry point (experimental skeleton).
 *
 * <p>There is no metadata file: the @Plugin annotation plus code registration describe the
 * plugin. SpongeAPI renames its lifecycle and command APIs between release lines, so wire the
 * registrations up against the exact API line this project targets.
{% elif cap_dual_lang %}
 * Sponge 入口（实验性骨架）。
 *
 * <p>没有元数据文件：描述符由 @Plugin 注解加代码注册组成。SpongeAPI 的 lifecycle 与 command
 * API 在不同发行线之间会改名，请针对本项目实际目标的 API 线接入注册逻辑。
 * Sponge entry point (experimental skeleton). There is no metadata file: the @Plugin
 * annotation plus code registration describe the plugin. SpongeAPI renames its lifecycle and
 * command APIs between release lines, so wire the registrations up against the exact API line
 * this project targets.
{% else %}
 * Sponge 入口（实验性骨架）。
 *
 * <p>没有元数据文件：描述符由 @Plugin 注解加代码注册组成。SpongeAPI 的 lifecycle 与 command
 * API 在不同发行线之间会改名，请针对本项目实际目标的 API 线接入注册逻辑。
{% endif %}
 */
@Plugin("{{ pluginId }}")
public final class {{ pluginName }} {

    private final PluginContainer container;
    private final Logger logger;
    private final ExampleCommand command;
    private final ExampleListener listener;

    /**
{% if is_lang_en %}
     * @param container this plugin's container
     * @param logger the plugin logger
{% elif cap_dual_lang %}
     * @param container 本插件的容器
     * @param logger 插件日志器
     * @param container this plugin's container
     * @param logger the plugin logger
{% else %}
     * @param container 本插件的容器
     * @param logger 插件日志器
{% endif %}
     */
    @Inject
    {{ pluginName }}(final PluginContainer container, final Logger logger) {
        this.container = container;
        this.logger = logger;

        final Map<String, Object> config = new HashMap<String, Object>();
        config.put("welcome-message", "&aWelcome, {player}!");
        config.put("broadcast-prefix", "[{{ pluginName }}]");
        config.put("debug", Boolean.FALSE);

        final Map<String, String> templates = new HashMap<String, String>();
        templates.put("welcome", "{message}");
        templates.put("broadcast", "{prefix} {message}");
        templates.put("command-usage", "&eUsage: /{label} <message>");

        final ExampleService service = new ExampleService(PluginConfig.from(config), Messages.from(templates));
        this.command = new ExampleCommand(service);
        this.listener = new ExampleListener(service);
        this.logger.info("{{ pluginName }} container initialised for Minecraft {{ mcVersion }}");
    }

    /**
{% if is_lang_en %}
     * @return the command behaviour, to be registered in the Sponge lifecycle event
{% elif cap_dual_lang %}
     * @return 命令行为对象，请在 Sponge lifecycle 事件里注册
     * @return the command behaviour, to be registered in the Sponge lifecycle event
{% else %}
     * @return 命令行为对象，请在 Sponge lifecycle 事件里注册
{% endif %}
     */
    public ExampleCommand command() {
        return command;
    }

    /**
{% if is_lang_en %}
     * @return the listener behaviour, to be registered in the Sponge lifecycle event
{% elif cap_dual_lang %}
     * @return 监听器行为对象，请在 Sponge lifecycle 事件里注册
     * @return the listener behaviour, to be registered in the Sponge lifecycle event
{% else %}
     * @return 监听器行为对象，请在 Sponge lifecycle 事件里注册
{% endif %}
     */
    public ExampleListener listener() {
        return listener;
    }
}
