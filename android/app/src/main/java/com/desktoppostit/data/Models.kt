package com.desktoppostit.data

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement

/**
 * Mirrors `shared/schema.json`. The same contract used by the Rust core,
 * Tauri desktop widget, and the agents (MCP/REST).
 */
@Serializable
data class TodoDoc(
    val version: String = "1.0",
    @SerialName("updated_at") val updatedAt: String? = null,
    @SerialName("updated_by") val updatedBy: String? = null,
    val todos: List<Todo> = emptyList(),
)

@Serializable
data class Todo(
    val id: String,
    val title: String,
    val note: String? = null,
    val done: Boolean = false,
    val priority: Priority = Priority.medium,
    @SerialName("due_date") val dueDate: String? = null,
    val tags: List<String> = emptyList(),
    @SerialName("created_at") val createdAt: String? = null,
    @SerialName("created_by") val createdBy: String? = null,
    @SerialName("completed_at") val completedAt: String? = null,
    @SerialName("completed_by") val completedBy: String? = null,
    @SerialName("updated_at") val updatedAt: String? = null,
    @SerialName("updated_by") val updatedBy: String? = null,
    val history: List<HistoryEntry> = emptyList(),
)

@Serializable
enum class Priority { high, medium, low }

@Serializable
data class HistoryEntry(
    val action: String,
    val at: String,
    val by: String,
)

/** Payload used when creating a Gist (POST /gists) and editing it (PATCH). */
@Serializable
data class GistFileContent(val content: String)

@Serializable
data class GistPayload(
    val description: String? = null,
    val public: Boolean = false,
    val files: Map<String, GistFileContent>,
)

@Serializable
data class GistResponse(
    val id: String,
    val files: Map<String, GistFileResponse> = emptyMap(),
)

@Serializable
data class GistFileResponse(val content: String = "")

/** Optional error body wrapper used for diagnostics. */
@Serializable
data class GistError(val message: String? = null, val documentation_url: String? = null)

typealias RawJson = JsonElement
