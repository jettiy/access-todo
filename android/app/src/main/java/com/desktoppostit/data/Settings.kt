package com.desktoppostit.data

import android.content.Context
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey

/**
 * Encrypted storage for the GitHub token and the gist id. Uses
 * EncryptedSharedPreferences (Android Keystore-backed).
 */
class Settings(context: Context) {
    private val prefs = run {
        val masterKey = MasterKey.Builder(context)
            .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
            .build()
        EncryptedSharedPreferences.create(
            context,
            FILE_NAME,
            masterKey,
            EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
            EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM,
        )
    }

    var githubToken: String?
        get() = prefs.getString(KEY_TOKEN, null)
        set(value) = prefs.edit().putString(KEY_TOKEN, value).apply()

    var gistId: String?
        get() = prefs.getString(KEY_GIST, null)
        set(value) = prefs.edit().putString(KEY_GIST, value).apply()

    var agentName: String
        get() = prefs.getString(KEY_AGENT, "android-user") ?: "android-user"
        set(value) = prefs.edit().putString(KEY_AGENT, value).apply()

    companion object {
        private const val FILE_NAME = "postit_secrets"
        private const val KEY_TOKEN = "github_token"
        private const val KEY_GIST = "gist_id"
        private const val KEY_AGENT = "agent_name"
    }
}
