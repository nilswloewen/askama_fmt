package com.askamafmt

import com.intellij.openapi.components.PersistentStateComponent
import com.intellij.openapi.components.Service
import com.intellij.openapi.components.State
import com.intellij.openapi.components.Storage
import com.intellij.openapi.components.service

@Service(Service.Level.APP)
@State(name = "AskamaFmtSettings", storages = [Storage("askama_fmt.xml")])
class AskamaFmtSettings : PersistentStateComponent<AskamaFmtSettings.State> {

    data class State(var formatOnSave: Boolean = false)

    private var _state = State()

    override fun getState(): State = _state
    override fun loadState(s: State) { _state = s }

    var formatOnSave: Boolean
        get() = _state.formatOnSave
        set(value) { _state.formatOnSave = value }

    companion object {
        val instance: AskamaFmtSettings get() = service()
    }
}
