package {package_name};

import android.view.ViewGroup;
import android.app.Activity;
import android.widget.FrameLayout;

public class Goyda {
    static {
        System.loadLibrary("{lib_name}");
    }

    public native void nativeInit(ViewGroup root);

    public void start(Activity activity) {
        FrameLayout layout = new FrameLayout(activity);
        activity.setContentView(layout);
        nativeInit(layout);
    }
}
