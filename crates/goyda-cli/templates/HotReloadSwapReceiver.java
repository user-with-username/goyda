package {package_name};

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.util.Log;

// `goy run android`'s quick reload (`r`) broadcasts `<package>.HOT_SWAP`
// with a `path` extra (the absolute path of a freshly rebuilt generation of
// the consumer's `.so`, already pushed into this app's own private storage
// via `adb push` + `run-as` - see `goyda-cli`'s android target) once the
// app is already running. `System.load(path)` pulls that generation into
// this *already-running* process - no reinstall, no restart - and
// `nativeHotSwap()` re-renders the current page with whatever
// `#[page(...)]`s it just registered.
//
// Declared in AndroidManifest.xml (`android:exported="{debuggable}"` -
// `true` only for a non-`--release` build) rather than registered
// dynamically via `Context.registerReceiver` - the flagged 3-arg overload
// that requires API 33's `RECEIVER_NOT_EXPORTED` isn't declared by this
// toolchain's (much older stub) `android.jar`; a manifest `exported`
// attribute has no such compile-time API dependency. Has to be exported at
// all (unlike a typical app-internal receiver) because `adb shell am
// broadcast`, even targeted at this exact component, is sent from the
// `shell` uid - which a non-exported receiver otherwise rejects outright
// (confirmed the hard way: `dumpsys activity broadcasts` showed
// "Permission Denial: ... is not exported" before this was flipped).
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
