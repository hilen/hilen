# Builds the leaf card fixture from scratch and saves it as the Blender
# file given after `--`, then `export_glb.py` exports it:
#
#   blender --background --python deps/models/make_leaf_card.py -- assets/models/leaf_card.blend
#   blender --background --python deps/models/export_glb.py -- assets/models/leaf_card.blend
#
# Two upright quads crossed at a right angle, the way a bush is built,
# both facing the front half, the engine culls back faces. They carry one
# texture of round leaves whose alpha is zero between the leaves, and a
# material the exporter writes as `alphaMode` `MASK` with the cutoff
# below. The origin is at the foot, nothing else is in the file.

import sys
from typing import Final

import bpy

TEXELS: Final = 128
# Leaves along each side of the texture, and a leaf's radius in cells.
LEAVES: Final = 4
RADIUS: Final = 0.4
LEAF: Final = (0.25, 0.64, 0.30)
# The color between the leaves, seen only when nothing cuts it out.
GAP: Final = (0.08, 0.25, 0.11)
# Not the glTF default of 0.5, so a loader that drops it is caught.
CUTOFF: Final = 0.3
HALF_WIDTH: Final = 1.0
HEIGHT: Final = 2.0
DIAGONAL: Final = 0.70710678


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


def leaves_image() -> bpy.types.Image:
    image = bpy.data.images.new("Leaves", TEXELS, TEXELS, alpha=True)
    cell = TEXELS / LEAVES
    pixels: list[float] = []
    for y in range(TEXELS):
        for x in range(TEXELS):
            dx = (x % cell) - cell / 2.0 + 0.5
            dy = (y % cell) - cell / 2.0 + 0.5
            inside = dx * dx + dy * dy < (cell * RADIUS) ** 2
            pixels.extend((*LEAF, 1.0) if inside else (*GAP, 0.0))
    image.pixels[:] = pixels
    image.pack()
    return image


def material(image: bpy.types.Image) -> bpy.types.Material:
    mat = bpy.data.materials.new("Leaves")
    mat.use_nodes = True
    tree = present(mat.node_tree, "node tree")
    links = tree.links
    bsdf = present(principled(tree).inputs, "BSDF inputs")
    bsdf["Roughness"].default_value = 0.8
    texture = new_node(tree, bpy.types.ShaderNodeTexImage)
    texture.image = image
    texels = present(texture.outputs, "texture outputs")
    links.new(texels["Color"], bsdf["Base Color"])
    # The shape the exporter reads as a mask: one minus alpha below the
    # cutoff.
    below = new_node(tree, bpy.types.ShaderNodeMath)
    below.operation = "LESS_THAN"
    below_in = present(below.inputs, "math inputs")
    below_in[1].default_value = CUTOFF
    links.new(texels["Alpha"], below_in[0])
    keep = new_node(tree, bpy.types.ShaderNodeMath)
    keep.operation = "SUBTRACT"
    keep_in = present(keep.inputs, "math inputs")
    keep_in[0].default_value = 1.0
    links.new(present(below.outputs, "math outputs")["Value"], keep_in[1])
    links.new(present(keep.outputs, "math outputs")["Value"], bsdf["Alpha"])
    return mat


# An upright quad along `right`, a unit direction on the ground, wound
# counter clockwise seen from the side `right` turns left to.
def quad(name: str, right: tuple[float, float], mat: bpy.types.Material):
    rx, ry = right[0] * HALF_WIDTH, right[1] * HALF_WIDTH
    corners = [(-rx, -ry, 0.0), (rx, ry, 0.0), (rx, ry, HEIGHT), (-rx, -ry, HEIGHT)]
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(corners, [], [(0, 1, 2, 3)])
    uv = mesh.uv_layers.new(name="UVMap")
    for loop, point in zip(uv.data, [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)], strict=True):
        loop.uv = point
    mesh.materials.append(mat)
    obj = bpy.data.objects.new(name, mesh)
    present(bpy.context.scene, "scene").collection.objects.link(obj)


def main():
    dst = sys.argv[sys.argv.index("--") + 1]
    bpy.ops.wm.read_factory_settings(use_empty=True)

    mat = material(leaves_image())
    # Blender is z up and its front is minus y, so both quads face minus y.
    quad("Card.0", (DIAGONAL, DIAGONAL), mat)
    quad("Card.1", (DIAGONAL, -DIAGONAL), mat)

    bpy.ops.wm.save_as_mainfile(filepath=dst)
    print("SAVED", dst)


if __name__ == "__main__":
    main()
