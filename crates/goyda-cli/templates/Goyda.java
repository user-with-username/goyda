package {package_name};

import android.view.ViewGroup;
import android.app.Activity;
import android.widget.FrameLayout;

public class Goyda {
    static {
        // "goyda" first, explicitly - its `JNI_OnLoad` (which registers
        // every `native*` method below) only actually runs for whichever
        // library is the *direct* target of a `System.loadLibrary`/
        // `System.load` call, not for a library merely pulled in as a
        // `DT_NEEDED` dependency of another one. "{lib_name}" (the
        // consumer's own library, dynamically linked against "goyda" - see
        // `goyda-cli`'s android target) reuses this already-loaded
        // instance rather than loading a second copy.
        System.loadLibrary("goyda");
        System.loadLibrary("{lib_name}");
    }

    public native void nativeInit(ViewGroup root);
    public native boolean nativeBack();
    public native void nativeHotSwap();

    public void start(Activity activity) {
        FrameLayout layout = new FrameLayout(activity);
        activity.setContentView(layout);
        nativeInit(layout);
    }
}
