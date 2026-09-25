package {{ packageName }}.{{ platform }}.command;

import com.velocitypowered.api.command.CommandSource;
import com.velocitypowered.api.command.SimpleCommand;
import {{ packageName }}.ExampleService;
import java.util.Collections;
import java.util.List;
import net.kyori.adventure.text.Component;

/**
{% if is_lang_en %}
 * The example proxy command.
 *
 * <p>The command is registered in code only: a proxy module has no metadata file, and Velocity
 * derives the descriptor from the main class annotation.
{% elif cap_dual_lang %}
 * 示例代理端命令。
 *
 * <p>命令只在代码里注册：代理端模块没有元数据文件，描述符由主类注解生成。
 * The example proxy command. The command is registered in code only: a proxy module has no
 * metadata file, and Velocity derives the descriptor from the main class annotation.
{% else %}
 * 示例代理端命令。
 *
 * <p>命令只在代码里注册：代理端模块没有元数据文件，描述符由主类注解生成。
{% endif %}
 */
public final class ExampleCommand implements SimpleCommand {

    private final ExampleService service;

    public ExampleCommand(final ExampleService service) {
        this.service = service;
    }

    @Override
    public void execute(final Invocation invocation) {
        final CommandSource source = invocation.source();
        final String[] args = invocation.arguments();
        if (args.length == 0) {
            source.sendMessage(Component.text(service.usage("{{ commandName }}")));
            return;
        }
        // A proxy has no server-wide broadcast helper: fan the message out over
        // ProxyServer#getAllPlayers() when you add one.
        source.sendMessage(Component.text(service.broadcast(String.join(" ", args))));
    }

    @Override
    public List<String> suggest(final Invocation invocation) {
        return Collections.emptyList();
    }

    @Override
    public boolean hasPermission(final Invocation invocation) {
{% if cap_permissions %}
        return invocation.source().hasPermission("{{ permissionNode }}.example");
{% else %}
        return true;
{% endif %}
    }
}
