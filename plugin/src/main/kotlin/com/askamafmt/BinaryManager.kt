package com.askamafmt

import com.intellij.openapi.util.SystemInfo
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import java.util.concurrent.TimeUnit

/**
 * Locates the `askama_fmt` executable.
 *
 * The plugin does not ship a binary: askama_fmt is a Rust tool and every user
 * of an Askama formatter already has cargo. Bundling it only ever produced
 * stale copies that shadowed the one the user installed.
 *
 * Resolution order: the path configured in settings, then `$PATH`, then
 * cargo's install root.
 */
object BinaryManager {

    const val INSTALL_HINT: String =
        "askama_fmt was not found.\n\n" +
            "Install it with:\n" +
            "    cargo install askama_fmt\n\n" +
            "If it lives somewhere unusual, set the path under " +
            "Settings → Tools → Askama Formatter."

    private var cached: Path? = null

    fun resolve(): Path {
        cached?.let { if (isUsable(it)) return it }
        val path = fromSettings() ?: fromPath() ?: fromCargoBin() ?: error(INSTALL_HINT)
        cached = path
        return path
    }

    /** Forget the resolved binary, e.g. after the configured path changes. */
    fun invalidate() {
        cached = null
    }

    /** Explicit override from Settings → Tools → Askama Formatter. */
    private fun fromSettings(): Path? {
        val configured = AskamaFmtSettings.instance.binaryPath.trim()
        if (configured.isEmpty()) return null
        val path = Paths.get(configured)
        if (!isUsable(path)) {
            error("The configured askama_fmt path is not an executable file:\n    $configured")
        }
        return path
    }

    /**
     * `which` / `where`. This only sees the environment the IDE was launched
     * with — see [fromCargoBin] for why that is not enough on its own.
     */
    private fun fromPath(): Path? = runCatching {
        val cmd = listOf(if (SystemInfo.isWindows) "where" else "which", binaryName())
        val process = ProcessBuilder(cmd).start()
        // Deliberately not redirectErrorStream(true): a miss prints to stderr
        // ("which: no askama_fmt in (...)"), and merging that into stdout makes
        // the error text look like a path.
        val out = process.inputStream.reader().use { it.readText() }
        if (!process.waitFor(5, TimeUnit.SECONDS)) {
            process.destroy()
            return@runCatching null
        }
        if (process.exitValue() != 0) return@runCatching null
        out.lineSequence()
            .map(String::trim)
            .firstOrNull { it.isNotEmpty() }
            ?.let { Paths.get(it) }
    }.getOrNull()?.takeIf { isUsable(it) }

    /**
     * Cargo's install root, probed directly.
     *
     * A GUI-launched IDE does not source the user's shell profile, so
     * `~/.cargo/bin` is routinely missing from its `PATH` even though the
     * binary is installed and works fine in a terminal.
     */
    private fun fromCargoBin(): Path? {
        val root = System.getenv("CARGO_HOME")?.takeIf { it.isNotBlank() }
            ?: run {
                val home = System.getProperty("user.home") ?: return null
                Paths.get(home, ".cargo").toString()
            }
        return Paths.get(root, "bin", binaryName()).takeIf { isUsable(it) }
    }

    private fun isUsable(path: Path): Boolean =
        Files.isRegularFile(path) && Files.isExecutable(path)

    private fun binaryName(): String =
        if (SystemInfo.isWindows) "askama_fmt.exe" else "askama_fmt"
}
