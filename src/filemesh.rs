// https://devforum.roblox.com/t/roblox-filemesh-format-specification/326114/ 
use crate::error::{ConversionError, Result};
use crate::mesh_types::{FileMeshFace, FileMeshHeaderV2, FileMeshHeaderV3, FileMeshHeaderV4, FileMeshHeaderV5, FileMeshVertex, IntermediateMesh, IntermediateVertex};
use byteorder::{LittleEndian, ReadBytesExt};
use std::cmp::min;
use std::fmt::{self, Write as FmtWrite};
use std::io::{Cursor, Read};

const FILEMESH_VERTEX_SIZE_WITH_RGBA: usize = std::mem::size_of::<FileMeshVertex>();

pub fn parse_filemesh(data: &[u8]) -> Result<IntermediateMesh> {
    let newline = data
        .iter()
        .position(|&b| b == b'\n')
        .ok_or_else(|| parse_err("missing version header"))?;

    let mut header_bytes = &data[..newline];
    if header_bytes.ends_with(&[b'\r']) {
        header_bytes = &header_bytes[..header_bytes.len() - 1];
    }

    let version_str = std::str::from_utf8(header_bytes)
        .map_err(|_| parse_err("header is not valid UTF-8"))?
        .trim();

    let body = &data[newline + 1..];

    match version_str {
        "version 1.00" => parse_v1(body, true),
        "version 1.01" => parse_v1(body, false),
        "version 2.00" => parse_v2(body),
        "version 3.00" | "version 3.01" => parse_v3(body),
        "version 4.00" | "version 4.01" => parse_v4(body),
        "version 5.00" => parse_v5(body),
        _ => Err(ConversionError::Unsupported(format!(
            "unsupported filemesh version: {}",
            version_str
        ))),
    }
}

pub fn filemesh_to_obj_bytes(data: &[u8]) -> Result<Vec<u8>> {
    let mesh = parse_filemesh(data)?;
    mesh_to_obj_bytes(&mesh)
}

pub fn mesh_to_obj_bytes(mesh: &IntermediateMesh) -> Result<Vec<u8>> {
    let mut output = String::new();

    for vertex in &mesh.vertices {
        fmt_ok(writeln!(
            &mut output,
            "v {:.6} {:.6} {:.6}",
            vertex.pos[0],
            vertex.pos[1],
            vertex.pos[2]
        ))?;
    }

    for vertex in &mesh.vertices {
        fmt_ok(writeln!(
            &mut output,
            "vt {:.6} {:.6}",
            vertex.uv[0],
            1.0 - vertex.uv[1]
        ))?;
    }

    for vertex in &mesh.vertices {
        fmt_ok(writeln!(
            &mut output,
            "vn {:.6} {:.6} {:.6}",
            vertex.normal[0],
            vertex.normal[1],
            vertex.normal[2]
        ))?;
    }

    for face in &mesh.faces {
        fmt_ok(writeln!(
            &mut output,
            "f {}/{}/{} {}/{}/{} {}/{}/{}",
            face[0] + 1,
            face[0] + 1,
            face[0] + 1,
            face[1] + 1,
            face[1] + 1,
            face[1] + 1,
            face[2] + 1,
            face[2] + 1,
            face[2] + 1
        ))?;
    }

    Ok(output.into_bytes())
}

fn parse_v1(body: &[u8], scale_half: bool) -> Result<IntermediateMesh> {
    let body_str = std::str::from_utf8(body).map_err(|_| parse_err("ascii mesh is not UTF-8"))?;
    let mut lines = body_str.lines();

    let faces_line = lines
        .next()
        .ok_or_else(|| parse_err("missing face count"))?
        .trim();
    let num_faces: usize = faces_line
        .parse()
        .map_err(|_| parse_err("invalid face count"))?;

    let data_line = lines
        .next()
        .ok_or_else(|| parse_err("missing vertex data line"))?
        .trim();

    let vectors = parse_bracket_vectors(data_line)?;
    if vectors.len() != num_faces * 9 {
        return Err(parse_err("unexpected vertex vector count"));
    }

    let mut vertices = Vec::with_capacity(num_faces * 3);
    let mut faces = Vec::with_capacity(num_faces);

    for face_index in 0..num_faces {
        let mut face = [0u32; 3];
        for corner in 0..3 {
            let base = face_index * 9 + corner * 3;
            let pos_vec = vectors[base];
            let norm_vec = vectors[base + 1];
            let uv_vec = vectors[base + 2];

            let mut pos = pos_vec;
            if scale_half {
                pos = [pos[0] * 0.5, pos[1] * 0.5, pos[2] * 0.5];
            }

            let vertex = IntermediateVertex {
                pos,
                normal: norm_vec,
                uv: [uv_vec[0], 1.0 - uv_vec[1]],
            };

            let stored_index = vertices.len() as u32;
            vertices.push(vertex);
            face[corner] = stored_index;
        }
        faces.push(face);
    }

    Ok(IntermediateMesh { vertices, faces })
}

fn parse_v2(body: &[u8]) -> Result<IntermediateMesh> {
    let mut cursor = Cursor::new(body);

    let header_size = cursor.read_u16::<LittleEndian>()?;
    if header_size as usize != std::mem::size_of::<FileMeshHeaderV2>() {
        return Err(parse_err("unexpected header size for v2"));
    }

    let sizeof_vertex = cursor.read_u8()?;
    let sizeof_face = cursor.read_u8()?;
    if sizeof_face as usize != std::mem::size_of::<FileMeshFace>() {
        return Err(parse_err("unexpected face size for v2"));
    }

    let num_verts = cursor.read_u32::<LittleEndian>()?;
    let num_faces = cursor.read_u32::<LittleEndian>()?;

    let has_rgba = sizeof_vertex as usize == FILEMESH_VERTEX_SIZE_WITH_RGBA;
    let vertices = read_vertices(&mut cursor, num_verts as usize, has_rgba)?;
    let faces = read_faces(&mut cursor, num_faces as usize)?;

    Ok(IntermediateMesh { vertices, faces })
}

fn parse_v3(body: &[u8]) -> Result<IntermediateMesh> {
    let mut cursor = Cursor::new(body);

    let header_size = cursor.read_u16::<LittleEndian>()?;
    if header_size as usize != std::mem::size_of::<FileMeshHeaderV3>() {
        return Err(parse_err("unexpected header size for v3"));
    }

    let sizeof_vertex = cursor.read_u8()? as usize;
    let sizeof_face = cursor.read_u8()? as usize;
    if sizeof_face != std::mem::size_of::<FileMeshFace>() {
        return Err(parse_err("unexpected face size for v3"));
    }

    let _sizeof_lod_offset = cursor.read_u16::<LittleEndian>()?;
    let num_lod_offsets = cursor.read_u16::<LittleEndian>()? as usize;
    let num_verts = cursor.read_u32::<LittleEndian>()?;
    let num_faces = cursor.read_u32::<LittleEndian>()?;

    let has_rgba = match sizeof_vertex {
        FILEMESH_VERTEX_SIZE_WITH_RGBA => true,
        _ => return Err(parse_err("unsupported v3 vertex stride")),
    };
    let vertices = read_vertices(&mut cursor, num_verts as usize, has_rgba)?;
    let mut faces = read_faces(&mut cursor, num_faces as usize)?;

    let mut lod_offsets = Vec::with_capacity(num_lod_offsets);
    for _ in 0..num_lod_offsets {
        lod_offsets.push(cursor.read_u32::<LittleEndian>()?);
    }

    let base_face_count = lod_offsets.get(1).copied().unwrap_or(num_faces);
    let base_face_count = min(base_face_count, num_faces);
    faces.truncate(min(base_face_count as usize, faces.len()));

    Ok(IntermediateMesh { vertices, faces })
}

fn parse_v4(body: &[u8]) -> Result<IntermediateMesh> {
    let mut cursor = Cursor::new(body);

    let header_size = cursor.read_u16::<LittleEndian>()?;
    if header_size as usize != std::mem::size_of::<FileMeshHeaderV4>() {
        return Err(parse_err("unexpected header size for v4"));
    }

    let _lod_type = cursor.read_u16::<LittleEndian>()?;
    let num_verts = cursor.read_u32::<LittleEndian>()?;
    let num_faces = cursor.read_u32::<LittleEndian>()?;
    let num_lod_offsets = cursor.read_u16::<LittleEndian>()? as usize;
    let num_bones = cursor.read_u16::<LittleEndian>()?;
    let sizeof_bone_names = cursor.read_u32::<LittleEndian>()?;
    let num_subsets = cursor.read_u16::<LittleEndian>()?;
    let _num_high_quality_lods = cursor.read_u8()?;
    let _unused = cursor.read_u8()?;

    if num_bones != 0 || sizeof_bone_names != 0 || num_subsets != 0 {
        return Err(ConversionError::Unsupported(
            "v4 meshes with skinning/subsets are not supported".to_string(),
        ));
    }

    let vertex_block_bytes = {
        let total_len = cursor.get_ref().len();
        let current_pos = cursor.position() as usize;
        let faces_bytes = num_faces as usize * std::mem::size_of::<FileMeshFace>();
        let lod_bytes = num_lod_offsets * 4;
        total_len
            .checked_sub(current_pos)
            .and_then(|remaining| remaining.checked_sub(faces_bytes + lod_bytes))
            .ok_or_else(|| parse_err("invalid v4 vertex block size"))?
    };
    let sizeof_vertex = vertex_block_bytes / num_verts as usize;
    let has_rgba = match sizeof_vertex {
        s if s == FILEMESH_VERTEX_SIZE_WITH_RGBA => true,
        s if s == FILEMESH_VERTEX_SIZE_WITH_RGBA - 4 => false,
        _ => {
            return Err(parse_err("unsupported v4 vertex stride"));
        }
    };

    let mut vertices = read_vertices(&mut cursor, num_verts as usize, has_rgba)?;
    let mut faces = read_faces(&mut cursor, num_faces as usize)?;

    let mut lod_offsets = Vec::with_capacity(num_lod_offsets);
    for _ in 0..num_lod_offsets {
        lod_offsets.push(cursor.read_u32::<LittleEndian>()?);
    }

    let base_face_count = lod_offsets.get(1).copied().unwrap_or(num_faces);
    let base_face_count = min(base_face_count, num_faces);
    faces.truncate(min(base_face_count as usize, faces.len()));

    Ok(IntermediateMesh { vertices: vertices.drain(..).collect(), faces })
}

fn parse_v5(body: &[u8]) -> Result<IntermediateMesh> {
    let mut cursor = Cursor::new(body);

    let header_size = cursor.read_u16::<LittleEndian>()?;
    if header_size as usize != std::mem::size_of::<FileMeshHeaderV5>() {
        return Err(parse_err("unexpected header size for v5"));
    }

    let _lod_type = cursor.read_u16::<LittleEndian>()?;
    let num_verts = cursor.read_u32::<LittleEndian>()?;
    let num_faces = cursor.read_u32::<LittleEndian>()?;
    let num_lod_offsets = cursor.read_u16::<LittleEndian>()? as usize;
    let num_bones = cursor.read_u16::<LittleEndian>()?;
    let sizeof_bone_names = cursor.read_u32::<LittleEndian>()?;
    let num_subsets = cursor.read_u16::<LittleEndian>()?;
    let _num_high_quality_lods = cursor.read_u8()?;
    let _unused = cursor.read_u8()?;
    let facs_format = cursor.read_u32::<LittleEndian>()?;
    let facs_size = cursor.read_u32::<LittleEndian>()?;

    if num_bones != 0 || sizeof_bone_names != 0 || num_subsets != 0 {
        return Err(ConversionError::Unsupported(
            "v5 meshes with skinning/subsets are not supported".to_string(),
        ));
    }

    if facs_format != 0 || facs_size != 0 {
        return Err(ConversionError::Unsupported(
            "v5 meshes with FACS data are not supported".to_string(),
        ));
    }

    let mut vertices = read_vertices(&mut cursor, num_verts as usize, true)?;
    let mut faces = read_faces(&mut cursor, num_faces as usize)?;

    let mut lod_offsets = Vec::with_capacity(num_lod_offsets);
    for _ in 0..num_lod_offsets {
        lod_offsets.push(cursor.read_u32::<LittleEndian>()?);
    }

    let base_face_count = lod_offsets.get(1).copied().unwrap_or(num_faces);
    let base_face_count = min(base_face_count, num_faces);
    faces.truncate(min(base_face_count as usize, faces.len()));

    Ok(IntermediateMesh { vertices: vertices.drain(..).collect(), faces })
}

fn read_vertices(cursor: &mut Cursor<&[u8]>, count: usize, has_rgba: bool) -> Result<Vec<IntermediateVertex>> {
    let mut vertices = Vec::with_capacity(count);

    for _ in 0..count {
        let px = cursor.read_f32::<LittleEndian>()?;
        let py = cursor.read_f32::<LittleEndian>()?;
        let pz = cursor.read_f32::<LittleEndian>()?;
        let nx = cursor.read_f32::<LittleEndian>()?;
        let ny = cursor.read_f32::<LittleEndian>()?;
        let nz = cursor.read_f32::<LittleEndian>()?;
        let tu = cursor.read_f32::<LittleEndian>()?;
        let tv = cursor.read_f32::<LittleEndian>()?;
        let _tx = cursor.read_i8()?;
        let _ty = cursor.read_i8()?;
        let _tz = cursor.read_i8()?;
        let _ts = cursor.read_i8()?;

        if has_rgba {
            let mut rgba = [0u8; 4];
            cursor.read_exact(&mut rgba)?;
        }

        vertices.push(IntermediateVertex {
            pos: [px, py, pz],
            normal: [nx, ny, nz],
            uv: [tu, 1.0 - tv],
        });
    }

    Ok(vertices)
}

fn read_faces(cursor: &mut Cursor<&[u8]>, count: usize) -> Result<Vec<[u32; 3]>> {
    let mut faces = Vec::with_capacity(count);
    for _ in 0..count {
        let a = cursor.read_u32::<LittleEndian>()?;
        let b = cursor.read_u32::<LittleEndian>()?;
        let c = cursor.read_u32::<LittleEndian>()?;
        faces.push([a, b, c]);
    }
    Ok(faces)
}

fn parse_bracket_vectors(input: &str) -> Result<Vec<[f32; 3]>> {
    let mut vectors = Vec::new();
    let mut rest = input;

    while let Some(start) = rest.find('[') {
        let after_start = &rest[start + 1..];
        let end_rel = after_start
            .find(']')
            .ok_or_else(|| parse_err("missing closing bracket in ASCII mesh"))?;
        let end = start + 1 + end_rel;
        let inside = &rest[start + 1..end];

        let mut components = Vec::new();
        for comp in inside.split(',') {
            components.push(
                comp.trim()
                    .parse::<f32>()
                    .map_err(|_| parse_err("invalid float in ASCII mesh"))?,
            );
        }

        if components.len() != 3 {
            return Err(parse_err("expected three components per vector"));
        }

        vectors.push([components[0], components[1], components[2]]);

        rest = &rest[end + 1..];
    }

    Ok(vectors)
}

fn parse_err(message: impl Into<String>) -> ConversionError {
    ConversionError::RobloxMeshParse(message.into())
}

fn fmt_ok(result: fmt::Result) -> Result<()> {
    result.map_err(|_| parse_err("failed to format OBJ output"))
}

