package {{ packageName }}.sender;

/**
{% if is_lang_en %}
 * Platform-independent view of whoever triggered an action.
 *
 * <p>Commands and listeners receive a server-specific sender object. The adapter wraps it in
 * this interface so that shared logic can reply and check permissions without importing a
 * server API.
{% elif cap_dual_lang %}
 * 触发动作者的平台无关视图。
 *
 * <p>命令与监听器拿到的是某个服务端的发送者对象；适配层把它包装成本接口，共享逻辑因此无需
 * import 任何服务端 API 就能回消息与查权限。
 * Platform-independent view of whoever triggered an action. Commands and listeners receive a
 * server-specific sender object. The adapter wraps it in this interface so that shared logic
 * can reply and check permissions without importing a server API.
{% else %}
 * 触发动作者的平台无关视图。
 *
 * <p>命令与监听器拿到的是某个服务端的发送者对象；适配层把它包装成本接口，共享逻辑因此无需
 * import 任何服务端 API 就能回消息与查权限。
{% endif %}
 */
public interface Sender {

    /**
{% if is_lang_en %}
     * @return the sender name as shown in game or on the proxy
{% elif cap_dual_lang %}
     * @return 发送者名称（游戏内/代理端显示名）
     * @return the sender name as shown in game or on the proxy
{% else %}
     * @return 发送者名称（游戏内/代理端显示名）
{% endif %}
     */
    String name();

    /**
{% if is_lang_en %}
     * @return whether the sender is a player rather than the console
{% elif cap_dual_lang %}
     * @return 发送者是否为玩家而非控制台
     * @return whether the sender is a player rather than the console
{% else %}
     * @return 发送者是否为玩家而非控制台
{% endif %}
     */
    boolean isPlayer();

    /**
{% if is_lang_en %}
     * @param node the permission node to test
     * @return whether the sender holds the node
{% elif cap_dual_lang %}
     * @param node 要检查的权限节点
     * @return 发送者是否拥有该节点
     * @param node the permission node to test
     * @return whether the sender holds the node
{% else %}
     * @param node 要检查的权限节点
     * @return 发送者是否拥有该节点
{% endif %}
     */
    boolean hasPermission(String node);

    /**
{% if is_lang_en %}
     * @param message the message to deliver
{% elif cap_dual_lang %}
     * @param message 要发送的消息
     * @param message the message to deliver
{% else %}
     * @param message 要发送的消息
{% endif %}
     */
    void sendMessage(String message);
}
