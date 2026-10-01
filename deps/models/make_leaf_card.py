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

import bpy, sys

dst = sys.argv[sys.argv.index("--") + 1]

TEXELS = 128
# Leaves along each side of the texture, and a leaf's radius in cells.
LEAVES = 4
RADIUS = 0.4
LEAF = (0.25, 0.64, 0.30)
# The color between the leaves, seen only when nothing cuts it out.
GAP = (0.08, 0.25, 0.11)
# Not the glTF default of 0.5, so a loader that drops it is caught.
CUTOFF = 0.3
HALF_WIDTH = 1.0
HEIGHT = 2.0


def leaves_image():
    image = bpy.data.images.new("Leaves", TEXELS, TEXELS, alpha=True)
    cell = TEXELS / LEAVES
    pixels = []
    for y in range(TEXELS):
        for x in range(TEXELS):
            dx = (x % cell) - cell / 2.0 + 0.5
            dy = (y % cell) - cell / 2.0 + 0.5
            inside = dx * dx + dy * dy < (cell * RADIUS) ** 2
            pixels.extend((*LEAF, 1.0) if inside else (*GAP, 0.0))
    image.pixels = pixels
    image.pack()
    return image


def material(image):
    mat = bpy.data.materials.new("Leaves")
    mat.use_nodes = True
    nodes, links = mat.node_tree.nodes, mat.node_tree.links
    bsdf = nodes["Principled BSDF"]
    bsdf.inputs["Roughness"].default_value = 0.8
    texture = nodes.new("ShaderNodeTexImage")
    texture.image = image
    links.new(texture.outputs["Color"], bsdf.inputs["Base Color"])
    # The shape the exporter reads as a mask: one minus alpha below the
    # cutoff.
    below = nodes.new("ShaderNodeMath")
    below.operation = "LESS_THAN"
    below.inputs[1].default_value = CUTOFF
    links.new(texture.outputs["Alpha"], below.inputs[0])
    keep = nodes.new("ShaderNodeMath")
    keep.operation = "SUBTRACT"
    keep.inputs[0].default_value = 1.0
    links.new(below.outputs["Value"], keep.inputs[1])
    links.new(keep.outputs["Value"], bsdf.inputs["Alpha"])
    return mat


def quad(name, right, mat):
    """An upright quad along `right`, a unit direction on the ground,
    wound counter clockwise seen from the side `right` turns left to."""
    rx, ry = right[0] * HALF_WIDTH, right[1] * HALF_WIDTH
    corners = [(-rx, -ry, 0.0), (rx, ry, 0.0), (rx, ry, HEIGHT), (-rx, -ry, HEIGHT)]
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(corners, [], [(0, 1, 2, 3)])
    uv = mesh.uv_layers.new(name="UVMap")
    for loop, point in zip(uv.data, [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]):
        loop.uv = point
    mesh.materials.append(mat)
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.scene.collection.objects.link(obj)


bpy.ops.wm.read_factory_settings(use_empty=True)

mat = material(leaves_image())
# Blender is z up and its front is minus y, so both quads face minus y.
DIAGONAL = 0.70710678
quad("Card.0", (DIAGONAL, DIAGONAL), mat)
quad("Card.1", (DIAGONAL, -DIAGONAL), mat)

bpy.ops.wm.save_as_mainfile(filepath=dst)
print("SAVED", dst)
