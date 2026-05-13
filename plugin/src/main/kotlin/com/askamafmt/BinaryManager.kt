package com.askamafmt

import com.intellij.openapi.util.SystemInfo
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import java.nio.file.StandardCopyOption
import java.nio.file.attribute.PosixFilePermissions

object BinaryManager {

    private var cached: Path? = null

    fun resolve(): Path {
        cached?.let { if (Files.isExecutable(it)) return it }
        val path = fromPath() ?: fromBundle()
        cached = path
        return path
    }

    private fun fromPath(): Path? {
        val cmd = if (SystemInfo.isWindows) listOf("where", "askama_fmt") else listOf("which", "askama_fmt")
        return runCatching {
            val out = ProcessBuilder(cmd)
                .redirectErrorStream(true)
                .start()
                .inputStream.reader().readText().trim()
            if (out.isNotEmpty()) Paths.get(out.lines().first()) else null
        }.getOrNull()
    }

    private fun fromBundle(): Path {
        val resourceName = resourceName()
        val binaryName = if (SystemInfo.isWindows) "askama_fmt.exe" else "askama_fmt"
        val targetDir = Paths.get(System.getProperty("user.home"), ".askama_fmt", "bin")
        val target = targetDir.resolve(binaryName)

        val stream = BinaryManager::class.java.getResourceAsStream("/binaries/$resourceName")
            ?: error("No bundled binary for this platform ($resourceName). Install askama_fmt via `cargo install askama_fmt`.")

        Files.createDirectories(targetDir)
        stream.use { Files.copy(it, target, StandardCopyOption.REPLACE_EXISTING) }

        if (!SystemInfo.isWindows) {
            Files.setPosixFilePermissions(target, PosixFilePermissions.fromString("rwxr-xr-x"))
        }

        return target
    }

    private fun resourceName(): String {
        val os = when {
            SystemInfo.isLinux -> "linux"
            SystemInfo.isMac -> "macos"
            SystemInfo.isWindows -> "windows"
            else -> error("Unsupported OS: ${System.getProperty("os.name")}")
        }
        val arch = when (System.getProperty("os.arch")) {
            "aarch64" -> "aarch64"
            else -> "x86_64"
        }
        val bin = if (SystemInfo.isWindows) "askama_fmt.exe" else "askama_fmt"
        return "$os-$arch/$bin"
    }
}
