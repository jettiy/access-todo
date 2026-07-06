package com.desktoppostit.worker

import android.content.Context
import androidx.work.CoroutineWorker
import androidx.work.WorkerParameters
import com.desktoppostit.data.TodoRepository

/**
 * Periodic background sync. Pulls the latest from Gist every ~30 minutes
 * so the home widget and the app stay fresh even when not in the foreground.
 */
class SyncWorker(appContext: Context, params: WorkerParameters) :
    CoroutineWorker(appContext, params) {

    override suspend fun doWork(): Result {
        val repo = TodoRepository(applicationContext)
        if (!repo.isConfigured) return Result.success()
        return try {
            repo.refresh()
            Result.success()
        } catch (e: Exception) {
            Result.retry()
        }
    }

    companion object {
        const val WORK_NAME = "postit-sync"
    }
}
