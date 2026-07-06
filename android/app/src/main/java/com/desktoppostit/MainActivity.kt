package com.desktoppostit

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.ui.Modifier
import com.desktoppostit.data.TodoRepository
import com.desktoppostit.ui.PostItScreen
import com.desktoppostit.ui.SettingsScreen
import com.desktoppostit.ui.theme.PostItTheme

class MainActivity : ComponentActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val repo = TodoRepository(this)

        setContent {
            PostItTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background,
                ) {
                    if (repo.isConfigured) {
                        PostItScreen(repo)
                    } else {
                        SettingsScreen(repo)
                    }
                }
            }
        }
    }
}
