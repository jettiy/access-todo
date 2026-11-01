package com.desktoppostit.data

import android.Manifest
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat

/**
 * P2: raises a local notification whenever a new pilot action item
 * (tags: `agent:user` + `pilot:approval|decision|confirm`) appears after a
 * Gist sync — created by agents via the `request_pilot` MCP tool.
 *
 * Seen ids live in plain SharedPreferences (not secrets). On the very first
 * run after install/upgrade the current pilot items are marked seen silently
 * so the user is not flooded with notifications for old requests.
 */
object PilotNotifier {

    private const val CHANNEL_ID = "pilot-actions"
    private const val PREFS = "pilot_notifier"
    private const val KEY_SEEN = "seen_ids"

    private val KIND_LABELS = mapOf(
        "pilot:approval" to "승인 대기",
        "pilot:decision" to "결정 필요",
        "pilot:confirm" to "확인 요망",
    )

    private fun isPilotItem(t: Todo): Boolean =
        !t.done &&
            t.tags.any { it == "agent:user" } &&
            t.tags.any { it.startsWith("pilot:") }

    fun ensureChannel(context: Context) {
        val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        nm.createNotificationChannel(
            NotificationChannel(
                CHANNEL_ID,
                "파일럿 요청",
                NotificationManager.IMPORTANCE_HIGH,
            ).apply { description = "에이전트가 파일럿에게 요청한 작업" },
        )
    }

    private fun canNotify(context: Context): Boolean =
        Build.VERSION.SDK_INT < 33 ||
            ContextCompat.checkSelfPermission(
                context,
                Manifest.permission.POST_NOTIFICATIONS,
            ) == PackageManager.PERMISSION_GRANTED

    /** Notify every unseen pilot todo. Returns the number of notifications posted. */
    fun notifyNewPilotItems(context: Context, todos: List<Todo>): Int {
        val prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        val pilot = todos.filter(::isPilotItem)

        // First run: swallow the backlog so only future requests notify.
        if (!prefs.contains(KEY_SEEN)) {
            prefs.edit().putStringSet(KEY_SEEN, pilot.map { it.id }.toSet()).apply()
            return 0
        }

        val seen = prefs.getStringSet(KEY_SEEN, emptySet())?.toMutableSet() ?: mutableSetOf()
        val fresh = pilot.filter { it.id !in seen }
        if (fresh.isEmpty()) return 0

        var posted = 0
        if (canNotify(context)) {
            ensureChannel(context)
            val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
            for (t in fresh) {
                val kind = t.tags.firstNotNullOfOrNull { KIND_LABELS[it] } ?: "요청"
                val n = NotificationCompat.Builder(context, CHANNEL_ID)
                    .setSmallIcon(com.desktoppostit.R.drawable.ic_postit)
                    .setContentTitle("[$kind] ${t.title}")
                    .setContentText(t.note ?: "")
                    .setStyle(
                        NotificationCompat.BigTextStyle().bigText(
                            "${t.note ?: ""}\n(${t.createdBy ?: "agent"} 요청)",
                        ),
                    )
                    .setAutoCancel(true)
                    .build()
                nm.notify(t.id.hashCode(), n)
                posted++
            }
        }
        // Mark seen even when notifications are disabled — the widget still
        // shows the items and we must not replay them later.
        seen.addAll(fresh.map { it.id })
        prefs.edit().putStringSet(KEY_SEEN, seen).apply()
        return posted
    }
}
