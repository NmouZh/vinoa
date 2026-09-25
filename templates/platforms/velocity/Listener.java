package {{ packageName }}.{{ platform }}.listener;

import com.velocitypowered.api.event.Subscribe;
import com.velocitypowered.api.event.connection.PostLoginEvent;
import {{ packageName }}.ExampleService;
import net.kyori.adventure.text.Component;

/**
{% if is_lang_en %}
 * The example proxy listener: greets a player who logs in.
 *
 * <p>The main class is registered as a listener with the proxy event manager; a separate class
 * like this one has to be registered explicitly.
{% elif cap_dual_lang %}
 * 示例代理端监听器：问候登录的玩家。
 *
 * <p>主类由代理端事件管理器自动注册；像本类这样独立的监听器需要显式注册。
 * The example proxy listener: greets a player who logs in. The main class is registered as a
 * listener with the proxy event manager; a separate class like this one has to be registered
 * explicitly.
{% else %}
 * 示例代理端监听器：问候登录的玩家。
 *
 * <p>主类由代理端事件管理器自动注册；像本类这样独立的监听器需要显式注册。
{% endif %}
 */
public final class ExampleListener {

    private final ExampleService service;

    public ExampleListener(final ExampleService service) {
        this.service = service;
    }

    @Subscribe
    public void onPostLogin(final PostLoginEvent event) {
        event.getPlayer().sendMessage(Component.text(service.welcomeMessage(event.getPlayer().getUsername())));
    }
}
