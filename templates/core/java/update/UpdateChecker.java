package {{ packageName }}.update;

import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.net.HttpURLConnection;
import java.net.URI;
import java.nio.charset.StandardCharsets;

/**
{% if is_lang_en %}
 * Minimal update check against a plain-text endpoint that returns the latest version.
 *
 * <p>It uses only the JDK, so no extra dependency (and no shading) is needed. Call it off the
 * main thread: every target platform's main thread must stay free for tick work.
{% elif cap_dual_lang %}
 * 对"返回最新版本号的纯文本端点"做最小更新检查。
 *
 * <p>只用 JDK，不需要额外依赖（也就不需要 shading）。请在主线程之外调用：所有目标平台的主
 * 线程都必须留给 tick 逻辑。
 * Minimal update check against a plain-text endpoint that returns the latest version. It uses
 * only the JDK, so no extra dependency (and no shading) is needed. Call it off the main thread.
{% else %}
 * 对"返回最新版本号的纯文本端点"做最小更新检查。
 *
 * <p>只用 JDK，不需要额外依赖（也就不需要 shading）。请在主线程之外调用：所有目标平台的主
 * 线程都必须留给 tick 逻辑。
{% endif %}
 */
public final class UpdateChecker {

    private final String endpoint;
    private final String currentVersion;

    /**
{% if is_lang_en %}
     * @param endpoint absolute URL returning the latest version as plain text
     * @param currentVersion the version this build reports, usually {@code {{ projectVersion }}}
{% elif cap_dual_lang %}
     * @param endpoint 返回最新版本号的纯文本绝对 URL
     * @param currentVersion 本构建的版本，通常是 {@code {{ projectVersion }}}
     * @param endpoint absolute URL returning the latest version as plain text
     * @param currentVersion the version this build reports
{% else %}
     * @param endpoint 返回最新版本号的纯文本绝对 URL
     * @param currentVersion 本构建的版本，通常是 {@code {{ projectVersion }}}
{% endif %}
     */
    public UpdateChecker(final String endpoint, final String currentVersion) {
        this.endpoint = endpoint;
        this.currentVersion = currentVersion;
    }

    /**
{% if is_lang_en %}
     * @return the latest published version, or an empty string when the endpoint is blank
     * @throws IOException when the request fails
{% elif cap_dual_lang %}
     * @return 最新发布版本；端点为空时返回空串
     * @throws IOException 请求失败时抛出
     * @return the latest published version, or an empty string when the endpoint is blank
     * @throws IOException when the request fails
{% else %}
     * @return 最新发布版本；端点为空时返回空串
     * @throws IOException 请求失败时抛出
{% endif %}
     */
    public String latestVersion() throws IOException {
        if (endpoint == null || endpoint.isEmpty()) {
            return "";
        }
        final HttpURLConnection connection = (HttpURLConnection) URI.create(endpoint).toURL().openConnection();
        connection.setRequestMethod("GET");
        connection.setConnectTimeout(5000);
        connection.setReadTimeout(5000);
        connection.setRequestProperty("User-Agent", "{{ pluginName }}/{{ projectVersion }}");
        try (BufferedReader reader = new BufferedReader(
                new InputStreamReader(connection.getInputStream(), StandardCharsets.UTF_8))) {
            final String line = reader.readLine();
            return line == null ? "" : line.trim();
        } finally {
            connection.disconnect();
        }
    }

    /**
{% if is_lang_en %}
     * @return whether the endpoint reports a version different from this build
     * @throws IOException when the request fails
{% elif cap_dual_lang %}
     * @return 端点报告的版本是否与本构建不同
     * @throws IOException 请求失败时抛出
     * @return whether the endpoint reports a version different from this build
     * @throws IOException when the request fails
{% else %}
     * @return 端点报告的版本是否与本构建不同
     * @throws IOException 请求失败时抛出
{% endif %}
     */
    public boolean isUpdateAvailable() throws IOException {
        final String latest = latestVersion();
        return !latest.isEmpty() && !latest.equals(currentVersion);
    }
}
