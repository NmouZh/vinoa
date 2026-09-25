package {{ packageName }}.{{ platform }}.listener;

import {{ packageName }}.ExampleService;
import org.bukkit.event.EventHandler;
import org.bukkit.event.Listener;
import org.bukkit.event.player.PlayerJoinEvent;

/**
{% if is_lang_en %}
 * The example listener: greets a joining player with the configured welcome message.
{% elif cap_dual_lang %}
 * 示例监听器：用配置里的欢迎消息问候加入的玩家。
 * The example listener: greets a joining player with the configured welcome message.
{% else %}
 * 示例监听器：用配置里的欢迎消息问候加入的玩家。
{% endif %}
 */
public final class ExampleListener implements Listener {

    private final ExampleService service;

    public ExampleListener(final ExampleService service) {
        this.service = service;
    }

    @EventHandler
    public void onPlayerJoin(final PlayerJoinEvent event) {
        event.getPlayer().sendMessage(service.welcomeMessage(event.getPlayer().getName()));
    }
}
