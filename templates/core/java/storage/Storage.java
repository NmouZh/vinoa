package {{ packageName }}.storage;

/**
{% if is_lang_en %}
 * Platform-independent key/value persistence port.
 *
 * <p>The shared logic only sees this interface; the SQLite implementation (or any other) stays
 * behind it, so nothing in {@code core} has to know about JDBC.
{% elif cap_dual_lang %}
 * 平台无关的键值持久化端口。
 *
 * <p>共享逻辑只看到这个接口，SQLite 实现（或任何别的实现）留在它后面，因此 {@code core}
 * 里没有任何地方需要知道 JDBC。
 * Platform-independent key/value persistence port. The shared logic only sees this interface;
 * the SQLite implementation stays behind it.
{% else %}
 * 平台无关的键值持久化端口。
 *
 * <p>共享逻辑只看到这个接口，SQLite 实现（或任何别的实现）留在它后面，因此 {@code core}
 * 里没有任何地方需要知道 JDBC。
{% endif %}
 */
public interface Storage extends AutoCloseable {

    /**
{% if is_lang_en %}
     * @return the on-disk schema version, formatted with two digits (for example {@code "01"})
{% elif cap_dual_lang %}
     * @return 磁盘上的 schema 版本号，两位数格式（例如 {@code "01"}）
     * @return the on-disk schema version, formatted with two digits
{% else %}
     * @return 磁盘上的 schema 版本号，两位数格式（例如 {@code "01"}）
{% endif %}
     */
    String schemaVersion();

    /**
{% if is_lang_en %}
     * @param key the entry key
     * @param fallback the value returned when the key is absent or unreadable
     * @return the stored integer, or {@code fallback}
{% elif cap_dual_lang %}
     * @param key 条目键
     * @param fallback 键不存在或不可读时返回的值
     * @return 存储的整数，或 {@code fallback}
     * @param key the entry key
     * @param fallback the value returned when the key is absent or unreadable
     * @return the stored integer, or {@code fallback}
{% else %}
     * @param key 条目键
     * @param fallback 键不存在或不可读时返回的值
     * @return 存储的整数，或 {@code fallback}
{% endif %}
     */
    int readInt(String key, int fallback);

    /**
{% if is_lang_en %}
     * @param key the entry key
     * @param value the integer to store
{% elif cap_dual_lang %}
     * @param key 条目键
     * @param value 要存储的整数
     * @param key the entry key
     * @param value the integer to store
{% else %}
     * @param key 条目键
     * @param value 要存储的整数
{% endif %}
     */
    void writeInt(String key, int value);

    @Override
    void close();
}
