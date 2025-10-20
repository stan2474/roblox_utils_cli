use clap::{Parser, Subcommand, ValueEnum};
use std::{fs, path::PathBuf};
use std::collections::HashMap;
use rbx_dom_weak::{WeakDom, Ustr, InstanceBuilder};
use rbx_types::Variant;
use rbx_binary::{from_reader, to_writer};
use rbx_xml::{from_reader_default, to_writer_default};
use chrono::Utc;
use std::io::Cursor;
use std::error::Error;
use rbx_types::Content;
mod error;
mod filemesh;
mod importer;
mod mesh_types;
mod ser;

#[derive(ValueEnum, Clone, Copy, Debug)]
enum RobloxMeshVersion {
    V1_00,
    V1_01,
    V2_00,
    V3_00,
    V4_00,
    V5_00,
}

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    ObjToFilemesh {
        input: PathBuf,
        output: PathBuf,
        #[arg(value_enum, default_value_t = RobloxMeshVersion::V2_00)]
        version: RobloxMeshVersion,
    },
    FilemeshToObj {
        input: PathBuf,
        output: PathBuf,
    },
    FixPlace {
        input: PathBuf,
        output: PathBuf,
        #[arg(long)]
        folders_to_models: bool,
        #[arg(long)]
        convert_meshparts: bool,
        #[arg(long)]
        force_xml: bool,
        #[arg(long)]
        force_binary: bool,
        #[arg(long)]
        convert_assetid_to_url: bool,
        #[arg(long, default_value = "http://www.roblox.com/asset/?id=")]
        asset_url_format: String,
        #[arg(long)]
        instance_mappings_file: Option<PathBuf>,
    },
}

fn is_binary_rbxl(bytes: &[u8]) -> bool {
    const MAGIC: [u8; 16] = [
        0x3C, 0x72, 0x6F, 0x62, 0x6C, 0x6F, 0x78, 0x21,
        0x89, 0xFF, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00,
    ];
    bytes.starts_with(&MAGIC)
}

fn convert_obj_to_filemesh(obj_data: &[u8], version: RobloxMeshVersion) -> error::Result<Vec<u8>> {
    let mesh = importer::obj_to_intermediate(obj_data)?;
    let bytes = match version {
        RobloxMeshVersion::V1_00 => ser::write_v1(&mesh, ser::V1Version::V1_00)?,
        RobloxMeshVersion::V1_01 => ser::write_v1(&mesh, ser::V1Version::V1_01)?,
        RobloxMeshVersion::V2_00 => ser::write_v2(&mesh)?,
        RobloxMeshVersion::V3_00 => ser::write_v3(&mesh)?,
        RobloxMeshVersion::V4_00 => ser::write_v4(&mesh)?,
        RobloxMeshVersion::V5_00 => ser::write_v5(&mesh)?,
    };
    Ok(bytes)
}

fn convert_filemesh_to_obj(filemesh_data: &[u8]) -> error::Result<Vec<u8>> {
    filemesh::filemesh_to_obj_bytes(filemesh_data)
}

const LEGACY_FONT_SIZE_OPTIONS: [(i64, u32); 10] = [
    (8, 0), (9, 1), (10, 2), (11, 3), (12, 4),
    (14, 5), (18, 6), (24, 7), (36, 8), (48, 9),
];
const FONT_SIZE_COMPATIBILITY: [(u32, u32); 5] = [(10, 7), (11, 8), (12, 9), (13, 9), (14, 9)];

fn font_size_name_from_value(value: u32) -> &'static str {
    match value {
        0 => "Size8", 1 => "Size9", 2 => "Size10", 3 => "Size11", 4 => "Size12",
        5 => "Size14", 6 => "Size18", 7 => "Size24", 8 => "Size36", 9 => "Size48",
        10 => "Size28", 11 => "Size32", 12 => "Size42", 13 => "Size60", 14 => "Size96",
        _ => "Unknown",
    }
}

fn normalize_font_size_value(value: u32) -> u32 {
    FONT_SIZE_COMPATIBILITY
        .iter()
        .find(|&&(modern, _)| value == modern)
        .map_or(value, |&(_, legacy)| legacy)
}

fn font_enum_from_text_size(text_size: i64) -> u32 {
    LEGACY_FONT_SIZE_OPTIONS
        .iter()
        .min_by_key(|&&(size, _)| (text_size - size).abs())
        .map(|&(_, enum_val)| enum_val)
        .unwrap_or(0)
}

fn apply_instance_conversions(
    dom: &mut WeakDom,
    folders_to_models: bool,
    mappings: &HashMap<Ustr, Ustr>,
    convert_assetid_to_url: bool,
    asset_url_format: &str,
    convert_meshpart_to_specialmesh: bool,
) {
    let instance_refs: Vec<_> = dom.descendants().map(|instance| instance.referent()).collect();
    let text_size_key: Ustr = "TextSize".into();
    let font_size_key: Ustr = "FontSize".into();
    let folder_class: Ustr = "Folder".into();
    let model_class: Ustr = "Model".into();
    let meshpart_class: Ustr = "MeshPart".into();
    let part_class: Ustr = "Part".into();

    for instance_ref in instance_refs {
        let mut pending_special_mesh: Option<(InstanceBuilder, String, rbx_dom_weak::types::Vector3)> = None;

        if let Some(instance) = dom.get_by_ref_mut(instance_ref) {
            if let Some(new_class) = mappings.get(&instance.class) {
                println!(
                    "[legacy_place::convert] mapped instance '{}' from {} to {}",
                    instance.name, instance.class, new_class
                );
                instance.class = *new_class;
            }
            if instance.class == meshpart_class && convert_meshpart_to_specialmesh {
                let initial_size = match instance.properties.get(&"InitialSize".into()) {
                    Some(Variant::Vector3(v)) => *v,
                    _ => {
                        println!(
                            "[legacy_place::convert] meshpart '{}' missing initialsize property, skipping conversion",
                            instance.name
                        );
                        continue;
                    },
                };
                let size = match instance.properties.get(&"Size".into()) {
                    Some(Variant::Vector3(v)) => *v,
                    _ => {
                        println!(
                            "[legacy_place::convert] meshpart '{}' missing size property, skipping conversion",
                            instance.name
                        );
                        continue;
                    },
                };
                if initial_size.x == 0.0 || initial_size.y == 0.0 || initial_size.z == 0.0 {
                    println!(
                        "[legacy_place::convert] meshpart '{}' has zero initialsize, skipping conversion",
                        instance.name
                    );
                    continue;
                }
                let scale = rbx_dom_weak::types::Vector3 {
                    x: size.x / initial_size.x,
                    y: size.y / initial_size.y,
                    z: size.z / initial_size.z,
                };
                instance.class = part_class;
                let mesh_id = instance.properties.get(&"MeshId".into()).unwrap_or(&Variant::Content(Content::from_uri(String::new()))).clone();
                let instance_name = instance.name.clone();
                let special_mesh = InstanceBuilder::new("SpecialMesh")
                    .with_name("Mesh")
                    .with_property("Scale", Variant::Vector3(scale))
                    .with_property("MeshType", Variant::Enum(rbx_dom_weak::types::Enum::from_u32(5)))
                    .with_property("MeshId", mesh_id);
                pending_special_mesh = Some((special_mesh, instance_name, scale));
            }

            if folders_to_models && instance.class == folder_class {
                println!(
                    "[legacy_place::convert] converted folder '{}' to model",
                    instance.name
                );
                instance.class = model_class;
            }
            if instance.class == "KeyframeSequence" {
                instance.class = "Part".into();
                println!("[legacy_place::convert] converted keyframesequence '{}' to part to avoid errors in old clients", instance.name);
            }

            if instance.class == "UnionOperation" {
                println!("[legacy_place::convert] reading MeshData2 for unionoperation '{}'", instance.name);
                let mesh_data_variant = instance.properties.get(&"PhysicalConfigData".into()).cloned();
                println!("mesh_data_variant: {:?}", mesh_data_variant);
            }

            let mut font_size_to_add: Option<Variant> = None;
            let mut props_to_update: Vec<(Ustr, Variant)> = Vec::new();

            for (prop_name, prop_value) in &instance.properties {
                if *prop_name == text_size_key {
                    let text_size_opt = match prop_value {
                        Variant::Int64(val) => Some(*val),
                        Variant::Int32(val) => Some(*val as i64),
                        Variant::Float32(val) => Some(*val as i64),
                        Variant::Float64(val) => Some(*val as i64),
                        _ => None,
                    };
                    if let Some(text_size) = text_size_opt {
                        let enum_value = normalize_font_size_value(font_enum_from_text_size(text_size));
                        font_size_to_add = Some(Variant::Enum(rbx_dom_weak::types::Enum::from_u32(enum_value)));
                        println!(
                            "[legacy_place::convert] converted TextSize {} on '{}' to FontSize {}",
                            text_size,
                            instance.name,
                            font_size_name_from_value(enum_value)
                        );
                    } else {
                        println!(
                            "[legacy_place::convert] textsize on '{}' has unexpected type: {:?}",
                            instance.name,
                            prop_value
                        );
                    }
                }

                if convert_assetid_to_url {
                    if let Variant::Content(content) = prop_value {
                        if let Some(uri) = content.as_uri() {
                            if let Some(id_part) = uri.strip_prefix("rbxassetid://") {
                                if id_part.parse::<u64>().is_ok() {
                                    let new_url = format!("{}{}", asset_url_format, id_part);
                                    println!(
                                        "[legacy_place::convert] converting asset ID on '{}', property '{}' changed to {}",
                                        instance.name, prop_name, new_url
                                    );
                                    props_to_update.push((*prop_name, Variant::Content(Content::from_uri(new_url))));
                                }
                            }
                        }
                    }
                }
            }

            if let Some(font_size) = font_size_to_add {
                instance.properties.insert(font_size_key, font_size);
                instance.properties.remove(&text_size_key);
            }

            for (prop_name, new_value) in props_to_update {
                instance.properties.insert(prop_name, new_value);
            }
        }
        if convert_meshpart_to_specialmesh {
            if let Some((special_mesh, instance_name, scale)) = pending_special_mesh {
                dom.insert(instance_ref, special_mesh);
                println!("[legacy_place::convert] converted meshpart '{}' -> part + specialmesh scale=({}, {}, {})",
                    instance_name, scale.x, scale.y, scale.z
                );
            }
        }
    }
}

fn load_instance_mappings(path: &PathBuf) -> Result<HashMap<Ustr, Ustr>, Box<dyn Error>> {
    let data = fs::read_to_string(path)?;
    let raw: HashMap<String, String> = serde_json::from_str(&data)?;
    Ok(raw.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
}

fn fix_place(
    input_bytes: &[u8],
    force_xml_output: bool,
    force_binary_output: bool,
    folders_to_models: bool,
    convert_assetid_to_url: bool,
    asset_url_format: String,
    convert_meshpart_to_specialmesh: bool,
    instance_mappings: Option<HashMap<Ustr, Ustr>>,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let start = Utc::now();
    let is_binary_input = is_binary_rbxl(input_bytes);
    let mut reader = Cursor::new(input_bytes);
    let mut dom: WeakDom = if is_binary_input {
        from_reader(&mut reader).map_err(|e| Box::<dyn Error>::from(e.to_string()))?
    } else {
        from_reader_default(&mut reader).map_err(|e| Box::<dyn Error>::from(e.to_string()))?
    };
    let mappings = instance_mappings.unwrap_or_default();
    apply_instance_conversions(
        &mut dom,
        folders_to_models,
        &mappings,
        convert_assetid_to_url,
        &asset_url_format,
        convert_meshpart_to_specialmesh,
    );
    let root_refs: Vec<_> = dom.root().children().to_vec();
    let mut output = Vec::new();
    let should_output_xml = (!is_binary_input && !force_binary_output) || force_xml_output;
    if should_output_xml {
        to_writer_default(&mut output, &dom, &root_refs).map_err(|e| Box::<dyn Error>::from(e.to_string()))?;
    } else {
        to_writer(&mut output, &dom, &root_refs).map_err(|e| Box::<dyn Error>::from(e.to_string()))?;
    }
    let end = Utc::now();
    let elapsed = end.signed_duration_since(start);
    println!("done in {} ms", elapsed.num_milliseconds());
    Ok(output)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Commands::ObjToFilemesh { input, output, version } => {
            let obj_data = fs::read(input)?;
            let bytes = convert_obj_to_filemesh(&obj_data, version)?;
            fs::write(output, bytes)?;
        }
        Commands::FilemeshToObj { input, output } => {
            let data = fs::read(input)?;
            let bytes = convert_filemesh_to_obj(&data)?;
            fs::write(output, bytes)?;
        }
        Commands::FixPlace {
            input,
            output,
            folders_to_models,
            convert_meshparts,
            force_xml,
            force_binary,
            convert_assetid_to_url,
            asset_url_format,
            instance_mappings_file,
        } => {
            let data = fs::read(input)?;
            let mappings = if let Some(path) = instance_mappings_file {
                Some(load_instance_mappings(&path)?)
            } else { None };
            let out = fix_place(
                &data,
                force_xml,
                force_binary,
                folders_to_models,
                convert_assetid_to_url,
                asset_url_format,
                convert_meshparts,
                mappings,
            )?;
            fs::write(output, out)?;
        }
    }
    Ok(())
}
