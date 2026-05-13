package com.askamafmt

import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.actionSystem.CommonDataKeys
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.command.WriteCommandAction
import com.intellij.openapi.ui.Messages

class AskamaFmtAction : AnAction() {

    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project ?: return
        val editor = e.getData(CommonDataKeys.EDITOR) ?: return
        val file = e.getData(CommonDataKeys.VIRTUAL_FILE) ?: return

        val binary = try {
            BinaryManager.resolve()
        } catch (ex: Exception) {
            Messages.showErrorDialog(project, ex.message ?: "Failed to locate askama_fmt binary.", "Askama Formatter")
            return
        }

        val content = editor.document.text
        val filePath = file.path

        ApplicationManager.getApplication().executeOnPooledThread {
            val result = runCatching {
                val process = ProcessBuilder(binary.toString(), "--stdin-filepath", filePath)
                    .redirectErrorStream(false)
                    .start()

                process.outputStream.writer(Charsets.UTF_8).use { it.write(content) }
                val formatted = process.inputStream.reader(Charsets.UTF_8).readText()
                val exitCode = process.waitFor()

                if (exitCode != 0) {
                    val stderr = process.errorStream.reader(Charsets.UTF_8).readText()
                    error("askama_fmt exited $exitCode: $stderr")
                }
                formatted
            }

            result.fold(
                onSuccess = { formatted ->
                    if (formatted != content) {
                        ApplicationManager.getApplication().invokeLater {
                            WriteCommandAction.runWriteCommandAction(project, "Format with askama_fmt", null, {
                                editor.document.setText(formatted)
                            })
                        }
                    }
                },
                onFailure = { ex ->
                    ApplicationManager.getApplication().invokeLater {
                        Messages.showErrorDialog(project, ex.message ?: "Unknown error", "Askama Formatter")
                    }
                }
            )
        }
    }

    override fun update(e: AnActionEvent) {
        val name = e.getData(CommonDataKeys.VIRTUAL_FILE)?.name ?: ""
        e.presentation.isEnabledAndVisible = name.endsWith(".askama.html") || name.endsWith(".html")
    }
}
