package com.modrinth.theseus.agent.transformers;

import java.util.ListIterator;
import org.objectweb.asm.Opcodes;
import org.objectweb.asm.tree.AbstractInsnNode;
import org.objectweb.asm.tree.ClassNode;
import org.objectweb.asm.tree.MethodInsnNode;
import org.objectweb.asm.tree.MethodNode;
import org.objectweb.asm.tree.VarInsnNode;

/**
 * Patches {@code com.mojang.blaze3d.platform.Window.setTitle(String)} so that a
 * user-configured window title (passed by the launcher as the
 * {@code modrinth.window.title} system property) takes precedence over the
 * default Minecraft-provided title.
 *
 * <p>Blaze3D ships unobfuscated, so the class and method names are stable across
 * Minecraft versions. The transformer inserts three instructions at the method
 * entry which route the argument through
 * {@code com.modrinth.theseus.agent.WindowTitleHelper#resolveTitle}. No branches
 * are introduced, so no new stack map frames are required and
 * {@link org.objectweb.asm.ClassWriter#COMPUTE_MAXS} is sufficient. */
public final class WindowTitleTransformer extends ClassNodeTransformer {
    private static final String SET_TITLE_DESC = "(Ljava/lang/String;)V";

    @Override
    protected boolean transform(ClassNode classNode) {
        MethodNode setTitle = null;
        for (final MethodNode method : classNode.methods) {
            if (method.name.equals("setTitle") && method.desc.equals(SET_TITLE_DESC)) {
                setTitle = method;
                break;
            }
        }
        if (setTitle == null) {
            return false;
        }

        // At entry: local 0 = this, local 1 = title argument. We replace local 1
        // with the helper's resolved value. Inserting at the very start means the
        // original body sees the (possibly overridden) title.
        final ListIterator<AbstractInsnNode> it = setTitle.instructions.iterator();
        it.add(new VarInsnNode(Opcodes.ALOAD, 1));
        it.add(new MethodInsnNode(
                Opcodes.INVOKESTATIC,
                "com/modrinth/theseus/agent/WindowTitleHelper",
                "resolveTitle",
                "(Ljava/lang/String;)Ljava/lang/String;",
                false));
        it.add(new VarInsnNode(Opcodes.ASTORE, 1));

        return true;
    }
}
