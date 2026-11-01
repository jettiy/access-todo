package com.desktoppostit

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.ui.Modifier
import androidx.lifecycle.lifecycleScope
import com.desktoppostit.data.PilotNotifier
import com.desktoppostit.data.TodoRepository
import com.desktoppostit.ui.PostItScreen
import com.desktoppostit.ui.SettingsScreen
import com.desktoppostit.ui.theme.PostItTheme
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val repo = TodoRepository(this)

        // P2: 파일럿 요청 알림 권한 (Android 13+)
        if (Build.VERSION.SDK_INT >= 33 &&
            checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) !=
            PackageManager.PERMISSION_GRANTED
        ) {
            requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), 1001)
        }
        // 포그라운드 동기화에서도 새 파일럿 요청을 알림
        lifecycleScope.launch {
            repo.doc.collect { PilotNotifier.notifyNewPilotItems(this@MainActivity, it.todos) }
        }

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
