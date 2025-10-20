use crate::error::{ConversionError, Result};
use crate::mesh_types::{IntermediateMesh, IntermediateVertex};
use std::collections::HashMap;

pub fn obj_to_intermediate(obj_data: &[u8]) -> Result<IntermediateMesh> {
    let (models, _) = tobj::load_obj_buf(
        &mut obj_data.as_ref(),
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
        |_| {
            Ok(Default::default())
        },
    )?;

    if models.is_empty() || models[0].mesh.indices.is_empty() {
        return Err(ConversionError::NoMeshData);
    }
    
    let mut combined_vertices: Vec<IntermediateVertex> = Vec::new();
    let mut combined_faces: Vec<[u32; 3]> = Vec::new();

    for model in models {
        let mesh = &model.mesh;
        let mut vertex_map: HashMap<u32, u32> = HashMap::new();

        let has_normals = !mesh.normals.is_empty();
        let has_uvs = !mesh.texcoords.is_empty();
        
        let new_faces = mesh.indices
            .chunks_exact(3)
            .map(|face_indices| {
                let mut new_face = [0u32; 3];
                for i in 0..3 {
                    let original_index = face_indices[i];
                    let new_index = if let Some(&existing) = vertex_map.get(&original_index) {
                        existing
                    } else {
                        let idx = original_index as usize;
                        let pos = [
                            mesh.positions[idx * 3],
                            mesh.positions[idx * 3 + 1],
                            mesh.positions[idx * 3 + 2],
                        ];
                        let normal = if has_normals && idx * 3 + 2 < mesh.normals.len() {
                            [
                                mesh.normals[idx * 3],
                                mesh.normals[idx * 3 + 1],
                                mesh.normals[idx * 3 + 2],
                            ]
                        } else {
                            [0.0, 1.0, 0.0]
                        };
                        let uv = if has_uvs && idx * 2 + 1 < mesh.texcoords.len() {
                            [
                                mesh.texcoords[idx * 2],
                                mesh.texcoords[idx * 2 + 1],
                            ]
                        } else {
                            [0.0, 0.0]
                        };

                        let vertex = IntermediateVertex {
                            pos,
                            normal,
                            uv: [uv[0], 1.0 - uv[1]],
                        };
                        
                        combined_vertices.push(vertex);
                        let stored = (combined_vertices.len() - 1) as u32;
                        vertex_map.insert(original_index, stored);
                        stored
                    };
                    new_face[i] = new_index;
                }
                new_face
            })
            .collect::<Vec<_>>();
        
        combined_faces.extend(new_faces);
    }

    Ok(IntermediateMesh {
        vertices: combined_vertices,
        faces: combined_faces,
    })
}
