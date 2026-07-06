package com.desktoppostit

import android.app.Application
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import com.desktoppostit.worker.SyncWorker
import java.util.concurrent.TimeUnit

class PostItApp : Application() {
    override fun onCreate() {
        super.onCreate()
        // Schedule periodic background sync (every 30 minutes).
        val request = PeriodicWorkRequestBuilder<SyncWorker>(30, TimeUnit.MINUTES)
            .build()
        WorkManager.getInstance(this)
            .enqueueUniquePeriodicWork(
                SyncWorker.WORK_NAME,
                ExistingPeriodicWorkPolicy.KEEP,
                request,
            )
    }
}
