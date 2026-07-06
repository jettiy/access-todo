package com.desktoppostit.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Checkbox
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Divider
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.unit.dp
import com.desktoppostit.data.Priority
import com.desktoppostit.data.Todo
import com.desktoppostit.data.TodoRepository
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PostItScreen(repo: TodoRepository) {
    val scope = rememberCoroutineScope()
    val doc by repo.doc.collectAsState()
    var loading by remember { mutableStateOf(false) }
    var showAdd by remember { mutableStateOf(false) }

    LaunchedEffect(Unit) {
        loading = true
        runCatching { repo.refresh() }
        loading = false
    }

    fun sync() {
        scope.launch {
            loading = true
            runCatching { repo.refresh() }
            loading = false
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("📋 내 할 일") },
                actions = {
                    if (loading) {
                        CircularProgressIndicator(
                            modifier = Modifier.size(24.dp),
                            strokeWidth = 2.dp,
                        )
                    } else {
                        IconButton(onClick = { sync() }) {
                            Icon(Icons.Default.Refresh, contentDescription = "새로고침")
                        }
                    }
                    Spacer(Modifier.size(8.dp))
                },
            )
        },
        floatingActionButton = {
            FloatingActionButton(onClick = { showAdd = true }) {
                Icon(Icons.Default.Add, contentDescription = "추가")
            }
        },
    ) { padding ->
        Column(Modifier.padding(padding).fillMaxSize().padding(16.dp)) {
            val today = repo.todayTodos()
            val other = doc.todos.filterNot { it.done || it.dueDate == todayDateString() }
            val done = doc.todos.filter { it.done }

            Section("오늘 (${today.size})", today, repo)
            if (other.isNotEmpty()) {
                Spacer(Modifier.height(8.dp))
                Section("예정 (${other.size})", other, repo)
            }
            if (done.isNotEmpty()) {
                Spacer(Modifier.height(8.dp))
                Section("완료 (${done.size})", done, repo, collapsible = true)
            }
        }

        if (showAdd) {
            AddTodoDialog(
                onDismiss = { showAdd = false },
                onAdd = { title, note, prio ->
                    repo.add(title, note, prio, todayDateString())
                    scope.launch { runCatching { repo.push() } }
                    showAdd = false
                },
            )
        }
    }
}

@Composable
private fun Section(
    title: String,
    todos: List<Todo>,
    repo: TodoRepository,
    collapsible: Boolean = false,
) {
    Text(title, style = MaterialTheme.typography.titleMedium)
    Divider(Modifier.padding(vertical = 4.dp))
    LazyColumn(Modifier.fillMaxWidth()) {
        items(todos, key = { it.id }) { t ->
            TodoRow(t, repo)
        }
    }
}

@Composable
private fun TodoRow(t: Todo, repo: TodoRepository) {
    val scope = rememberCoroutineScope()
    Card(
        modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp),
        colors = CardDefaults.cardColors(
            containerColor = if (t.done) Color(0xFFE8E8E8) else priorityColor(t.priority),
        ),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Checkbox(checked = t.done, onCheckedChange = {
                repo.toggle(t.id)
                scope.launch { runCatching { repo.push() } }
            })
            Spacer(Modifier.size(8.dp))
            Column(Modifier.weight(1f)) {
                Text(
                    text = t.title,
                    style = MaterialTheme.typography.bodyLarge,
                    textDecoration = if (t.done) TextDecoration.LineThrough else null,
                )
                if (!t.note.isNullOrBlank()) {
                    Text("📝 ${t.note}", style = MaterialTheme.typography.bodySmall)
                }
                if (t.done && t.completedBy != null && t.completedBy != repo.agentName) {
                    Text("🤖 ${t.completedBy}", style = MaterialTheme.typography.labelSmall)
                }
            }
            IconButton(onClick = {
                repo.delete(t.id)
                scope.launch { runCatching { repo.push() } }
            }) {
                Icon(Icons.Default.Delete, contentDescription = "삭제")
            }
        }
    }
}

private fun priorityColor(p: Priority): Color = when (p) {
    Priority.high -> Color(0xFFFFE0E0)
    Priority.medium -> Color(0xFFFFF4D6)
    Priority.low -> Color(0xFFE0F5E0)
}

private fun todayDateString(): String =
    java.time.LocalDate.now().format(java.time.format.DateTimeFormatter.ISO_DATE)
