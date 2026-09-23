use std::mem::take;

/// Indices are 16 bit, so a mesh holds at most this many vertices and a
/// bigger primitive is split.
pub(crate) const MAX_VERTICES: usize = u16::MAX as usize;

/// Splits a primitive of `count` vertices into meshes of at most
/// `MAX_VERTICES` vertices, whole triangles only, so every part draws
/// with 16 bit indices. Each part is the source index of its vertices
/// and its own indices.
pub(crate) fn split_u16(count: usize, indices: &[usize]) -> Vec<(Vec<usize>, Vec<u16>)> {
    let index = |i: usize| u16::try_from(i).expect("a part holds at most 65535 vertices");

    if count <= MAX_VERTICES {
        return vec![((0..count).collect(), indices.iter().map(|&i| index(i)).collect())];
    }

    let mut parts = vec![];
    let mut remap: Vec<Option<u16>> = vec![None; count];
    let mut part_sources: Vec<usize> = vec![];
    let mut part_indices: Vec<u16> = vec![];

    for triangle in indices.as_chunks::<3>().0 {
        let new = triangle.iter().filter(|&&i| remap[i].is_none()).count();
        if part_sources.len() + new > MAX_VERTICES {
            parts.push((take(&mut part_sources), take(&mut part_indices)));
            remap.fill(None);
        }
        for &i in triangle {
            let slot = *remap[i].get_or_insert_with(|| {
                part_sources.push(i);
                index(part_sources.len() - 1)
            });
            part_indices.push(slot);
        }
    }

    if !part_indices.is_empty() {
        parts.push((part_sources, part_indices));
    }

    parts
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn a_big_primitive_splits_into_parts_of_whole_triangles() {
        let count = MAX_VERTICES + 10;
        // A fan, every triangle shares vertex zero.
        let indices: Vec<usize> = (1..count - 1).flat_map(|i| [0, i, i + 1]).collect();
        let parts = split_u16(count, &indices);
        assert_eq!(parts.len(), 2);
        let total: usize = parts.iter().map(|(_, indices)| indices.len()).sum();
        assert_eq!(total, indices.len());
        for (sources, indices) in &parts {
            assert!(sources.len() <= MAX_VERTICES);
            assert_eq!(indices.len() % 3, 0);
            assert!(indices.iter().all(|&i| usize::from(i) < sources.len()));
        }
        // Vertex zero is in both parts, once each.
        assert!(
            parts
                .iter()
                .all(|(sources, _)| sources.iter().filter(|&&i| i == 0).count() == 1)
        );
        assert!(parts[1].0.contains(&(count - 1)));
    }

    #[test]
    fn a_small_primitive_is_one_part() {
        let parts = split_u16(3, &[0, 1, 2]);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].0, vec![0, 1, 2]);
        assert_eq!(parts[0].1, vec![0, 1, 2]);
    }
}
