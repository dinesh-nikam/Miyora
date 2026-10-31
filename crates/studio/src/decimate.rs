//! Polygon reduction by quadric-error-metric edge collapse
//! (Garland & Heckbert, "Surface Simplification Using Quadric Error Metrics", 1997).
//!
//! Collapses are rejected when they would flip a face or create non-manifold
//! topology. Open borders (including UV seams, which are split vertices) can be
//! locked so silhouettes and texture seams stay intact. Vertex attributes come
//! from the surviving endpoint, or are averaged when the midpoint wins.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use crate::export::ExportMesh;

/// Minimum cosine between a face normal before and after a collapse.
const FLIP_DOT_MIN: f64 = 0.2;
/// Weight of the virtual planes that keep unlocked borders in place.
const BORDER_PLANE_WEIGHT: f64 = 1_000.0;
pub const MIN_TARGET_TRIANGLES: usize = 4;

#[derive(Debug, Clone, Copy)]
pub struct DecimateOptions {
    pub target_triangles: usize,
    /// Never move vertices on open borders or UV seams.
    pub preserve_borders: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecimateReport {
    pub source_triangles: usize,
    pub result_triangles: usize,
    pub source_vertices: usize,
    pub result_vertices: usize,
}

/// Symmetric 4x4 quadric stored as its 10 unique entries.
#[derive(Debug, Clone, Copy, Default)]
struct Quadric([f64; 10]);

impl Quadric {
    fn plane([a, b, c]: [f64; 3], d: f64, weight: f64) -> Self {
        Self(
            [
                a * a,
                a * b,
                a * c,
                a * d,
                b * b,
                b * c,
                b * d,
                c * c,
                c * d,
                d * d,
            ]
            .map(|value| value * weight),
        )
    }

    fn add(&mut self, other: &Self) {
        for (slot, value) in self.0.iter_mut().zip(other.0) {
            *slot += value;
        }
    }

    fn sum(&self, other: &Self) -> Self {
        let mut out = *self;
        out.add(other);
        out
    }

    fn error(&self, [x, y, z]: [f64; 3]) -> f64 {
        let q = &self.0;
        q[0] * x * x
            + 2.0 * q[1] * x * y
            + 2.0 * q[2] * x * z
            + 2.0 * q[3] * x
            + q[4] * y * y
            + 2.0 * q[5] * y * z
            + 2.0 * q[6] * y
            + q[7] * z * z
            + 2.0 * q[8] * z
            + q[9]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Keep {
    U,
    V,
    Mid,
}

struct Candidate {
    cost: f64,
    u: u32,
    v: u32,
    stamps: (u32, u32),
    target: [f64; 3],
    keep: Keep,
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.cost.total_cmp(&other.cost) == Ordering::Equal
    }
}
impl Eq for Candidate {}
impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Candidate {
    /// Reversed so `BinaryHeap` pops the cheapest collapse first.
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.total_cmp(&self.cost)
    }
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn length(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

fn normalized(a: [f64; 3]) -> Option<[f64; 3]> {
    let len = length(a);
    (len > f64::EPSILON).then(|| a.map(|value| value / len))
}

struct Simplifier {
    positions: Vec<[f64; 3]>,
    uvs: Option<Vec<[f32; 2]>>,
    colors: Option<Vec<[u8; 3]>>,
    faces: Vec<[u32; 3]>,
    face_alive: Vec<bool>,
    vertex_faces: Vec<Vec<u32>>,
    quadrics: Vec<Quadric>,
    locked: Vec<bool>,
    alive: Vec<bool>,
    stamps: Vec<u32>,
    live_faces: usize,
    /// Every undirected edge once, for the initial candidate set.
    edges: Vec<(u32, u32)>,
}

impl Simplifier {
    fn new(mesh: &ExportMesh, preserve_borders: bool) -> Self {
        let count = mesh.positions.len();
        let positions: Vec<[f64; 3]> = mesh.positions.iter().map(|p| p.map(f64::from)).collect();
        let faces: Vec<[u32; 3]> = mesh.indices.as_chunks::<3>().0.to_vec();
        let mut vertex_faces = vec![Vec::new(); count];
        let mut quadrics = vec![Quadric::default(); count];
        let mut edges: HashMap<(u32, u32), (u32, usize)> = HashMap::new();
        for (face_index, face) in faces.iter().enumerate() {
            let [a, b, c] = face.map(|index| positions[index as usize]);
            let area_normal = cross(sub(b, a), sub(c, a));
            if let Some(normal) = normalized(area_normal) {
                let plane = Quadric::plane(normal, -dot(normal, a), length(area_normal) / 2.0);
                for index in face {
                    quadrics[*index as usize].add(&plane);
                }
            }
            for (corner, index) in face.iter().enumerate() {
                vertex_faces[*index as usize].push(face_index as u32);
                let next = face[(corner + 1) % 3];
                let key = ((*index).min(next), (*index).max(next));
                edges.entry(key).or_insert((0, face_index)).0 += 1;
            }
        }
        let mut locked = vec![false; count];
        for (&(u, v), &(uses, face_index)) in &edges {
            if uses != 1 {
                continue;
            }
            if preserve_borders {
                locked[u as usize] = true;
                locked[v as usize] = true;
                continue;
            }
            let [a, b, c] = faces[face_index].map(|index| positions[index as usize]);
            let edge = sub(positions[v as usize], positions[u as usize]);
            let normal = normalized(cross(sub(b, a), sub(c, a)))
                .and_then(|face_normal| normalized(cross(edge, face_normal)));
            if let Some(normal) = normal {
                let p = positions[u as usize];
                let plane = Quadric::plane(
                    normal,
                    -dot(normal, p),
                    BORDER_PLANE_WEIGHT * dot(edge, edge),
                );
                quadrics[u as usize].add(&plane);
                quadrics[v as usize].add(&plane);
            }
        }
        let mut edge_list: Vec<(u32, u32)> = edges.into_keys().collect();
        // Deterministic order for reproducible results.
        edge_list.sort_unstable();
        Self {
            positions,
            uvs: mesh.uvs.clone(),
            colors: mesh.colors.clone(),
            live_faces: faces.len(),
            face_alive: vec![true; faces.len()],
            faces,
            vertex_faces,
            quadrics,
            locked,
            alive: vec![true; count],
            stamps: vec![0; count],
            edges: edge_list,
        }
    }

    fn neighbors(&self, vertex: u32) -> Vec<u32> {
        let mut out: Vec<u32> = self.vertex_faces[vertex as usize]
            .iter()
            .filter(|face| self.face_alive[**face as usize])
            .flat_map(|face| self.faces[*face as usize])
            .filter(|other| *other != vertex)
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    fn candidate(&self, u: u32, v: u32) -> Option<Candidate> {
        let (lu, lv) = (self.locked[u as usize], self.locked[v as usize]);
        if lu && lv {
            return None;
        }
        let quadric = self.quadrics[u as usize].sum(&self.quadrics[v as usize]);
        let (pu, pv) = (self.positions[u as usize], self.positions[v as usize]);
        let mid = [
            (pu[0] + pv[0]) / 2.0,
            (pu[1] + pv[1]) / 2.0,
            (pu[2] + pv[2]) / 2.0,
        ];
        let options: &[(Keep, [f64; 3])] = if lu {
            &[(Keep::U, pu)]
        } else if lv {
            &[(Keep::V, pv)]
        } else {
            &[(Keep::U, pu), (Keep::V, pv), (Keep::Mid, mid)]
        };
        let (keep, target) = options
            .iter()
            .copied()
            .min_by(|a, b| quadric.error(a.1).total_cmp(&quadric.error(b.1)))?;
        Some(Candidate {
            cost: quadric.error(target).max(0.0),
            u,
            v,
            stamps: (self.stamps[u as usize], self.stamps[v as usize]),
            target,
            keep,
        })
    }

    /// Rejects collapses that create non-manifold edges or flip faces.
    fn allowed(&self, u: u32, v: u32, target: [f64; 3]) -> bool {
        let (nu, nv) = (self.neighbors(u), self.neighbors(v));
        let shared = nu
            .iter()
            .filter(|vertex| nv.binary_search(vertex).is_ok())
            .count();
        if shared > 2 {
            return false;
        }
        for moving in [u, v] {
            for &face in &self.vertex_faces[moving as usize] {
                if !self.face_alive[face as usize] {
                    continue;
                }
                let corners = self.faces[face as usize];
                if corners.contains(&u) && corners.contains(&v) {
                    continue;
                }
                let before = corners.map(|index| self.positions[index as usize]);
                let after = corners.map(|index| {
                    if index == moving {
                        target
                    } else {
                        self.positions[index as usize]
                    }
                });
                let old_normal =
                    normalized(cross(sub(before[1], before[0]), sub(before[2], before[0])));
                let new_normal =
                    normalized(cross(sub(after[1], after[0]), sub(after[2], after[0])));
                match (old_normal, new_normal) {
                    (Some(old), Some(new)) if dot(old, new) >= FLIP_DOT_MIN => {}
                    (None, Some(_)) => {}
                    _ => return false,
                }
            }
        }
        true
    }

    /// Merges `u` into `v`.
    fn collapse(&mut self, candidate: &Candidate) {
        let (u, v) = (candidate.u as usize, candidate.v as usize);
        for face in std::mem::take(&mut self.vertex_faces[u]) {
            if !self.face_alive[face as usize] {
                continue;
            }
            let corners = &mut self.faces[face as usize];
            if corners.contains(&(v as u32)) {
                self.face_alive[face as usize] = false;
                self.live_faces -= 1;
            } else {
                for corner in corners.iter_mut().filter(|corner| **corner == u as u32) {
                    *corner = v as u32;
                }
                self.vertex_faces[v].push(face);
            }
        }
        let face_alive = &self.face_alive;
        self.vertex_faces[v].retain(|face| face_alive[*face as usize]);
        self.positions[v] = candidate.target;
        match candidate.keep {
            Keep::V => {}
            Keep::U => {
                if let Some(uvs) = &mut self.uvs {
                    uvs[v] = uvs[u];
                }
                if let Some(colors) = &mut self.colors {
                    colors[v] = colors[u];
                }
            }
            Keep::Mid => {
                if let Some(uvs) = &mut self.uvs {
                    uvs[v] = [(uvs[u][0] + uvs[v][0]) / 2.0, (uvs[u][1] + uvs[v][1]) / 2.0];
                }
                if let Some(colors) = &mut self.colors {
                    colors[v] = [0, 1, 2].map(|channel| {
                        ((u16::from(colors[u][channel]) + u16::from(colors[v][channel])) / 2) as u8
                    });
                }
            }
        }
        let merged = self.quadrics[u];
        self.quadrics[v].add(&merged);
        self.locked[v] |= self.locked[u];
        self.alive[u] = false;
        self.stamps[v] = self.stamps[v].wrapping_add(1);
    }

    fn into_mesh(self, source: &ExportMesh) -> ExportMesh {
        let mut remap = vec![u32::MAX; self.positions.len()];
        let mut out = ExportMesh {
            texture: source.texture.clone(),
            uvs: self.uvs.as_ref().map(|_| Vec::new()),
            colors: self.colors.as_ref().map(|_| Vec::new()),
            ..ExportMesh::default()
        };
        for (face, alive) in self.faces.iter().zip(&self.face_alive) {
            if !alive {
                continue;
            }
            for &index in face {
                let slot = &mut remap[index as usize];
                if *slot == u32::MAX {
                    *slot = out.positions.len() as u32;
                    out.positions
                        .push(self.positions[index as usize].map(|value| value as f32));
                    if let (Some(dst), Some(src)) = (&mut out.uvs, &self.uvs) {
                        dst.push(src[index as usize]);
                    }
                    if let (Some(dst), Some(src)) = (&mut out.colors, &self.colors) {
                        dst.push(src[index as usize]);
                    }
                }
                out.indices.push(*slot);
            }
        }
        out
    }
}

/// Reduces `mesh` towards `target_triangles`. The result may stay above the
/// target when every remaining collapse would damage the surface or a border.
pub fn decimate(
    mesh: &ExportMesh,
    options: DecimateOptions,
) -> Result<(ExportMesh, DecimateReport), String> {
    let source_triangles = mesh.triangles();
    if options.target_triangles < MIN_TARGET_TRIANGLES
        || options.target_triangles >= source_triangles
    {
        return Err(format!(
            "Target must be between {MIN_TARGET_TRIANGLES} and {} triangles (below the current count).",
            source_triangles.saturating_sub(1)
        ));
    }
    let mut simplifier = Simplifier::new(mesh, options.preserve_borders);
    let mut heap: BinaryHeap<Candidate> = simplifier
        .edges
        .iter()
        .filter_map(|&(u, v)| simplifier.candidate(u, v))
        .collect();
    while simplifier.live_faces > options.target_triangles {
        let Some(candidate) = heap.pop() else {
            break;
        };
        let (u, v) = (candidate.u as usize, candidate.v as usize);
        let stale = !simplifier.alive[u]
            || !simplifier.alive[v]
            || candidate.stamps != (simplifier.stamps[u], simplifier.stamps[v]);
        if stale || !simplifier.allowed(candidate.u, candidate.v, candidate.target) {
            continue;
        }
        simplifier.collapse(&candidate);
        for neighbor in simplifier.neighbors(candidate.v) {
            if let Some(next) = simplifier.candidate(candidate.v, neighbor) {
                heap.push(next);
            }
        }
    }
    let result = simplifier.into_mesh(mesh);
    let report = DecimateReport {
        source_triangles,
        result_triangles: result.triangles(),
        source_vertices: mesh.positions.len(),
        result_vertices: result.positions.len(),
    };
    Ok((result, report))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Flat n x n grid of quads in the z=0 plane, 2 * n * n triangles.
    fn grid(n: u32) -> ExportMesh {
        let mut mesh = ExportMesh::default();
        for y in 0..=n {
            for x in 0..=n {
                mesh.positions.push([x as f32, y as f32, 0.0]);
            }
        }
        let at = |x: u32, y: u32| y * (n + 1) + x;
        for y in 0..n {
            for x in 0..n {
                mesh.indices.extend([
                    at(x, y),
                    at(x + 1, y),
                    at(x + 1, y + 1),
                    at(x, y),
                    at(x + 1, y + 1),
                    at(x, y + 1),
                ]);
            }
        }
        mesh.colors = Some(vec![[200, 100, 50]; mesh.positions.len()]);
        mesh
    }

    /// Closed UV sphere of radius 1.
    fn sphere(rings: u32, segments: u32) -> ExportMesh {
        let mut mesh = ExportMesh::default();
        mesh.positions.push([0.0, 1.0, 0.0]);
        for ring in 1..rings {
            let phi = std::f32::consts::PI * ring as f32 / rings as f32;
            for segment in 0..segments {
                let theta = std::f32::consts::TAU * segment as f32 / segments as f32;
                mesh.positions
                    .push([phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin()]);
            }
        }
        mesh.positions.push([0.0, -1.0, 0.0]);
        let ring_start = |ring: u32| 1 + (ring - 1) * segments;
        let bottom = mesh.positions.len() as u32 - 1;
        for segment in 0..segments {
            let next = (segment + 1) % segments;
            mesh.indices
                .extend([0, ring_start(1) + next, ring_start(1) + segment]);
            mesh.indices.extend([
                bottom,
                ring_start(rings - 1) + segment,
                ring_start(rings - 1) + next,
            ]);
        }
        for ring in 1..rings - 1 {
            for segment in 0..segments {
                let next = (segment + 1) % segments;
                let (a, b) = (ring_start(ring) + segment, ring_start(ring) + next);
                let (c, d) = (ring_start(ring + 1) + segment, ring_start(ring + 1) + next);
                mesh.indices.extend([a, b, d, a, d, c]);
            }
        }
        mesh
    }

    fn assert_valid(mesh: &ExportMesh) {
        assert_eq!(mesh.indices.len() % 3, 0);
        assert!(mesh
            .indices
            .iter()
            .all(|index| (*index as usize) < mesh.positions.len()));
        for face in mesh.indices.as_chunks::<3>().0 {
            assert!(
                face[0] != face[1] && face[1] != face[2] && face[0] != face[2],
                "degenerate face"
            );
        }
    }

    #[test]
    fn flat_grid_reduces_and_stays_flat_with_fixed_border() {
        let mesh = grid(20);
        let options = DecimateOptions {
            target_triangles: 120,
            preserve_borders: true,
        };
        let (out, report) = decimate(&mesh, options).unwrap();
        assert_valid(&out);
        assert!(report.result_triangles <= 200, "{report:?}");
        assert!(report.result_triangles < report.source_triangles / 3);
        assert!(
            out.positions.iter().all(|p| p[2] == 0.0),
            "planar surface stays planar"
        );
        let max = out
            .positions
            .iter()
            .fold([0f32; 2], |m, p| [m[0].max(p[0]), m[1].max(p[1])]);
        assert_eq!(max, [20.0, 20.0], "border corners kept");
        assert!(
            out.colors.unwrap().iter().all(|c| *c == [200, 100, 50]),
            "attributes carried"
        );
    }

    #[test]
    fn sphere_keeps_its_shape_closely() {
        let mesh = sphere(32, 64);
        let source = mesh.triangles();
        let options = DecimateOptions {
            target_triangles: source / 10,
            preserve_borders: true,
        };
        let (out, report) = decimate(&mesh, options).unwrap();
        assert_valid(&out);
        assert!(report.result_triangles <= source / 10 + 2, "{report:?}");
        let worst = out
            .positions
            .iter()
            .map(|p| ((p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt() - 1.0).abs())
            .fold(0.0f32, f32::max);
        assert!(
            worst < 0.05,
            "vertices stay on the sphere, worst deviation {worst}"
        );
        // Closed surface: every edge still shared by exactly two faces.
        let mut uses: HashMap<(u32, u32), u32> = HashMap::new();
        for face in out.indices.as_chunks::<3>().0 {
            for corner in 0..3 {
                let (a, b) = (face[corner], face[(corner + 1) % 3]);
                *uses.entry((a.min(b), a.max(b))).or_default() += 1;
            }
        }
        assert!(
            uses.values().all(|count| *count == 2),
            "stays watertight and manifold"
        );
    }

    #[test]
    fn unlocked_borders_allow_further_reduction() {
        let mesh = grid(20);
        let locked = decimate(
            &mesh,
            DecimateOptions {
                target_triangles: 20,
                preserve_borders: true,
            },
        )
        .unwrap()
        .1;
        let free = decimate(
            &mesh,
            DecimateOptions {
                target_triangles: 20,
                preserve_borders: false,
            },
        )
        .unwrap()
        .1;
        assert!(free.result_triangles <= locked.result_triangles);
    }

    #[test]
    fn rejects_invalid_targets() {
        let mesh = grid(2);
        assert!(decimate(
            &mesh,
            DecimateOptions {
                target_triangles: 8,
                preserve_borders: true
            }
        )
        .is_err());
        assert!(decimate(
            &mesh,
            DecimateOptions {
                target_triangles: 2,
                preserve_borders: true
            }
        )
        .is_err());
    }
}
