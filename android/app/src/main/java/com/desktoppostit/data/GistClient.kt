package com.desktoppostit.data

import kotlinx.serialization.json.Json
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import java.io.IOException
import java.util.concurrent.TimeUnit

/**
 * Direct GitHub Gist client. The Android app talks to Gist directly (not
 * the core service) so it works on mobile data, away from the desktop.
 *
 * Concurrency: uses the ETag from the last fetch for conditional requests
 * and to detect mid-air edits. On push, the local document is written as
 * the new canonical state; a per-id merge is performed in [mergeDocs] to
 * reconcile local edits with a fresher remote.
 */
class GistClient(
    private val token: String,
    private val baseUrl: String = BASE_URL,
) {
    private val http = OkHttpClient.Builder()
        .connectTimeout(15, TimeUnit.SECONDS)
        .readTimeout(20, TimeUnit.SECONDS)
        .build()
    private val json = Json {
        ignoreUnknownKeys = true
        encodeDefaults = true
        prettyPrint = false
    }

    /** Holds the ETag from the latest successful fetch. */
    @Volatile
    var etag: String? = null
        private set

    /** Fetch the gist and parse `todos.json`. Updates [etag]. */
    @Throws(IOException::class)
    fun fetch(gistId: String): TodoDoc {
        val builder = Request.Builder()
            .url("$baseUrl/gists/$gistId")
            .header("Authorization", "Bearer $token")
            .header("Accept", "application/vnd.github+json")
        etag?.let { builder.header("If-None-Match", it) }

        http.newCall(builder.build()).execute().use { resp ->
            if (!resp.isSuccessful) throw IOException("fetch failed: HTTP ${resp.code}")
            resp.header("ETag")?.let { etag = it }
            val body = resp.body?.string().orEmpty()
            val gist = json.decodeFromString(GistResponse.serializer(), body)
            val content = gist.files["todos.json"]?.content
                ?: throw IOException("todos.json missing from gist")
            return json.decodeFromString(TodoDoc.serializer(), content)
        }
    }

    /** Push the document to the gist (PATCH). Updates [etag]. */
    @Throws(IOException::class)
    fun push(gistId: String, doc: TodoDoc) {
        val content = json.encodeToString(TodoDoc.serializer(), doc)
        val payload = json.encodeToString(
            GistPayload.serializer(),
            GistPayload(files = mapOf("todos.json" to GistFileContent(content))),
        )
        val request = Request.Builder()
            .url("$baseUrl/gists/$gistId")
            .header("Authorization", "Bearer $token")
            .header("Accept", "application/vnd.github+json")
            .patch(payload.toRequestBody(JSON_MEDIA))
            .build()

        http.newCall(request).execute().use { resp ->
            if (!resp.isSuccessful) {
                val errBody = resp.body?.string().orEmpty()
                throw IOException("push failed: HTTP ${resp.code}: $errBody")
            }
            resp.header("ETag")?.let { etag = it }
        }
    }

    companion object {
        private const val BASE_URL = "https://api.github.com"
        private val JSON_MEDIA = "application/json; charset=utf-8".toMediaType()

        /**
         * Per-id merge of two documents. Same algorithm as the Rust core:
         * union of disjoint ids; for matching ids the one with the latest
         * `updated_at` (falling back to `created_at`) wins.
         */
        fun mergeDocs(local: TodoDoc, remote: TodoDoc): TodoDoc {
            val map = LinkedHashMap<String, Todo>()
            for (t in local.todos) map[t.id] = t
            for (t in remote.todos) {
                val existing = map[t.id]
                if (existing == null || ts(t) >= ts(existing)) {
                    map[t.id] = t
                }
            }
            val sorted = map.values.sortedBy { it.id }
            return remote.copy(todos = sorted)
        }

        private fun ts(t: Todo): String =
            (t.updatedAt ?: t.createdAt) ?: ""
    }
}
