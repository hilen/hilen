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

import math
import sys
from dataclasses import dataclass
from typing import Final

import bpy

SEGMENTS: Final = 8
# Below this an encoded sRGB channel is linear, see the sRGB transfer function.
SRGB_KNEE: Final = 0.04045


@dataclass(frozen=True, slots=True)
class Cone:
    name: str
    radius_bottom: float
    radius_top: float
    height: float
    foot: float
    color: str


TRUNK: Final = Cone("Trunk", 0.38, 0.26, 2.4, 0.0, "#6b4a2b")
# Each tier starts inside the one below so no gap shows.
TIERS: Final = (
    Cone("Tier.0", 2.0, 0.0, 1.7, 1.3, "#2f6b2a"),
    Cone("Tier.1", 1.6, 0.0, 1.6, 2.4, "#3f8a36"),
    Cone("Tier.2", 1.2, 0.0, 1.5, 3.5, "#2f6b2a"),
    Cone("Tier.3", 0.8, 0.0, 1.4, 4.5, "#3f8a36"),
)


def to_linear(channel: float) -> float:
    if channel <= SRGB_KNEE:
        return channel / 12.92
    return math.pow((channel + 0.055) / 1.055, 2.4)


# Blender takes linear color, the hex is encoded sRGB.
def linear(hex_color: str) -> tuple[float, float, float]:
    red, green, blue = (to_linear(int(hex_color[i : i + 2], 16) / 255.0) for i in (1, 3, 5))
    return (red, green, blue)


def present[T](value: T | None, what: str) -> T:
    if value is None:
        msg = f"{what} is missing"
        raise ValueError(msg)
    return value


def principled(tree: bpy.types.ShaderNodeTree) -> bpy.types.ShaderNodeBsdfPrincipled:
    node = tree.nodes["Principled BSDF"]
    if not isinstance(node, bpy.types.ShaderNodeBsdfPrincipled):
        msg = f"Principled BSDF is a {type(node).__name__}"
        raise TypeError(msg)
    return node


def material(name: str, hex_color: str) -> bpy.types.Material:
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    inputs = present(principled(present(mat.node_tree, "node tree")).inputs, "BSDF inputs")
    inputs["Base Color"].default_value[:] = (*linear(hex_color), 1.0)
    inputs["Roughness"].default_value = 0.9
    return mat


def cone(spec: Cone, mat: bpy.types.Material):
    bpy.ops.mesh.primitive_cone_add(
        vertices=SEGMENTS,
        radius1=spec.radius_bottom,
        radius2=spec.radius_top,
        depth=spec.height,
        location=(0.0, 0.0, spec.foot + spec.height / 2.0),
    )
    obj = present(bpy.context.active_object, f"object of cone {spec.name}")
    mesh = obj.data
    if not isinstance(mesh, bpy.types.Mesh):
        msg = f"cone {spec.name} is not a mesh"
        raise TypeError(msg)
    obj.name = spec.name
    mesh.name = spec.name
    mesh.materials.append(mat)


def main():
    dst = sys.argv[sys.argv.index("--") + 1]
    bpy.ops.wm.read_factory_settings(use_empty=True)

    cone(TRUNK, material("Bark", TRUNK.color))

    greens: dict[str, bpy.types.Material] = {}
    for tier in TIERS:
        mat = greens.get(tier.color)
        if mat is None:
            mat = material(f"Needles.{len(greens)}", tier.color)
            greens[tier.color] = mat
        cone(tier, mat)

    bpy.ops.wm.save_as_mainfile(filepath=dst)
    print("SAVED", dst)


if __name__ == "__main__":
    main()
