package com.modrinth.theseus.agent;

/**
 * Helper used by {@link WindowTitleTransformer} to resolve the effective window title.
 *
 * <p>If the {@code modrinth.window.title} system property is set and non-empty, it
 * overrides whatever title Minecraft was about to apply. Otherwise the original
 * title is preserved. Keeping the branch logic in a helper method lets the
 * bytecode transformer insert a straight {@code ALOAD/INVOKESTATIC/ASTORE}
 * sequence with no jump frames. */
public final class WindowTitleHelper {
    private WindowTitleHelper() {}

    public static String resolveTitle(String original) {
        final String custom = System.getProperty("modrinth.window.title");
        if (custom != null && !custom.isEmpty()) {
            return custom;
        }
        return original;
    }
}
