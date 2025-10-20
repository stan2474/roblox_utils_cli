use crate::error::Result;
use crate::mesh_types::*;
use byteorder::{LittleEndian, WriteBytesExt};
use std::io::Write;

pub enum V1Version {
    V1_00,
    V1_01,
}

pub fn write_v1(mesh: &IntermediateMesh, version: V1Version) -> Result<Vec<u8>> {
    let mut writer = Vec::new();
    let (header, scaler) = match version {
        V1Version::V1_00 => ("version 1.00\n", 2.0f32),
        V1Version::V1_01 => ("version 1.01\n", 1.0f32),
    };

    write!(writer, "{}", header)?;
    write!(writer, "{}\n", mesh.faces.len())?;

    for face in &mesh.faces {
        for &vertex_index in face {
            let vertex = &mesh.vertices[vertex_index as usize];
            let p = vertex.pos;
            let n = vertex.normal;
            let uv = vertex.uv;

            write!(
                writer,
                "[{:.6},{:.6},{:.6}][{:.6},{:.6},{:.6}][{:.6},{:.6},{:.6}]",
                p[0] * scaler, p[1] * scaler, p[2] * scaler,
                n[0], n[1], n[2],
                uv[0], uv[1], 0.0,
            )?;
        }
    }

    Ok(writer)
}

pub fn write_v2(mesh: &IntermediateMesh) -> Result<Vec<u8>> {
    let mut writer = Vec::new();
    write!(writer, "version 2.00\n")?;

    let num_verts = mesh.vertices.len() as u32;
    let num_faces = mesh.faces.len() as u32;

    let header = FileMeshHeaderV2 {
        sizeof_FileMeshHeaderV2: std::mem::size_of::<FileMeshHeaderV2>() as u16,
        sizeof_FileMeshVertex: std::mem::size_of::<FileMeshVertex>() as u8,
        sizeof_FileMeshFace: std::mem::size_of::<FileMeshFace>() as u8,
        numVerts: num_verts,
        numFaces: num_faces,
    };
    
    writer.write_all(as_bytes(&header))?;
    
    for vertex in &mesh.vertices {
        let file_vertex = FileMeshVertex {
            px: vertex.pos[0], py: vertex.pos[1], pz: vertex.pos[2],
            nx: vertex.normal[0], ny: vertex.normal[1], nz: vertex.normal[2],
            tu: vertex.uv[0], tv: vertex.uv[1],
            ..Default::default()
        };
        writer.write_all(as_bytes(&file_vertex))?;
    }

    for face in &mesh.faces {
        let file_face = FileMeshFace { a: face[0], b: face[1], c: face[2] };
        writer.write_all(as_bytes(&file_face))?;
    }
    
    Ok(writer)
}

pub fn write_v3(mesh: &IntermediateMesh) -> Result<Vec<u8>> {
    let mut writer = Vec::new();
    write!(writer, "version 3.00\n")?;

    let num_verts = mesh.vertices.len() as u32;
    let num_faces = mesh.faces.len() as u32;

    let header = FileMeshHeaderV3 {
        sizeof_FileMeshHeaderV3: std::mem::size_of::<FileMeshHeaderV3>() as u16,
        sizeof_FileMeshVertex: std::mem::size_of::<FileMeshVertex>() as u8,
        sizeof_FileMeshFace: std::mem::size_of::<FileMeshFace>() as u8,
        sizeof_LodOffset: 4,
        numLodOffsets: 1,
        numVerts: num_verts,
        numFaces: num_faces,
    };

    writer.write_all(as_bytes(&header))?;
    
    for vertex in &mesh.vertices {
        let file_vertex = FileMeshVertex {
            px: vertex.pos[0], py: vertex.pos[1], pz: vertex.pos[2],
            nx: vertex.normal[0], ny: vertex.normal[1], nz: vertex.normal[2],
            tu: vertex.uv[0], tv: vertex.uv[1],
            ..Default::default()
        };
        writer.write_all(as_bytes(&file_vertex))?;
    }

    for face in &mesh.faces {
        let file_face = FileMeshFace { a: face[0], b: face[1], c: face[2] };
        writer.write_all(as_bytes(&file_face))?;
    }

    writer.write_u32::<LittleEndian>(0)?;

    Ok(writer)
}

pub fn write_v4(mesh: &IntermediateMesh) -> Result<Vec<u8>> {
    let mut writer = Vec::new();
    write!(writer, "version 4.00\n")?;

    let num_verts = mesh.vertices.len() as u32;
    let num_faces = mesh.faces.len() as u32;

    let header = FileMeshHeaderV4 {
        sizeof_FileMeshHeaderV4: std::mem::size_of::<FileMeshHeaderV4>() as u16,
        lodType: 0,
        numVerts: num_verts,
        numFaces: num_faces,
        numLodOffsets: 1,
        numBones: 0,
        sizeof_boneNames: 0,
        numSubsets: 0,
        numHighQualityLODs: 1,
        unused: 0,
    };
    
    writer.write_all(as_bytes(&header))?;
    
    for vertex in &mesh.vertices {
        let file_vertex = FileMeshVertex {
            px: vertex.pos[0], py: vertex.pos[1], pz: vertex.pos[2],
            nx: vertex.normal[0], ny: vertex.normal[1], nz: vertex.normal[2],
            tu: vertex.uv[0], tv: vertex.uv[1],
            ..Default::default()
        };
        writer.write_all(as_bytes(&file_vertex))?;
    }

    for face in &mesh.faces {
        let file_face = FileMeshFace { a: face[0], b: face[1], c: face[2] };
        writer.write_all(as_bytes(&file_face))?;
    }

    writer.write_u32::<LittleEndian>(0)?;

    Ok(writer)
}

pub fn write_v5(mesh: &IntermediateMesh) -> Result<Vec<u8>> {
    let mut writer = Vec::new();
    write!(writer, "version 5.00\n")?;

    let num_verts = mesh.vertices.len() as u32;
    let num_faces = mesh.faces.len() as u32;

    let header = FileMeshHeaderV5 {
        sizeof_MeshHeader: std::mem::size_of::<FileMeshHeaderV5>() as u16,
        lodType: 0,
        numVerts: num_verts,
        numFaces: num_faces,
        numLodOffsets: 1,
        numBones: 0,
        sizeof_boneNameBuffer: 0,
        numSubsets: 0,
        numHighQualityLODs: 1,
        unusedPadding: 0,
        facsDataFormat: 0,
        facsDataSize: 0,
    };
    
    writer.write_all(as_bytes(&header))?;

    for vertex in &mesh.vertices {
        let file_vertex = FileMeshVertex {
            px: vertex.pos[0], py: vertex.pos[1], pz: vertex.pos[2],
            nx: vertex.normal[0], ny: vertex.normal[1], nz: vertex.normal[2],
            tu: vertex.uv[0], tv: vertex.uv[1],
            ..Default::default()
        };
        writer.write_all(as_bytes(&file_vertex))?;
    }

    for face in &mesh.faces {
        let file_face = FileMeshFace { a: face[0], b: face[1], c: face[2] };
        writer.write_all(as_bytes(&file_face))?;
    }

    writer.write_u32::<LittleEndian>(0)?;

    Ok(writer)
}

fn as_bytes<T: Sized>(p: &T) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            (p as *const T) as *const u8,
            std::mem::size_of::<T>(),
        )
    }
}
