package com.desktoppostit.data

import android.content.Context
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.withContext
import java.time.LocalDate
import java.time.format.DateTimeFormatter

/**
 * Single source of truth for the UI. Owns the in-memory todo list and
 * synchronizes with the Gist via [GistClient].
 */
class TodoRepository(context: Context) {

    private val settings = Settings(context.applicationContext)
    private var client: GistClient? = null

    private val _doc = MutableStateFlow(TodoDoc())
    val doc: StateFlow<TodoDoc> = _doc.asStateFlow()

    val isConfigured: Boolean
        get() = settings.githubToken != null && settings.gistId != null

    val agentName: String
        get() = settings.agentName

    fun configure(token: String, gistId: String) {
        settings.githubToken = token
        settings.gistId = gistId
        client = GistClient(token)
    }

    private fun ensureClient(): Pair<GistClient, String> {
        val token = settings.githubToken ?: error("GitHub token not set")
        val gistId = settings.gistId ?: error("Gist id not set")
        val c = client ?: GistClient(token).also { client = it }
        return c to gistId
    }

    /** Pull remote, merge with local, and expose the merged document. */
    suspend fun refresh() = withContext(Dispatchers.IO) {
        val (c, gistId) = ensureClient()
        val remote = c.fetch(gistId)
        val merged = GistClient.mergeDocs(_doc.value, remote)
        _doc.value = merged
    }

    /** Persist the current local document to the Gist. */
    suspend fun push() = withContext(Dispatchers.IO) {
        val (c, gistId) = ensureClient()
        val stamped = _doc.value.copy(
            updatedBy = settings.agentName,
            updatedAt = nowIso(),
        )
        c.push(gistId, stamped)
    }

    fun add(title: String, note: String?, priority: Priority, dueDate: String?) {
        val now = nowIso()
        val id = java.util.UUID.randomUUID().toString()
        val todo = Todo(
            id = id,
            title = title,
            note = note,
            priority = priority,
            dueDate = dueDate,
            createdAt = now,
            createdBy = settings.agentName,
            history = listOf(HistoryEntry("created", now, settings.agentName)),
        )
        _doc.value = _doc.value.copy(todos = _doc.value.todos + todo)
    }

    fun toggle(id: String) {
        val now = nowIso()
        val todos = _doc.value.todos.map { t ->
            if (t.id != id) t else {
                val newDone = !t.done
                val action = if (newDone) "checked" else "unchecked"
                t.copy(
                    done = newDone,
                    completedAt = if (newDone) now else null,
                    completedBy = if (newDone) settings.agentName else null,
                    updatedAt = now,
                    updatedBy = settings.agentName,
                    history = t.history + HistoryEntry(action, now, settings.agentName),
                )
            }
        }
        _doc.value = _doc.value.copy(todos = todos)
    }

    fun delete(id: String) {
        _doc.value = _doc.value.copy(todos = _doc.value.todos.filterNot { it.id == id })
    }

    /** Today's todos (local timezone), incomplete only. */
    fun todayTodos(): List<Todo> {
        val today = LocalDate.now().format(DateTimeFormatter.ISO_DATE)
        return _doc.value.todos.filter { it.dueDate == today && !it.done }
    }

    companion object {
        fun nowIso(): String =
            java.time.OffsetDateTime.now().format(DateTimeFormatter.ISO_OFFSET_DATE_TIME)
    }
}
