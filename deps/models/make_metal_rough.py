# Builds the metal and rough fixture from scratch and saves it as the
# Blender file given after `--`, then `export_glb.py` exports it:
#
#   blender --background --python deps/models/make_metal_rough.py -- assets/models/metal_rough.blend
#   blender --background --python deps/models/export_glb.py -- assets/models/metal_rough.blend
#
# One upright quad, twice as wide as tall, facing the front, the engine
# culls back faces. Its material has a metallic and a roughness factor of
# 1 and takes both from one texture, glTF reads metallic from blue and
# roughness from green, and its occlusion from red of the same texture.
# The left half is a smooth metal, the right half a rough dielectric, the
# lower half is occluded and the upper half is not. A loader that drops
# the texture draws the whole quad as a rough metal. The origin is at
# the middle of the quad, nothing else is in the file.

import sys
from typing import Final

import bpy

TEXELS: Final = 64
HALF_WIDTH: Final = 2.0
HALF_HEIGHT: Final = 1.0
# Linear light, what the exporter writes as the base color factor.
BASE: Final = (0.9, 0.35, 0.08, 1.0)
METAL_ROUGHNESS: Final = 0.15
OCCLUDED: Final = 0.25


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


def principled(tree: bpy.types.ShaderNodeTree) -> bpy.types.ShaderNodeBsdfPrincipled:
    node = tree.nodes["Principled BSDF"]
    if not isinstance(node, bpy.types.ShaderNodeBsdfPrincipled):
        msg = f"Principled BSDF is a {type(node).__name__}"
        raise TypeError(msg)
    return node


# Red is the occlusion, green the roughness, blue the metallic.
def orm_image() -> bpy.types.Image:
    image = bpy.data.images.new("MetalRough", TEXELS, TEXELS, alpha=False)
    # The same as the colorspace name "Non-Color", which the stubs do not know.
    present(image.colorspace_settings, "colorspace").is_data = True
    pixels: list[float] = []
    # Blender rows run from the bottom up, the same way as the uv.
    for y in range(TEXELS):
        for x in range(TEXELS):
            metal = x < TEXELS // 2
            occlusion = OCCLUDED if y < TEXELS // 2 else 1.0
            roughness = METAL_ROUGHNESS if metal else 1.0
            pixels.extend((occlusion, roughness, 1.0 if metal else 0.0, 1.0))
    image.pixels[:] = pixels
    image.pack()
    return image


# The node group the glTF exporter reads the occlusion from. A new socket
# is a float by default.
def occlusion_output() -> bpy.types.ShaderNodeTree:
    group = bpy.data.node_groups.new("glTF Material Output", "ShaderNodeTree")
    if not isinstance(group, bpy.types.ShaderNodeTree):
        msg = f"the occlusion group is a {type(group).__name__}"
        raise TypeError(msg)
    present(group.interface, "group interface").new_socket("Occlusion", in_out="INPUT")
    return group


def material(image: bpy.types.Image) -> bpy.types.Material:
    mat = bpy.data.materials.new("MetalRough")
    mat.use_nodes = True
    tree = present(mat.node_tree, "node tree")
    links = tree.links
    bsdf = present(principled(tree).inputs, "BSDF inputs")
    bsdf["Base Color"].default_value[:] = BASE
    texture = new_node(tree, bpy.types.ShaderNodeTexImage)
    texture.image = image
    separate = new_node(tree, bpy.types.ShaderNodeSeparateColor)
    channels = present(separate.outputs, "separate outputs")
    links.new(
        present(texture.outputs, "texture outputs")["Color"],
        present(separate.inputs, "separate inputs")["Color"],
    )
    links.new(channels["Green"], bsdf["Roughness"])
    links.new(channels["Blue"], bsdf["Metallic"])
    output = new_node(tree, bpy.types.ShaderNodeGroup)
    output.node_tree = occlusion_output()
    links.new(channels["Red"], present(output.inputs, "group inputs")["Occlusion"])
    return mat


def main():
    dst = sys.argv[sys.argv.index("--") + 1]
    bpy.ops.wm.read_factory_settings(use_empty=True)

    mesh = bpy.data.meshes.new("Quad")
    # Blender is z up and its front is minus y, so the quad faces minus y.
    corners = [
        (-HALF_WIDTH, 0.0, -HALF_HEIGHT),
        (HALF_WIDTH, 0.0, -HALF_HEIGHT),
        (HALF_WIDTH, 0.0, HALF_HEIGHT),
        (-HALF_WIDTH, 0.0, HALF_HEIGHT),
    ]
    mesh.from_pydata(corners, [], [(0, 1, 2, 3)])
    uv = mesh.uv_layers.new(name="UVMap")
    for loop, point in zip(uv.data, [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)], strict=True):
        loop.uv = point
    mesh.materials.append(material(orm_image()))
    obj = bpy.data.objects.new("Quad", mesh)
    present(bpy.context.scene, "scene").collection.objects.link(obj)

    bpy.ops.wm.save_as_mainfile(filepath=dst)
    print("SAVED", dst)


if __name__ == "__main__":
    main()
