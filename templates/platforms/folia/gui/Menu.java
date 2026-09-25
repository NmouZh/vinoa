package {{ packageName }}.{{ platform }}.gui;

import java.util.HashMap;
import java.util.Map;
import org.bukkit.Bukkit;
import org.bukkit.entity.Player;
import org.bukkit.event.EventHandler;
import org.bukkit.event.Listener;
import org.bukkit.event.inventory.InventoryClickEvent;
import org.bukkit.inventory.Inventory;
import org.bukkit.inventory.ItemStack;

/**
{% if is_lang_en %}
 * Small chest-menu helper built on the plain Bukkit inventory API.
 *
 * <p>No third-party GUI library is needed: the inventory API exists on every Bukkit-family
 * target. Register the menu as a listener, then call {@link #open(Player)}. On Folia, open a
 * menu from the player's entity scheduler ({@code player.getScheduler()}) instead of a global
 * task.
{% elif cap_dual_lang %}
 * 基于 Bukkit 原生物品栏 API 的小型箱子菜单助手。
 *
 * <p>不需要第三方 GUI 库：物品栏 API 在所有 Bukkit 系目标上都存在。把本对象注册为监听器，
 * 然后调用 {@link #open(Player)}。在 Folia 上请从玩家的 entity scheduler
 * （{@code player.getScheduler()}）里打开菜单，不要用全局任务。
 * Small chest-menu helper built on the plain Bukkit inventory API. No third-party GUI library
 * is needed. On Folia, open a menu from the player's entity scheduler instead of a global task.
{% else %}
 * 基于 Bukkit 原生物品栏 API 的小型箱子菜单助手。
 *
 * <p>不需要第三方 GUI 库：物品栏 API 在所有 Bukkit 系目标上都存在。把本对象注册为监听器，
 * 然后调用 {@link #open(Player)}。在 Folia 上请从玩家的 entity scheduler
 * （{@code player.getScheduler()}）里打开菜单，不要用全局任务。
{% endif %}
 */
public final class Menu implements Listener {

    private final Inventory inventory;
    private final Map<Integer, Runnable> actions = new HashMap<Integer, Runnable>();

    /**
{% if is_lang_en %}
     * @param title the inventory title
     * @param size the inventory size, a multiple of 9
{% elif cap_dual_lang %}
     * @param title 物品栏标题
     * @param size 物品栏大小，必须是 9 的倍数
     * @param title the inventory title
     * @param size the inventory size, a multiple of 9
{% else %}
     * @param title 物品栏标题
     * @param size 物品栏大小，必须是 9 的倍数
{% endif %}
     */
    public Menu(final String title, final int size) {
        this.inventory = Bukkit.createInventory(null, size, title);
    }

    /**
{% if is_lang_en %}
     * Puts an icon in a slot and remembers what a click on it should do.
     *
     * @param slot the slot index
     * @param icon the item shown in that slot
     * @param action the action run when the slot is clicked
     * @return this menu, for chaining
{% elif cap_dual_lang %}
     * 在某个槽位放图标，并记住点击时的动作。
     *
     * @param slot 槽位下标
     * @param icon 该槽位显示的物品
     * @param action 点击该槽位时执行的动作
     * @return 本对象，便于链式调用
     * Puts an icon in a slot and remembers what a click on it should do.
     *
     * @param slot the slot index
     * @param icon the item shown in that slot
     * @param action the action run when the slot is clicked
     * @return this menu, for chaining
{% else %}
     * 在某个槽位放图标，并记住点击时的动作。
     *
     * @param slot 槽位下标
     * @param icon 该槽位显示的物品
     * @param action 点击该槽位时执行的动作
     * @return 本对象，便于链式调用
{% endif %}
     */
    public Menu set(final int slot, final ItemStack icon, final Runnable action) {
        inventory.setItem(slot, icon);
        actions.put(Integer.valueOf(slot), action);
        return this;
    }

    /**
{% if is_lang_en %}
     * @param player the player who should see this menu
{% elif cap_dual_lang %}
     * @param player 打开该菜单的玩家
     * @param player the player who should see this menu
{% else %}
     * @param player 打开该菜单的玩家
{% endif %}
     */
    public void open(final Player player) {
        player.openInventory(inventory);
    }

    /**
{% if is_lang_en %}
     * Cancels clicks in this menu and runs the action bound to the clicked slot.
     *
     * @param event the inventory click event
{% elif cap_dual_lang %}
     * 取消本菜单内的点击，并执行该槽位绑定的动作。
     *
     * @param event 物品栏点击事件
     * Cancels clicks in this menu and runs the action bound to the clicked slot.
     *
     * @param event the inventory click event
{% else %}
     * 取消本菜单内的点击，并执行该槽位绑定的动作。
     *
     * @param event 物品栏点击事件
{% endif %}
     */
    @EventHandler
    public void onClick(final InventoryClickEvent event) {
        if (!inventory.equals(event.getInventory())) {
            return;
        }
        event.setCancelled(true);
        final Runnable action = actions.get(Integer.valueOf(event.getRawSlot()));
        if (action != null) {
            action.run();
        }
    }
}
