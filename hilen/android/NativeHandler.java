package io.hilen;

import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;

/**
 * Lets Rust implement a Java interface through java.lang.reflect.Proxy.
 * Android cannot define a class from JNI, so this one class ships as a dex
 * inside the engine and is loaded at run time. Rust registers the native
 * method after the load.
 */
public final class NativeHandler implements InvocationHandler {
    @Override
    public Object invoke(Object proxy, Method method, Object[] args) {
        switch (method.getName()) {
            case "hashCode":
                return System.identityHashCode(proxy);
            case "equals":
                return proxy == args[0];
            case "toString":
                return "io.hilen.NativeHandler";
            default:
                call(method.getName(), args);
                return null;
        }
    }

    private native void call(String method, Object[] args);
}
