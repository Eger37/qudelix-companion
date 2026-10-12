package com.qudelixwidget.app;

import android.app.job.JobInfo;
import android.app.job.JobParameters;
import android.app.job.JobScheduler;
import android.app.job.JobService;
import android.content.ComponentName;
import android.content.Context;

/** Periodic (~15 min, network required) self-update check. */
public class UpdateJob extends JobService {
    private static final int JOB_ID = 42;

    /** Idempotent: keeps an existing schedule. */
    public static void schedule(Context c) {
        JobScheduler js = c.getSystemService(JobScheduler.class);
        if (js == null || js.getPendingJob(JOB_ID) != null) return;
        js.schedule(new JobInfo.Builder(JOB_ID, new ComponentName(c, UpdateJob.class))
                .setRequiredNetworkType(JobInfo.NETWORK_TYPE_ANY)
                .setPeriodic(15 * 60 * 1000L)
                .setPersisted(true)
                .build());
    }

    @Override
    public boolean onStartJob(JobParameters params) {
        new Thread(() -> {
            Updater.checkAndInstall(this, false);
            jobFinished(params, false);
        }, "qudelix-update").start();
        return true;
    }

    @Override
    public boolean onStopJob(JobParameters params) {
        return true; // retry later
    }
}
