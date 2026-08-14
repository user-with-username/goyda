package {package_name};

import android.content.Context;
import android.graphics.Rect;
import android.os.Bundle;
import android.app.Activity;
import android.view.MotionEvent;
import android.view.View;
import android.view.inputmethod.InputMethodManager;
import android.widget.EditText;

public class MainActivity extends Activity {
    private Goyda goyda;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        goyda = new Goyda();
        goyda.start(this);
    }

    @Override
    public void onBackPressed() {
        if (goyda == null || !goyda.nativeBack()) {
            super.onBackPressed();
        }
    }

    @Override
    public boolean dispatchTouchEvent(MotionEvent event) {
        if (event.getAction() == MotionEvent.ACTION_DOWN) {
            View focused = getCurrentFocus();
            if (focused instanceof EditText) {
                Rect bounds = new Rect();
                focused.getGlobalVisibleRect(bounds);
                if (!bounds.contains((int) event.getRawX(), (int) event.getRawY())) {
                    focused.clearFocus();
                    InputMethodManager imm =
                        (InputMethodManager) getSystemService(Context.INPUT_METHOD_SERVICE);
                    if (imm != null) {
                        imm.hideSoftInputFromWindow(focused.getWindowToken(), 0);
                    }
                }
            }
        }
        return super.dispatchTouchEvent(event);
    }
}
