package {{ packageName }}.storage;

import java.nio.file.Path;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;

/**
{% if is_lang_en %}
 * SQLite implementation of {@link Storage}.
 *
 * <p>Migrations are the two-digit {@code PRAGMA user_version} alone: no Flyway, no Liquibase.
 * To change the layout, bump {@link #SCHEMA_VERSION} and add the matching step to
 * {@link #migrate(Connection)}. The class only uses {@code java.sql}, so it compiles on a Java 8
 * target as well.
{% elif cap_dual_lang %}
 * {@link Storage} 的 SQLite 实现。
 *
 * <p>迁移只用两位数的 {@code PRAGMA user_version}：不引 Flyway，也不引 Liquibase。要改结构就
 * 提升 {@link #SCHEMA_VERSION} 并在 {@link #migrate(Connection)} 里加对应步骤。本类只用
 * {@code java.sql}，因此在 Java 8 目标上同样能编译。
 * SQLite implementation of Storage. Migrations are the two-digit PRAGMA user_version alone:
 * no Flyway, no Liquibase. The class only uses java.sql, so it compiles on a Java 8 target too.
{% else %}
 * {@link Storage} 的 SQLite 实现。
 *
 * <p>迁移只用两位数的 {@code PRAGMA user_version}：不引 Flyway，也不引 Liquibase。要改结构就
 * 提升 {@link #SCHEMA_VERSION} 并在 {@link #migrate(Connection)} 里加对应步骤。本类只用
 * {@code java.sql}，因此在 Java 8 目标上同样能编译。
{% endif %}
 */
public final class SqliteStorage implements Storage {

    /**
     * Bump this when the layout changes and add the matching step in {@link #migrate(Connection)}.
     * It is rendered with two digits, so {@code 1} is stored as {@code "01"}.
     */
    private static final int SCHEMA_VERSION = 1;

    private final String jdbcUrl;
    private Connection connection;

    /**
{% if is_lang_en %}
     * @param databaseFile the SQLite file to open (created on first use)
{% elif cap_dual_lang %}
     * @param databaseFile 要打开的 SQLite 文件（首次使用时创建）
     * @param databaseFile the SQLite file to open (created on first use)
{% else %}
     * @param databaseFile 要打开的 SQLite 文件（首次使用时创建）
{% endif %}
     */
    public SqliteStorage(final Path databaseFile) {
        this("jdbc:sqlite:" + databaseFile.toAbsolutePath());
    }

    /**
{% if is_lang_en %}
     * @param jdbcUrl a full JDBC URL, for example {@code jdbc:sqlite:/srv/plugins/data.db}
{% elif cap_dual_lang %}
     * @param jdbcUrl 完整 JDBC URL，例如 {@code jdbc:sqlite:/srv/plugins/data.db}
     * @param jdbcUrl a full JDBC URL
{% else %}
     * @param jdbcUrl 完整 JDBC URL，例如 {@code jdbc:sqlite:/srv/plugins/data.db}
{% endif %}
     */
    public SqliteStorage(final String jdbcUrl) {
        this.jdbcUrl = jdbcUrl;
    }

    /**
{% if is_lang_en %}
     * Opens the database and applies every pending schema step.
     *
     * @return this storage, ready to use
     * @throws SQLException when the database cannot be opened or migrated
{% elif cap_dual_lang %}
     * 打开数据库并应用所有待执行的 schema 步骤。
     *
     * @return 可直接使用的本对象
     * @throws SQLException 数据库无法打开或迁移时抛出
     * Opens the database and applies every pending schema step.
     *
     * @return this storage, ready to use
     * @throws SQLException when the database cannot be opened or migrated
{% else %}
     * 打开数据库并应用所有待执行的 schema 步骤。
     *
     * @return 可直接使用的本对象
     * @throws SQLException 数据库无法打开或迁移时抛出
{% endif %}
     */
    public SqliteStorage open() throws SQLException {
        this.connection = DriverManager.getConnection(jdbcUrl);
        migrate(connection);
        return this;
    }

    @Override
    public String schemaVersion() {
        return String.format("%02d", Integer.valueOf(SCHEMA_VERSION));
    }

    @Override
    public int readInt(final String key, final int fallback) {
        try (PreparedStatement statement =
                connection.prepareStatement("SELECT value FROM plugin_kv WHERE key = ?")) {
            statement.setString(1, key);
            try (ResultSet result = statement.executeQuery()) {
                if (!result.next()) {
                    return fallback;
                }
                return Integer.parseInt(result.getString(1));
            }
        } catch (SQLException exception) {
            return fallback;
        } catch (NumberFormatException exception) {
            return fallback;
        }
    }

    @Override
    public void writeInt(final String key, final int value) {
        try (PreparedStatement statement = connection.prepareStatement(
                "INSERT INTO plugin_kv (key, value) VALUES (?, ?) "
                        + "ON CONFLICT(key) DO UPDATE SET value = excluded.value")) {
            statement.setString(1, key);
            statement.setString(2, Integer.toString(value));
            statement.executeUpdate();
        } catch (SQLException exception) {
            throw new IllegalStateException("Could not store key " + key, exception);
        }
    }

    @Override
    public void close() {
        if (connection == null) {
            return;
        }
        try {
            connection.close();
        } catch (SQLException ignored) {
            // Closing a SQLite handle has nothing meaningful to recover from.
        }
    }

    private static void migrate(final Connection connection) throws SQLException {
        final int current = userVersion(connection);
        if (current < 1) {
            try (Statement statement = connection.createStatement()) {
                statement.execute("CREATE TABLE IF NOT EXISTS plugin_kv ("
                        + "key TEXT PRIMARY KEY, value TEXT NOT NULL)");
            }
        }
        setUserVersion(connection, SCHEMA_VERSION);
    }

    private static int userVersion(final Connection connection) throws SQLException {
        try (Statement statement = connection.createStatement();
                ResultSet result = statement.executeQuery("PRAGMA user_version")) {
            return result.next() ? result.getInt(1) : 0;
        }
    }

    private static void setUserVersion(final Connection connection, final int version) throws SQLException {
        // PRAGMA does not take bind parameters. The value is this class's own constant, never
        // user input, so building the statement text here cannot be injected.
        try (Statement statement = connection.createStatement()) {
            statement.execute("PRAGMA user_version = " + version);
        }
    }
}
