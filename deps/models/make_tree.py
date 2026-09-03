# Builds the low poly tree fixture from scratch and saves it as the
# Blender file given after `--`, then `export_glb.py` exports it:
#
#   blender --background --python deps/models/make_tree.py -- assets/models/tree.blend
#   blender --background --python deps/models/export_glb.py -- assets/models/tree.blend
#
# A tapered trunk and four cone tiers in two greens, the origin at the
# foot of the trunk, nothing else. The old file carried a 30 unit ground
# plane under the tree, which made the tree's bounds, and with them its
# collider, a slab the size of a yard.

import bpy, sys

dst = sys.argv[sys.argv.index("--") + 1]

SEGMENTS = 8
TRUNK = ("Trunk", 0.38, 0.26, 2.4, "#6b4a2b")
# Name, base radius, height, foot height and color of each tier, bottom
# to top. Each tier starts inside the one below so no gap shows.
TIERS = [
    ("Tier.0", 2.0, 1.7, 1.3, "#2f6b2a"),
    ("Tier.1", 1.6, 1.6, 2.4, "#3f8a36"),
    ("Tier.2", 1.2, 1.5, 3.5, "#2f6b2a"),
    ("Tier.3", 0.8, 1.4, 4.5, "#3f8a36"),
]


def linear(hex_color):
    """Blender takes linear color, the hex is encoded sRGB."""
    channels = [int(hex_color[i : i + 2], 16) / 255.0 for i in (1, 3, 5)]
    return tuple(c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4 for c in channels)


def material(name, hex_color):
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes["Principled BSDF"]
    bsdf.inputs["Base Color"].default_value = (*linear(hex_color), 1.0)
    bsdf.inputs["Roughness"].default_value = 0.9
    return mat


def cone(name, radius_bottom, radius_top, height, foot, mat):
    bpy.ops.mesh.primitive_cone_add(
        vertices=SEGMENTS,
        radius1=radius_bottom,
        radius2=radius_top,
        depth=height,
        location=(0.0, 0.0, foot + height / 2.0),
    )
    obj = bpy.context.active_object
    obj.name = name
    obj.data.name = name
    obj.data.materials.append(mat)
    return obj


bpy.ops.wm.read_factory_settings(use_empty=True)

name, bottom, top, height, color = TRUNK
cone(name, bottom, top, height, 0.0, material("Bark", color))

greens = {}
for name, radius, height, foot, color in TIERS:
    mat = greens.setdefault(color, material(f"Needles.{len(greens)}", color))
    cone(name, radius, 0.0, height, foot, mat)

bpy.ops.wm.save_as_mainfile(filepath=dst)
print("SAVED", dst)
