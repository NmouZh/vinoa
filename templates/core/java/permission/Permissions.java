package {{ packageName }}.permission;

/**
{% if is_lang_en %}
 * Permission nodes declared by this plugin.
 *
 * <p>Every node here also appears in the plugin metadata, so /help and permission listings stay
 * in sync with the code.
{% elif cap_dual_lang %}
 * 本插件声明的权限节点。
 *
 * <p>这里的每个节点都同时出现在插件元数据里，保证 /help 与权限列表和代码一致。
 * Permission nodes declared by this plugin. Every node here also appears in the plugin
 * metadata, so /help and permission listings stay in sync with the code.
{% else %}
 * 本插件声明的权限节点。
 *
 * <p>这里的每个节点都同时出现在插件元数据里，保证 /help 与权限列表和代码一致。
{% endif %}
 */
public final class Permissions {

    /**
{% if is_lang_en %}
     * Guards the example command.
{% elif cap_dual_lang %}
     * 保护示例命令。
     * Guards the example command.
{% else %}
     * 保护示例命令。
{% endif %}
     */
    public static final String COMMAND_EXAMPLE = "{{ permissionNode }}.example";

    /**
{% if is_lang_en %}
     * Administrative umbrella node; it inherits {@link #COMMAND_EXAMPLE}.
{% elif cap_dual_lang %}
     * 管理总节点，继承 {@link #COMMAND_EXAMPLE}。
     * Administrative umbrella node; it inherits {@link #COMMAND_EXAMPLE}.
{% else %}
     * 管理总节点，继承 {@link #COMMAND_EXAMPLE}。
{% endif %}
     */
    public static final String ADMIN = "{{ pluginId }}.admin";

    private Permissions() {
    }
}
