package {{ packageName }}.{{ platform }}.listener;

import {{ packageName }}.ExampleService;

/**
{% if is_lang_en %}
 * Listener behaviour for the Sponge line (experimental skeleton).
 *
 * <p>The SpongeAPI event type is line-specific, so the bootstrap owns the registration.
{% elif cap_dual_lang %}
 * Sponge 线的监听器行为（实验性骨架）。
 *
 * <p>SpongeAPI 的事件类型依发行线而变，注册由 bootstrap 负责。
 * Listener behaviour for the Sponge line (experimental skeleton). The SpongeAPI event type is
 * line-specific, so the bootstrap owns the registration.
{% else %}
 * Sponge 线的监听器行为（实验性骨架）。
 *
 * <p>SpongeAPI 的事件类型依发行线而变，注册由 bootstrap 负责。
{% endif %}
 */
public final class ExampleListener {

    private final ExampleService service;

    public ExampleListener(final ExampleService service) {
        this.service = service;
    }

    /**
{% if is_lang_en %}
     * @param playerName the joining player
     * @return the welcome message
{% elif cap_dual_lang %}
     * @param playerName 加入的玩家名
     * @return 欢迎消息
     * @param playerName the joining player
     * @return the welcome message
{% else %}
     * @param playerName 加入的玩家名
     * @return 欢迎消息
{% endif %}
     */
    public String welcome(final String playerName) {
        return service.welcomeMessage(playerName);
    }
}
