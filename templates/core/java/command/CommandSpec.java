package {{ packageName }}.command;

/**
{% if is_lang_en %}
 * Platform-independent description of one command.
{% elif cap_dual_lang %}
 * 平台无关的单条命令描述。
 * Platform-independent description of one command.
{% else %}
 * 平台无关的单条命令描述。
{% endif %}
 */
public final class CommandSpec {

    private final String name;
    private final String permission;
    private final String usage;

    public CommandSpec(final String name, final String permission, final String usage) {
        this.name = name;
        this.permission = permission;
        this.usage = usage;
    }

    /**
{% if is_lang_en %}
     * @return the command name without the leading slash
{% elif cap_dual_lang %}
     * @return 不含前导斜杠的命令名
     * @return the command name without the leading slash
{% else %}
     * @return 不含前导斜杠的命令名
{% endif %}
     */
    public String name() {
        return name;
    }

    /**
{% if is_lang_en %}
     * @return the permission node required to run the command, or an empty string when disabled
{% elif cap_dual_lang %}
     * @return 运行该命令所需的权限节点；未启用权限时为空串
     * @return the permission node required to run the command, or an empty string when disabled
{% else %}
     * @return 运行该命令所需的权限节点；未启用权限时为空串
{% endif %}
     */
    public String permission() {
        return permission;
    }

    /**
{% if is_lang_en %}
     * @return the usage line, with {label} as the placeholder for the invoked alias
{% elif cap_dual_lang %}
     * @return 用法行，{label} 是实际调用别名的占位符
     * @return the usage line, with {label} as the placeholder for the invoked alias
{% else %}
     * @return 用法行，{label} 是实际调用别名的占位符
{% endif %}
     */
    public String usage() {
        return usage;
    }

    /**
{% if is_lang_en %}
     * Renders the usage line for the alias the sender actually typed.
     *
     * @param label the invoked command label or alias
     * @return the rendered usage line
{% elif cap_dual_lang %}
     * 按玩家实际输入的别名渲染用法行。
     *
     * @param label 实际调用的命令名或别名
     * @return 渲染后的用法行
     * Renders the usage line for the alias the sender actually typed.
     *
     * @param label the invoked command label or alias
     * @return the rendered usage line
{% else %}
     * 按玩家实际输入的别名渲染用法行。
     *
     * @param label 实际调用的命令名或别名
     * @return 渲染后的用法行
{% endif %}
     */
    public String usageFor(final String label) {
        return usage.replace("{label}", label);
    }
}
