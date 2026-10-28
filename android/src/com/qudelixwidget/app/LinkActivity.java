package com.qudelixwidget.app;

import android.annotation.SuppressLint;
import android.app.Activity;
import android.bluetooth.BluetoothDevice;
import android.companion.AssociationInfo;
import android.companion.AssociationRequest;
import android.companion.BluetoothDeviceFilter;
import android.companion.CompanionDeviceManager;
import android.content.Intent;
import android.content.IntentSender;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;

/**
 * Widget's Link / Connect / Disconnect while the app has no Companion Device Manager
 * association with the 5K: asks for it once (Android's own "allow Qudelix Widget to
 * manage Qudelix-5K?" dialog), then hands the action to RfcommService. The association
 * is what lets the app drop / bring back every link at once (PhoneLink.allProfiles).
 * If it's refused or fails, the action still runs with the per-profile fallback.
 * Invisible apart from the dialog.
 */
@SuppressLint("MissingPermission") // checked before anything Bluetooth
public class LinkActivity extends Activity {

    static final String EXTRA_MODE = "mode";
    private static final int REQ_ASSOCIATE = 1;
    private int mode;

    @Override
    protected void onCreate(Bundle b) {
        super.onCreate(b);
        mode = getIntent().getIntExtra(EXTRA_MODE, RfcommService.LINK);
        if (!RfcommService.hasBtPermission(this)) {
            QudelixWidget.pushStatus(this, "open app → grant Bluetooth");
            finish();
            return;
        }
        BluetoothDevice dev = PhoneLink.target(this);
        if (dev == null) {
            QudelixWidget.pushStatus(this, "5K not paired / BT off");
            finish();
            return;
        }
        if (!PhoneLink.cdmAvailable(this) || PhoneLink.associated(this, dev)) {
            go();
            return;
        }
        Dbg.log(this, "asking for a CDM association with " + dev.getAddress());
        AssociationRequest req = new AssociationRequest.Builder()
                .addDeviceFilter(new BluetoothDeviceFilter.Builder().setAddress(dev.getAddress()).build())
                .setSingleDevice(true)
                .build();
        CompanionDeviceManager cdm = getSystemService(CompanionDeviceManager.class);
        CompanionDeviceManager.Callback cb = new CompanionDeviceManager.Callback() {
            @Override public void onAssociationPending(IntentSender chooser) { show(chooser); }
            @SuppressWarnings("deprecation")
            @Override public void onDeviceFound(IntentSender chooser) { show(chooser); } // before Android 13
            @Override public void onAssociationCreated(AssociationInfo info) {
                Dbg.log(LinkActivity.this, "CDM association created");
            }
            @Override public void onFailure(CharSequence error) {
                Dbg.log(LinkActivity.this, "CDM association failed: " + error);
                go();
            }
        };
        try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                cdm.associate(req, getMainExecutor(), cb);
            } else {
                cdm.associate(req, cb, new Handler(Looper.getMainLooper()));
            }
        } catch (Exception e) {
            Dbg.log(this, "CDM associate: " + e);
            go();
        }
    }

    private void show(IntentSender chooser) {
        try {
            startIntentSenderForResult(chooser, REQ_ASSOCIATE, null, 0, 0, 0);
        } catch (IntentSender.SendIntentException e) {
            Dbg.log(this, "CDM dialog: " + e);
            go();
        }
    }

    @Override
    protected void onActivityResult(int req, int result, Intent data) {
        super.onActivityResult(req, result, data);
        if (req != REQ_ASSOCIATE) return;
        Dbg.log(this, "CDM dialog → " + (result == RESULT_OK ? "allowed" : "declined"));
        go();
    }

    /** Run the action (with or without the association) and get out of the way. */
    private void go() {
        if (isFinishing()) return;
        if (!RfcommService.start(this, RfcommService.ACTION_PHONE_AUDIO, mode)) {
            QudelixWidget.pushStatus(this, "start failed");
        }
        QudelixWidget.update(this); // the buttons skip this activity from now on
        finish();
    }
}
