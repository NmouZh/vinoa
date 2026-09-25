package {{ packageName }}.{{ platform }}.listener;

import {{ packageName }}.ExampleService;
import net.md_5.bungee.api.event.PostLoginEvent;
import net.md_5.bungee.api.plugin.Listener;
import net.md_5.bungee.event.EventHandler;

/**
{% if is_lang_en %}
 * The example proxy listener: greets a player who logs in.
{% elif cap_dual_lang %}
 * 示例代理端监听器：问候登录的玩家。
 * The example proxy listener: greets a player who logs in.
{% else %}
 * 示例代理端监听器：问候登录的玩家。
{% endif %}
 */
public final class ExampleListener implements Listener {

    private final ExampleService service;

    public ExampleListener(final ExampleService service) {
        this.service = service;
    }

    @EventHandler
    public void onPostLogin(final PostLoginEvent event) {
        event.getPlayer().sendMessage(service.welcomeMessage(event.getPlayer().getName()));
    }
}
