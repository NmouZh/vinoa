package {{ packageName }}.{{ platform }};

import {{ packageName }}.ExampleService;
import {{ packageName }}.config.PluginConfig;
import {{ packageName }}.message.Messages;
import {{ packageName }}.{{ platform }}.command.ExampleCommand;
import {{ packageName }}.{{ platform }}.listener.ExampleListener;
import java.util.HashMap;
import java.util.Map;
import net.minestom.server.MinecraftServer;

/**
{% if is_lang_en %}
 * Minestom bootstrap (experimental skeleton).
 *
 * <p>Minestom is a library, not a packaged server: the embedding application creates the
 * server first and then calls {@link #bootstrap(MinecraftServer)}. Add the command and event
 * registrations against the exact Minestom version in use.
{% elif cap_dual_lang %}
 * Minestom 入口（实验性骨架）。
 *
 * <p>Minestom 是库而不是打包服务端：宿主应用先创建服务器，再调用
 * {@link #bootstrap(MinecraftServer)}。命令与事件注册请针对实际使用的 Minestom 版本补全。
 * Minestom bootstrap (experimental skeleton). Minestom is a library, not a packaged server:
 * the embedding application creates the server first and then calls bootstrap(MinecraftServer).
 * Add the command and event registrations against the exact Minestom version in use.
{% else %}
 * Minestom 入口（实验性骨架）。
 *
 * <p>Minestom 是库而不是打包服务端：宿主应用先创建服务器，再调用
 * {@link #bootstrap(MinecraftServer)}。命令与事件注册请针对实际使用的 Minestom 版本补全。
{% endif %}
 */
public final class {{ pluginName }} {

    private final ExampleCommand command;
    private final ExampleListener listener;

    /**
{% if is_lang_en %}
     * Builds the shared service and the example adapters.
{% elif cap_dual_lang %}
     * 构建共享服务与示例适配器。
     * Builds the shared service and the example adapters.
{% else %}
     * 构建共享服务与示例适配器。
{% endif %}
     */
    public {{ pluginName }}() {
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
    }

    /**
{% if is_lang_en %}
     * Called by the embedding application once the server exists.
     *
     * @param server the already-created Minestom server
     * @return the bootstrap instance holding the adapters
{% elif cap_dual_lang %}
     * 由宿主应用在服务器创建完成后调用。
     *
     * @param server 已创建的 Minestom 服务器
     * @return 持有适配器的实例
     * Called by the embedding application once the server exists.
     *
     * @param server the already-created Minestom server
     * @return the bootstrap instance holding the adapters
{% else %}
     * 由宿主应用在服务器创建完成后调用。
     *
     * @param server 已创建的 Minestom 服务器
     * @return 持有适配器的实例
{% endif %}
     */
    public static {{ pluginName }} bootstrap(final MinecraftServer server) {
        // Registration goes here, against the exact Minestom version:
        //   server.getCommandManager() ... with new ExampleCommand(...)
        //   server.getGlobalEventHandler() ... with new ExampleListener(...)
        return new {{ pluginName }}();
    }

    /**
{% if is_lang_en %}
     * @return the command behaviour
{% elif cap_dual_lang %}
     * @return 命令行为对象
     * @return the command behaviour
{% else %}
     * @return 命令行为对象
{% endif %}
     */
    public ExampleCommand command() {
        return command;
    }

    /**
{% if is_lang_en %}
     * @return the listener behaviour
{% elif cap_dual_lang %}
     * @return 监听器行为对象
     * @return the listener behaviour
{% else %}
     * @return 监听器行为对象
{% endif %}
     */
    public ExampleListener listener() {
        return listener;
    }
}
