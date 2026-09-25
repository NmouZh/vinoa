package {{ packageName }}.{{ platform }}.command;

import {{ packageName }}.ExampleService;
import java.util.Collections;
import java.util.List;
import org.bukkit.Bukkit;
import org.bukkit.command.Command;
import org.bukkit.command.CommandExecutor;
import org.bukkit.command.CommandSender;
import org.bukkit.command.TabCompleter;

/**
{% if is_lang_en %}
 * The example command: broadcasts its argument, or prints the usage line when it has none.
 *
 * <p>The command name, aliases and permission are declared in plugin.yml. This class only
 * implements the behaviour.
{% elif cap_dual_lang %}
 * 示例命令：有参数就广播，无参数就打印用法。
 *
 * <p>命令名、别名与权限声明在 plugin.yml 里，本类只实现行为。
 * The example command: broadcasts its argument, or prints the usage line when it has none.
 * The command name, aliases and permission are declared in plugin.yml.
{% else %}
 * 示例命令：有参数就广播，无参数就打印用法。
 *
 * <p>命令名、别名与权限声明在 plugin.yml 里，本类只实现行为。
{% endif %}
 */
public final class ExampleCommand implements CommandExecutor, TabCompleter {

    private final ExampleService service;

    public ExampleCommand(final ExampleService service) {
        this.service = service;
    }

    @Override
    public boolean onCommand(final CommandSender sender, final Command command, final String label,
            final String[] args) {
        if (args.length == 0) {
            sender.sendMessage(service.usage(label));
            return true;
        }
        Bukkit.broadcastMessage(service.broadcast(String.join(" ", args)));
        return true;
    }

    @Override
    public List<String> onTabComplete(final CommandSender sender, final Command command, final String alias,
            final String[] args) {
        return Collections.emptyList();
    }
}
