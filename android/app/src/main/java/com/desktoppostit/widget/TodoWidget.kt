package com.desktoppostit.widget

import android.content.Context
import androidx.compose.ui.unit.dp
import androidx.glance.GlanceId
import androidx.glance.GlanceModifier
import androidx.glance.appwidget.GlanceAppWidget
import androidx.glance.appwidget.GlanceAppWidgetReceiver
import androidx.glance.appwidget.cornerRadius
import androidx.glance.appwidget.lazy.LazyColumn
import androidx.glance.appwidget.lazy.items
import androidx.glance.appwidget.provideContent
import androidx.glance.background
import androidx.glance.layout.Column
import androidx.glance.layout.fillMaxSize
import androidx.glance.layout.padding
import androidx.glance.text.Text
import androidx.glance.text.TextStyle
import androidx.glance.text.FontWeight
import androidx.glance.unit.ColorProvider
import com.desktoppostit.data.TodoRepository

/** Home-screen widget: shows today's todo count + first few items. */
class TodoWidget : GlanceAppWidget() {
    override suspend fun provideGlance(context: Context, id: GlanceId) {
        val repo = TodoRepository(context)
        if (repo.isConfigured) {
            runCatching { repo.refresh() }
        }
        val today = repo.todayTodos()

        provideContent {
            Column(
                modifier = GlanceModifier
                    .fillMaxSize()
                    .background(ColorProvider(android.graphics.Color.argb(240, 255, 240, 130)))
                    .cornerRadius(8.dp)
                    .padding(8.dp),
                horizontalAlignment = androidx.glance.layout.Alignment.Start,
            ) {
                Text(
                    "오늘의 할 일 (${today.size})",
                    style = TextStyle(fontWeight = FontWeight.Bold),
                )
                LazyColumn {
                    items(today.take(5)) { t ->
                        Text(
                            if (t.done) "☑ ${t.title}" else "☐ ${t.title}",
                            modifier = GlanceModifier.padding(4.dp),
                        )
                    }
                }
            }
        }
    }
}

class TodoWidgetReceiver : GlanceAppWidgetReceiver() {
    override val glanceAppWidget = TodoWidget()
}
