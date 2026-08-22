use contour::ContourBuilder;
use noise::{NoiseFn, OpenSimplex, utils::PlaneMapBuilder};

use crate::gm::{
    LossyConvert,
    flat::{Point, Size},
};

#[derive(Debug)]
pub struct TerrainParams {
    pub seed:       u32,
    pub resolution: Size<u32>,
    pub size:       Size,
    pub position:   Point,
    pub threshold:  u8,
    pub skip:       usize,
}

impl Default for TerrainParams {
    fn default() -> Self {
        TerrainParams {
            seed:       0,
            resolution: Size {
                width:  100,
                height: 100,
            },
            size:       Size {
                width:  6.0,
                height: 6.0,
            },
            position:   Point { x: 65.0, y: 8.0 },
            threshold:  124,
            skip:       6,
        }
    }
}

pub struct TerrainData {
    pub pixels:  Vec<u8>,
    pub islands: Vec<Vec<Point>>,
}

pub fn generate_terrain(
    TerrainParams {
        seed,
        resolution,
        size,
        position,
        threshold,
        skip,
    }: TerrainParams,
) -> TerrainData {
    let open_simplex = OpenSimplex::new(seed);

    let half_w = size.width / 2.0;
    let half_h = size.width / 2.0;

    let map = PlaneMapBuilder::new_fn(|point: [f64; 2]| open_simplex.get(point))
        .set_size(resolution.width as usize, resolution.height as usize)
        .set_x_bounds(f64::from(position.x - half_w), f64::from(position.x + half_w))
        .set_y_bounds(f64::from(-position.y - half_h), f64::from(-position.y + half_h))
        .build();

    let (width, height) = map.size();
    let mut pixels: Vec<u8> = Vec::with_capacity(width * height);

    for i in map {
        let val: u8 = ((i * 0.5 + 0.5).clamp(0.0, 1.0) * 255.0).lossy_convert();
        pixels.push(if val > threshold { 0 } else { 255 });
    }

    let islands = extract_shapes(&pixels, resolution, skip);

    TerrainData { pixels, islands }
}

fn extract_shapes(data: &[u8], resolution: Size<u32>, skip: usize) -> Vec<Vec<Point>> {
    let data: Vec<_> = data.iter().map(|val| f32::from(*val)).collect();

    let c = ContourBuilder::new(resolution.width as usize, resolution.height as usize, false);
    let res = c.contours(&data, &[0.5]).unwrap();

    res.first()
        .unwrap()
        .geometry()
        .into_iter()
        .map(|polygon| {
            polygon
                .exterior()
                .into_iter()
                .map(|point| Point {
                    x: point.x,
                    y: point.y,
                })
                .step_by(skip)
                .collect()
        })
        .collect()
}
