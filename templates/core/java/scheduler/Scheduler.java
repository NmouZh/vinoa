package {{ packageName }}.scheduler;

/**
{% if is_lang_en %}
 * Platform-independent scheduling port.
 *
 * <p>A platform adapter implements this with its own scheduler: the Bukkit main-thread
 * scheduler, Folia's region scheduler or a proxy executor. Shared logic only ever sees this
 * interface, so it stays loadable on every target.
{% elif cap_dual_lang %}
 * 平台无关的调度端口。
 *
 * <p>各平台适配层用自己的调度器实现：Bukkit 主线程调度器、Folia 的 region 调度器或代理端的
 * executor。共享逻辑只看到这个接口，因此能在所有目标上加载。
 * Platform-independent scheduling port. A platform adapter implements this with its own
 * scheduler: the Bukkit main-thread scheduler, Folia's region scheduler or a proxy executor.
 * Shared logic only ever sees this interface, so it stays loadable on every target.
{% else %}
 * 平台无关的调度端口。
 *
 * <p>各平台适配层用自己的调度器实现：Bukkit 主线程调度器、Folia 的 region 调度器或代理端的
 * executor。共享逻辑只看到这个接口，因此能在所有目标上加载。
{% endif %}
 */
public interface Scheduler {

    /**
{% if is_lang_en %}
     * @param task work that must run on the server or proxy main thread
{% elif cap_dual_lang %}
     * @param task 必须在服务端/代理端主线程执行的任务
     * @param task work that must run on the server or proxy main thread
{% else %}
     * @param task 必须在服务端/代理端主线程执行的任务
{% endif %}
     */
    void runSync(Runnable task);

    /**
{% if is_lang_en %}
     * @param task work that may run off the main thread
{% elif cap_dual_lang %}
     * @param task 可以离开主线程执行的任务
     * @param task work that may run off the main thread
{% else %}
     * @param task 可以离开主线程执行的任务
{% endif %}
     */
    void runAsync(Runnable task);
}
