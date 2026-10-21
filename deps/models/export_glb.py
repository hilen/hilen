# Exports one Blender file next to itself as a .glb the engine loads,
# see docs/scene.md. Geometry with materials, skins and animations:
#
#   blender --background --python deps/models/export_glb.py -- assets/models/tree.blend
#
# An output node targeted at one render engine is invisible to the
# exporter, which then writes an empty material, so the active output
# is retargeted at all engines. A material with a packed image but no
# image node gets the image linked to the color of the shader its output
# uses, the old engine applied textures itself and those files carry the
# image without a node.

import sys
from collections.abc import Sequence
from pathlib import Path

import bpy


def present[T](value: T | None, what: str) -> T:
    if value is None:
        msg = f"{what} is missing"
        raise ValueError(msg)
    return value


def new_node[T: bpy.types.Node](tree: bpy.types.ShaderNodeTree, kind: type[T]) -> T:
    node = tree.nodes.new(kind.__name__)
    if not isinstance(node, kind):
        msg = f"{kind.__name__} made a {type(node).__name__}"
        raise TypeError(msg)
    return node


def active_output(tree: bpy.types.ShaderNodeTree) -> bpy.types.ShaderNodeOutputMaterial | None:
    for node in tree.nodes:
        if isinstance(node, bpy.types.ShaderNodeOutputMaterial) and node.is_active_output:
            return node
    return None


def link_packed_image(
    tree: bpy.types.ShaderNodeTree, surface: bpy.types.NodeSocket, image: bpy.types.Image
):
    if not surface.is_linked or any(
        isinstance(node, bpy.types.ShaderNodeTexImage) for node in tree.nodes
    ):
        return
    shader = present(present(surface.links, "surface links")[0].from_node, "shader")
    color = shader.inputs.get("Base Color") or shader.inputs.get("Color")
    if color is None:
        return
    texture = new_node(tree, bpy.types.ShaderNodeTexImage)
    texture.image = image
    tree.links.new(present(texture.outputs, "texture outputs")["Color"], color)


def fix_material(material: bpy.types.Material, packed: Sequence[bpy.types.Image]):
    tree = material.node_tree
    if not material.use_nodes or tree is None:
        return
    output = active_output(tree)
    if output is None:
        return
    output.target = "ALL"
    if len(packed) == 1:
        link_packed_image(tree, present(output.inputs, "output inputs")["Surface"], packed[0])


def main():
    src = Path(sys.argv[sys.argv.index("--") + 1])
    dst = src.with_suffix(".glb")
    bpy.ops.wm.open_mainfile(filepath=str(src))

    packed = tuple(image for image in bpy.data.images if image.packed_file is not None)
    for material in bpy.data.materials:
        fix_material(material, packed)

    bpy.ops.export_scene.gltf(
        filepath=str(dst),
        export_format="GLB",
        export_apply=True,
        export_animations=True,
        export_skins=True,
        export_morph=False,
        export_cameras=False,
        export_lights=False,
        export_yup=True,
        export_texcoords=True,
        export_normals=True,
        export_tangents=False,
        export_materials="EXPORT",
        # A png texture is megabytes in the repo, a jpeg a fraction of that.
        export_image_format="JPEG",
        export_jpeg_quality=85,
        use_selection=False,
    )
    print("EXPORTED", dst, dst.stat().st_size)


if __name__ == "__main__":
    main()
