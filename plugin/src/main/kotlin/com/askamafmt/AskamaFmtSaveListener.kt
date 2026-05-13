package com.askamafmt

import com.intellij.openapi.editor.Document
import com.intellij.openapi.fileEditor.FileDocumentManager
import com.intellij.openapi.fileEditor.FileDocumentManagerListener

class AskamaFmtSaveListener : FileDocumentManagerListener {

    override fun beforeDocumentSaving(document: Document) {
        if (!AskamaFmtSettings.instance.formatOnSave) return

        val file = FileDocumentManager.getInstance().getFile(document) ?: return
        if (!file.name.endsWith(".askama.html")) return

        val binary = runCatching { BinaryManager.resolve() }.getOrNull() ?: return
        val content = document.text

        val result = runCatching {
            val process = ProcessBuilder(binary.toString(), "--stdin-filepath", file.path)
                .redirectErrorStream(false)
                .start()

            process.outputStream.writer(Charsets.UTF_8).use { it.write(content) }
            val formatted = process.inputStream.reader(Charsets.UTF_8).readText()
            val exitCode = process.waitFor()

            if (exitCode == 0) formatted else null
        }.getOrNull() ?: return

        if (result != content) {
            document.setText(result)
        }
    }
}
