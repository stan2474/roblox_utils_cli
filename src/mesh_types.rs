#![allow(non_snake_case)]
#![allow(dead_code)]

#[derive(Debug, Clone, Copy)]
pub struct IntermediateVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

pub struct IntermediateMesh {
    pub vertices: Vec<IntermediateVertex>,
    pub faces: Vec<[u32; 3]>,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct FileMeshVertex {
    pub px: f32, pub py: f32, pub pz: f32,
    pub nx: f32, pub ny: f32, pub nz: f32,
    pub tu: f32, pub tv: f32,
    pub tx: i8,  pub ty: i8,  pub tz: i8, pub ts: i8,
    pub r: u8,   pub g: u8,   pub b: u8,  pub a: u8,
}

impl Default for FileMeshVertex {
    fn default() -> Self {
        Self {
            px: 0.0, py: 0.0, pz: 0.0,
            nx: 0.0, ny: 0.0, nz: 0.0,
            tu: 0.0, tv: 0.0,
            tx: 0, ty: 0, tz: -127, ts: 127,
            r: 255, g: 255, b: 255, a: 255,
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct FileMeshVertexNoRgba {
    pub px: f32, pub py: f32, pub pz: f32,
    pub nx: f32, pub ny: f32, pub nz: f32,
    pub tu: f32, pub tv: f32,
    pub tx: i8, pub ty: i8, pub tz: i8, pub ts: i8,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct FileMeshFace {
    pub a: u32,
    pub b: u32,
    pub c: u32,
}

#[repr(C, packed)]
pub struct FileMeshHeaderV2 {
    pub sizeof_FileMeshHeaderV2: u16,
    pub sizeof_FileMeshVertex: u8,
    pub sizeof_FileMeshFace: u8,
    pub numVerts: u32,
    pub numFaces: u32,
}

#[repr(C, packed)]
pub struct FileMeshHeaderV3 {
    pub sizeof_FileMeshHeaderV3: u16,
    pub sizeof_FileMeshVertex: u8,
    pub sizeof_FileMeshFace: u8,
    pub sizeof_LodOffset: u16,
    pub numLodOffsets: u16,
    pub numVerts: u32,
    pub numFaces: u32,
}

#[repr(C, packed)]
pub struct FileMeshHeaderV4 {
    pub sizeof_FileMeshHeaderV4: u16,
    pub lodType: u16,
    pub numVerts: u32,
    pub numFaces: u32,
    pub numLodOffsets: u16,
    pub numBones: u16,
    pub sizeof_boneNames: u32,
    pub numSubsets: u16,
    pub numHighQualityLODs: u8,
    pub unused: u8,
}

#[repr(C, packed)]
pub struct FileMeshHeaderV5 {
    pub sizeof_MeshHeader: u16,
    pub lodType: u16,
    pub numVerts: u32,
    pub numFaces: u32,
    pub numLodOffsets: u16,
    pub numBones: u16,
    pub sizeof_boneNameBuffer: u32,
    pub numSubsets: u16,
    pub numHighQualityLODs: u8,
    pub unusedPadding: u8,
    pub facsDataFormat: u32,
    pub facsDataSize: u32,
}
