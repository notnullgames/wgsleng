use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ObjModel {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

impl ObjModel {
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read OBJ file: {}", e))?;

        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut indices = Vec::new();

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            match parts[0] {
                "v" => {
                    // Vertex position
                    if parts.len() >= 4 {
                        let x: f32 = parts[1].parse().map_err(|e| format!("Failed to parse vertex x: {}", e))?;
                        let y: f32 = parts[2].parse().map_err(|e| format!("Failed to parse vertex y: {}", e))?;
                        let z: f32 = parts[3].parse().map_err(|e| format!("Failed to parse vertex z: {}", e))?;
                        positions.push([x, y, z]);
                    }
                }
                "vn" => {
                    // Vertex normal
                    if parts.len() >= 4 {
                        let x: f32 = parts[1].parse().map_err(|e| format!("Failed to parse normal x: {}", e))?;
                        let y: f32 = parts[2].parse().map_err(|e| format!("Failed to parse normal y: {}", e))?;
                        let z: f32 = parts[3].parse().map_err(|e| format!("Failed to parse normal z: {}", e))?;
                        normals.push([x, y, z]);
                    }
                }
                "f" => {
                    // Face (triangle)
                    if parts.len() >= 4 {
                        // Parse face indices (format: v, v/vt, v/vt/vn, or v//vn)
                        for i in 1..=3 {
                            let index_str = parts[i].split('/').next().unwrap();
                            let index: u32 = index_str.parse()
                                .map_err(|e| format!("Failed to parse face index: {}", e))?;
                            // OBJ indices are 1-based, convert to 0-based
                            indices.push(index - 1);
                        }
                    }
                }
                _ => {
                    // Ignore other OBJ elements (vt, mtllib, usemtl, etc.)
                }
            }
        }

        // If no normals provided, calculate them
        if normals.is_empty() && !positions.is_empty() && !indices.is_empty() {
            normals = Self::calculate_normals(&positions, &indices);
        }

        println!("Loaded OBJ: {} vertices, {} normals, {} triangles",
                 positions.len(), normals.len(), indices.len() / 3);

        Ok(ObjModel {
            positions,
            normals,
            indices,
        })
    }

    fn calculate_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
        let mut normals = vec![[0.0f32, 0.0, 0.0]; positions.len()];

        // Accumulate face normals for each vertex
        for triangle in indices.chunks(3) {
            let i0 = triangle[0] as usize;
            let i1 = triangle[1] as usize;
            let i2 = triangle[2] as usize;

            let v0 = positions[i0];
            let v1 = positions[i1];
            let v2 = positions[i2];

            // Calculate face normal
            let edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
            let edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

            let normal = [
                edge1[1] * edge2[2] - edge1[2] * edge2[1],
                edge1[2] * edge2[0] - edge1[0] * edge2[2],
                edge1[0] * edge2[1] - edge1[1] * edge2[0],
            ];

            // Accumulate to vertex normals
            normals[i0][0] += normal[0];
            normals[i0][1] += normal[1];
            normals[i0][2] += normal[2];

            normals[i1][0] += normal[0];
            normals[i1][1] += normal[1];
            normals[i1][2] += normal[2];

            normals[i2][0] += normal[0];
            normals[i2][1] += normal[1];
            normals[i2][2] += normal[2];
        }

        // Normalize all normals
        for normal in &mut normals {
            let len = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
            if len > 0.0 {
                normal[0] /= len;
                normal[1] /= len;
                normal[2] /= len;
            }
        }

        normals
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }
}
