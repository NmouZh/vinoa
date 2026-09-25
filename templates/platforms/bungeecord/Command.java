package {{ packageName }}.{{ platform }}.command;

import {{ packageName }}.ExampleService;
import java.util.Collections;
import net.md_5.bungee.api.CommandSender;
import net.md_5.bungee.api.ProxyServer;
import net.md_5.bungee.api.plugin.Command;
import net.md_5.bungee.api.plugin.TabExecutor;

/**
{% if is_lang_en %}
 * The example proxy command.
 *
 * <p>The name and permission are constructor arguments: a BungeeCord plugin.yml has no
 * commands: section to declare them in.
{% elif cap_dual_lang %}
 * 示例代理端命令。
 *
 * <p>命令名与权限是构造参数：BungeeCord 的 plugin.yml 没有 commands: 段可声明。
 * The example proxy command. The name and permission are constructor arguments: a BungeeCord
 * plugin.yml has no commands: section to declare them in.
{% else %}
 * 示例代理端命令。
 *
 * <p>命令名与权限是构造参数：BungeeCord 的 plugin.yml 没有 commands: 段可声明。
{% endif %}
 */
public final class ExampleCommand extends Command implements TabExecutor {

    private final ExampleService service;

    public ExampleCommand(final ExampleService service) {
        super("{{ commandName }}"{% if cap_permissions %}, "{{ permissionNode }}.example"{% endif %});
        this.service = service;
    }

    @Override
    public void execute(final CommandSender sender, final String[] args) {
        if (args.length == 0) {
            sender.sendMessage(service.usage("{{ commandName }}"));
            return;
        }
        ProxyServer.getInstance().broadcast(service.broadcast(String.join(" ", args)));
    }

    @Override
    public Iterable<String> onTabComplete(final CommandSender sender, final String[] args) {
        return Collections.emptyList();
    }
}
