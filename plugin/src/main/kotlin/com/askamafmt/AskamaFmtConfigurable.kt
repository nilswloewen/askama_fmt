package com.askamafmt

import com.intellij.openapi.options.Configurable
import com.intellij.ui.components.JBLabel
import com.intellij.ui.components.JBTextField
import com.intellij.util.ui.FormBuilder
import javax.swing.JComponent
import javax.swing.JPanel

/**
 * Settings → Tools → Askama Formatter.
 *
 * Only needed when auto-detection cannot find the binary — see [BinaryManager]
 * for the resolution order.
 */
class AskamaFmtConfigurable : Configurable {

    // A plain text field on purpose: `untilBuild` is unbounded, so this has to
    // keep working on IDE versions that do not exist yet. TextFieldWithBrowseButton's
    // listener overloads have already churned once, and a removed overload would
    // be a NoSuchMethodError at runtime rather than a compile error here.
    private val binaryPathField = JBTextField()

    override fun getDisplayName(): String = "Askama Formatter"

    override fun createComponent(): JComponent {
        return FormBuilder.createFormBuilder()
            .addLabeledComponent(JBLabel("askama_fmt path:"), binaryPathField, 1, false)
            .addTooltip("Leave empty to auto-detect: \$PATH, then cargo's install root.")
            .addTooltip("Not installed? Run: cargo install askama_fmt")
            .addComponentFillVertically(JPanel(), 0)
            .panel
    }

    override fun isModified(): Boolean =
        binaryPathField.text.trim() != AskamaFmtSettings.instance.binaryPath

    override fun apply() {
        AskamaFmtSettings.instance.binaryPath = binaryPathField.text.trim()
        // The next format call must re-resolve rather than reuse the old path.
        BinaryManager.invalidate()
    }

    override fun reset() {
        binaryPathField.text = AskamaFmtSettings.instance.binaryPath
    }
}
