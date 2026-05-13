package com.askamafmt

import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.actionSystem.ToggleAction

class AskamaFmtToggleSaveAction : ToggleAction() {
    override fun isSelected(e: AnActionEvent): Boolean = AskamaFmtSettings.instance.formatOnSave
    override fun setSelected(e: AnActionEvent, state: Boolean) {
        AskamaFmtSettings.instance.formatOnSave = state
    }
}
