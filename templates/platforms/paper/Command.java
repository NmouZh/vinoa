package {{ packageName }}.{{ platform }}.command;

import {{ packageName }}.ExampleService;
{% if cap_paper_brigadier %}
{% if cap_permissions %}
import {{ packageName }}.permission.Permissions;
{% endif %}
import io.papermc.paper.command.brigadier.BasicCommand;
import io.papermc.paper.command.brigadier.CommandSourceStack;
import org.bukkit.Bukkit;
import org.bukkit.command.CommandSender;
{% else %}
import java.util.Collections;
import java.util.List;
import org.bukkit.Bukkit;
import org.bukkit.command.Command;
import org.bukkit.command.CommandExecutor;
import org.bukkit.command.CommandSender;
import org.bukkit.command.TabCompleter;
{% endif %}

/**
{% if is_lang_en %}
 * The example command: broadcasts its argument, or prints the usage line when it has none.
{% elif cap_dual_lang %}
 * 示例命令：有参数就广播，无参数就打印用法。
 * The example command: broadcasts its argument, or prints the usage line when it has none.
{% else %}
 * 示例命令：有参数就广播，无参数就打印用法。
{% endif %}
 */
{% if cap_paper_brigadier %}
public final class ExampleCommand implements BasicCommand {
{% else %}
public final class ExampleCommand implements CommandExecutor, TabCompleter {
{% endif %}

    private final ExampleService service;

    public ExampleCommand(final ExampleService service) {
        this.service = service;
    }
{% if cap_paper_brigadier %}

    @Override
    public void execute(final CommandSourceStack source, final String[] args) {
        final CommandSender sender = source.getSender();
        if (args.length == 0) {
            sender.sendMessage(service.usage("{{ commandName }}"));
            return;
        }
        Bukkit.broadcastMessage(service.broadcast(String.join(" ", args)));
    }
{% if cap_permissions %}

    @Override
    public String permission() {
        return Permissions.COMMAND_EXAMPLE;
    }
{% endif %}
{% else %}

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
{% endif %}
}
