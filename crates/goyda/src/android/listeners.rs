use goyda_macro_rules::define_native_listener;

define_native_listener! {
    mod_name = click,
    class_name = "goyda/internal/GoydaClickListener",
    interface_name = "android/view/View$OnClickListener",
    setter = "setOnClickListener",
    callback = dyn Fn(Event),
    methods = [
        {
            name = "onClick",
            jni_sig = (("android/view/View") -> void),
            native_fn = native_on_click(env, this, _view: JObject<'a>) -> () {
                if let Some(cb) = self::get_callback(&mut env, &this) {
                    cb(Event::Click);
                }
            }
        }
    ]
}

define_native_listener! {
    mod_name = long_click,
    class_name = "goyda/internal/GoydaLongClickListener",
    interface_name = "android/view/View$OnLongClickListener",
    setter = "setOnLongClickListener",
    callback = dyn Fn(Event),
    methods = [
        {
            name = "onLongClick",
            jni_sig = (("android/view/View") -> int),
            native_fn = native_on_long_click(env, this, _view: JObject<'a>) -> jboolean {
                if let Some(cb) = self::get_callback(&mut env, &this) {
                    cb(Event::LongClick);
                }
                JNI_TRUE
            }
        }
    ]
}

define_native_listener! {
    mod_name = checked_change,
    class_name = "goyda/internal/GoydaCheckedChangeListener",
    interface_name = "android/widget/CompoundButton$OnCheckedChangeListener",
    setter = "setOnCheckedChangeListener",
    callback = dyn Fn(Event),
    methods = [
        {
            name = "onCheckedChanged",
            jni_sig = (("android/widget/CompoundButton", int) -> void),
            native_fn = native_on_checked_changed(env, this, _view: JObject<'a>, checked: jboolean) -> () {
                if let Some(cb) = self::get_callback(&mut env, &this) {
                    cb(Event::CheckedChanged(checked != 0));
                }
            }
        }
    ]
}

define_native_listener! {
    mod_name = focus_change,
    class_name = "goyda/internal/GoydaFocusChangeListener",
    interface_name = "android/view/View$OnFocusChangeListener",
    setter = "setOnFocusChangeListener",
    callback = dyn Fn(Event),
    methods = [
        {
            name = "onFocusChange",
            jni_sig = (("android/view/View", int) -> void),
            native_fn = native_on_focus_change(env, this, _view: JObject<'a>, has_focus: jboolean) -> () {
                if let Some(cb) = self::get_callback(&mut env, &this) {
                    cb(Event::FocusChanged(has_focus != 0));
                }
            }
        }
    ]
}

define_native_listener! {
    mod_name = text_watcher,
    class_name = "goyda/internal/GoydaTextWatcher",
    interface_name = "android/text/TextWatcher",
    setter = "addTextChangedListener",
    callback = dyn Fn(Event),
    methods = [
        {
            name = "beforeTextChanged",
            jni_sig = (("java/lang/CharSequence", int, int, int) -> void),
            native_fn = native_before_text_changed(env, _this, _s: JObject<'a>, _start: jint, _count: jint, _after: jint) -> () {}
        },
        {
            name = "onTextChanged",
            jni_sig = (("java/lang/CharSequence", int, int, int) -> void),
            native_fn = native_on_text_changed(env, this, s: JObject<'a>, start: jint, before: jint, count: jint) -> () {
                if let Some(cb) = self::get_callback(&mut env, &this) {
                    let jstr = ::jni::objects::JString::from(s);
                    let text: String = env.get_string(&jstr)
                        .map(|s| s.into())
                        .unwrap_or_default();
                    cb(Event::TextChanged {
                        text,
                        start: start as usize,
                        before: before as usize,
                        count: count as usize,
                    });
                }
            }
        },
        {
            name = "afterTextChanged",
            jni_sig = (("android/text/Editable") -> void),
            native_fn = native_after_text_changed(env, this, _editable: JObject<'a>) -> () {}
        }
    ]
}
