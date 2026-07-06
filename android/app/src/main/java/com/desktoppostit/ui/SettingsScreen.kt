package com.desktoppostit.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import com.desktoppostit.data.TodoRepository

/** First-run configuration: GitHub token + Gist id. Shown until [TodoRepository.isConfigured]. */
@Composable
fun SettingsScreen(repo: TodoRepository) {
    var token by remember { mutableStateOf("") }
    var gistId by remember { mutableStateOf("") }
    var agent by remember { mutableStateOf(repo.agentName) }

    Column(
        modifier = Modifier.fillMaxSize().padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text("PostIt Todo 설정", style = MaterialTheme.typography.headlineSmall)
        Text(
            "데스크톱과 동기화하려면 GitHub Personal Access Token (gist 권한)과 Gist id가 필요해요.",
            style = MaterialTheme.typography.bodyMedium,
        )
        OutlinedTextField(
            value = token,
            onValueChange = { token = it },
            label = { Text("GitHub Token (ghp_...)") },
            visualTransformation = PasswordVisualTransformation(),
            singleLine = true,
            modifier = Modifier.fillMaxWidth(),
        )
        OutlinedTextField(
            value = gistId,
            onValueChange = { gistId = it },
            label = { Text("Gist ID") },
            singleLine = true,
            modifier = Modifier.fillMaxWidth(),
        )
        OutlinedTextField(
            value = agent,
            onValueChange = { agent = it },
            label = { Text("내 에이전트 이름") },
            singleLine = true,
            modifier = Modifier.fillMaxWidth(),
        )
        Spacer(Modifier.height(8.dp))
        Button(
            onClick = { repo.configure(token.trim(), gistId.trim()) },
            enabled = token.isNotBlank() && gistId.isNotBlank(),
            modifier = Modifier.fillMaxWidth(),
        ) {
            Text("저장하고 시작")
        }
    }
}
