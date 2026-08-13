# Goyda

**One code to rule them all**
**One code to bind them all**
**One code to make them all run**

Goyda is cross-platform reactive UI framework written in rust

> [!IMPORTANT]
> Goyda is in the very early alpha testing. Not everything is stable

## Why Goyda?

- **No states or signals**: It's reactive, but you don't need to know anything about states. Proc-macro does all the dirty reactive work
- **Looks native**: Instead of Flutter, Goyda calls JNI/WinApi/WASM directly from the rust code. No custom renderers
- **Pure rust layout**: 