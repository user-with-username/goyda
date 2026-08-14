package {package_name};

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.util.Log;

public class HotReloadSwapReceiver extends BroadcastReceiver {
    @Override
    public void onReceive(Context context, Intent intent) {
        String path = intent.getStringExtra("path");
        if (path == null) {
            return;
        }
        try {
            System.load(path);
            new Goyda().nativeHotSwap();
        } catch (Throwable t) {
            Log.d("goyda", "hot-swap of " + path + " failed: " + t);
        }
    }
}

// kinda bad. but why not
