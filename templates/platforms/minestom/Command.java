package {{ packageName }}.{{ platform }}.command;

import {{ packageName }}.ExampleService;

/**
{% if is_lang_en %}
 * Command behaviour for the Minestom line (experimental skeleton).
 *
 * <p>It intentionally has no Minestom API type in its signature: the bootstrap registers it
 * against whichever API line the version matrix selected.
{% elif cap_dual_lang %}
 * Minestom 线的命令行为（实验性骨架）。
 *
 * <p>签名里刻意不出现 Minestom API 类型：由 bootstrap 针对版本矩阵选定的 API 线接入注册。
 * Command behaviour for the Minestom line (experimental skeleton). It intentionally has no
 * Minestom API type in its signature: the bootstrap registers it against whichever API line the
 * version matrix selected.
{% else %}
 * Minestom 线的命令行为（实验性骨架）。
 *
 * <p>签名里刻意不出现 Minestom API 类型：由 bootstrap 针对版本矩阵选定的 API 线接入注册。
{% endif %}
 */
public final class ExampleCommand {

    private final ExampleService service;

    public ExampleCommand(final ExampleService service) {
        this.service = service;
    }

    /**
{% if is_lang_en %}
     * @return the usage line for the sender
{% elif cap_dual_lang %}
     * @return 给发送者的用法提示
     * @return the usage line for the sender
{% else %}
     * @return 给发送者的用法提示
{% endif %}
     */
    public String usage() {
        return service.usage("{{ commandName }}");
    }

    /**
{% if is_lang_en %}
     * @param args the raw command arguments
     * @return the broadcast line
{% elif cap_dual_lang %}
     * @param args 原始命令参数
     * @return 广播文本
     * @param args the raw command arguments
     * @return the broadcast line
{% else %}
     * @param args 原始命令参数
     * @return 广播文本
{% endif %}
     */
    public String broadcast(final String[] args) {
        return service.broadcast(String.join(" ", args));
    }
}
