package com.desktoppostit.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

private val LightColors = lightColorScheme(
    primary = Color(0xFF7E5A00),
    secondary = Color(0xFFB89B00),
    background = Color(0xFFFFFBF0),
    surface = Color(0xFFFFF4D6),
)

private val DarkColors = darkColorScheme(
    primary = Color(0xFFFFD54F),
    secondary = Color(0xFFFFC107),
)

@Composable
fun PostItTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit,
) {
    MaterialTheme(
        colorScheme = if (darkTheme) DarkColors else LightColors,
        content = content,
    )
}
