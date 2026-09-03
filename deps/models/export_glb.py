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

import bpy, os, sys

src = sys.argv[sys.argv.index("--") + 1]
dst = os.path.splitext(src)[0] + ".glb"
bpy.ops.wm.open_mainfile(filepath=src)

packed = [image for image in bpy.data.images if image.packed_file is not None]
for material in bpy.data.materials:
    if not material.use_nodes:
        continue
    nodes = material.node_tree.nodes
    output = next((node for node in nodes if node.type == "OUTPUT_MATERIAL" and node.is_active_output), None)
    if output is None:
        continue
    output.target = "ALL"
    surface = output.inputs["Surface"]
    if len(packed) != 1 or not surface.is_linked or any(node.type == "TEX_IMAGE" for node in nodes):
        continue
    shader = surface.links[0].from_node
    color = shader.inputs.get("Base Color") or shader.inputs.get("Color")
    if color is None:
        continue
    texture = nodes.new("ShaderNodeTexImage")
    texture.image = packed[0]
    material.node_tree.links.new(texture.outputs["Color"], color)

bpy.ops.export_scene.gltf(
    filepath=dst,
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
print("EXPORTED", dst, os.path.getsize(dst))
