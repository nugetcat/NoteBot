/*
 * NoteBot Rust port: copyright is held by its respective contributors.
 * This file is distributed under GNU GPL version 3 only.
 */
use crate::{
    model::{Note, PALETTE},
    storage,
};
use java_serialization::{
    ArrayValues, ClassData, ClassDesc, ClassDescRef, FieldDesc, FieldValue, StreamObject,
    parse_serialization_stream,
};
use std::{collections::HashMap, fs, io, path::PathBuf};

const MAX_LEGACY_FILE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_NOTES: usize = 1_000;

pub fn load_if_available() -> io::Result<Option<Vec<Note>>> {
    let candidates = legacy_candidates()?;
    let mut last_error = None;
    let mut found_file = false;

    for path in candidates {
        match fs::metadata(&path) {
            Ok(metadata) => {
                found_file = true;
                if metadata.len() > MAX_LEGACY_FILE_BYTES {
                    last_error = Some(invalid_data(
                        "legacy sticky.dat exceeds the 32 MiB import limit",
                    ));
                    continue;
                }
                match fs::read(&path).and_then(|bytes| parse_legacy(&bytes)) {
                    Ok(notes) => return Ok(Some(notes)),
                    Err(error) => last_error = Some(error),
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => last_error = Some(error),
        }
    }

    if found_file {
        Err(last_error.unwrap_or_else(|| invalid_data("could not import legacy sticky.dat")))
    } else {
        Ok(None)
    }
}

fn legacy_candidates() -> io::Result<Vec<PathBuf>> {
    let directory = if cfg!(windows) {
        storage::data_directory()?
    } else {
        let dirs = directories::BaseDirs::new()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "home directory unavailable"))?;
        dirs.home_dir().join(".notebot")
    };

    Ok(["sticky.dat", "sticky.dat.bak", "sticky.dat.bak.2"]
        .into_iter()
        .map(|name| directory.join(name))
        .collect())
}

fn parse_legacy(bytes: &[u8]) -> io::Result<Vec<Note>> {
    let (remaining, stream) = parse_serialization_stream(bytes)
        .map_err(|error| invalid_data(format!("invalid Java serialization stream: {error}")))?;
    if !remaining.is_empty() || stream.version != 5 {
        return Err(invalid_data("unsupported or incomplete sticky.dat stream"));
    }

    let roots = stream
        .contents
        .iter()
        .map(|content| match content {
            java_serialization::ContentElement::Object(object) => Ok(object),
            java_serialization::ContentElement::BlockData(_) => {
                Err(invalid_data("unexpected raw data in sticky.dat"))
            }
        })
        .collect::<io::Result<Vec<_>>>()?;
    if roots.len() < 2 {
        return Err(invalid_data("sticky.dat is missing its header"));
    }

    let mut index = StreamIndex::default();
    for object in &roots {
        index.visit(object);
    }

    let saved_scale = root_float(roots[0], &index)
        .filter(|value| value.is_finite() && *value > 0.0)
        .ok_or_else(|| invalid_data("sticky.dat contains an invalid DPI scale"))?;
    let note_count = root_int(roots[1], &index)
        .and_then(|value| usize::try_from(value).ok())
        .filter(|count| *count <= MAX_NOTES)
        .ok_or_else(|| invalid_data("sticky.dat contains an invalid note count"))?;

    let records_end = 2usize
        .checked_add(
            note_count
                .checked_mul(4)
                .ok_or_else(|| invalid_data("note count overflow"))?,
        )
        .ok_or_else(|| invalid_data("note count overflow"))?;
    if roots.len() < records_end || roots.len() > records_end + note_count {
        return Err(invalid_data("sticky.dat has an incomplete note record"));
    }

    let mut notes = Vec::with_capacity(note_count);
    for index_in_file in 0..note_count {
        let offset = 2 + index_in_file * 4;
        let (x, y) = point_position(roots[offset], &index)?;
        let (width, height) = note_size(roots[offset + 1], &index, saved_scale)?;
        let color = color_scheme(roots[offset + 2], &index)?;
        let text = root_string(roots[offset + 3], &index)
            .ok_or_else(|| invalid_data("sticky.dat contains a non-text note"))?;

        let text_scale = if roots.len() > records_end + index_in_file {
            root_float(roots[records_end + index_in_file], &index)
                .filter(|value| value.is_finite())
                .unwrap_or(1.0)
                .clamp(0.2, 4.0)
        } else {
            1.0
        };

        notes.push(Note {
            id: index_in_file as u64 + 1,
            text,
            color: color.0,
            custom_color: color.1,
            text_scale,
            x: Some(x),
            y: Some(y),
            width: Some(width),
            height: Some(height),
        });
    }

    Ok(notes)
}

fn point_position(object: &StreamObject, index: &StreamIndex) -> io::Result<(f32, f32)> {
    let object = index
        .resolve(object)
        .ok_or_else(|| invalid_data("missing point object"))?;
    let x = named_int_field(object, "java.awt.Point", "x", index)
        .ok_or_else(|| invalid_data("sticky.dat point has no x coordinate"))?;
    let y = named_int_field(object, "java.awt.Point", "y", index)
        .ok_or_else(|| invalid_data("sticky.dat point has no y coordinate"))?;
    Ok((x as f32, y as f32))
}

fn note_size(
    object: &StreamObject,
    index: &StreamIndex,
    saved_scale: f32,
) -> io::Result<(f32, f32)> {
    let object = index
        .resolve(object)
        .ok_or_else(|| invalid_data("missing dimension object"))?;
    let width = named_int_field(object, "java.awt.Dimension", "width", index)
        .ok_or_else(|| invalid_data("sticky.dat dimension has no width"))?;
    let height = named_int_field(object, "java.awt.Dimension", "height", index)
        .ok_or_else(|| invalid_data("sticky.dat dimension has no height"))?;
    if width <= 0 || height <= 0 {
        return Err(invalid_data("sticky.dat contains a non-positive note size"));
    }
    Ok((
        (width as f32 / saved_scale).clamp(170.0, 8_192.0),
        (height as f32 / saved_scale).clamp(130.0, 8_192.0),
    ))
}

fn color_scheme(object: &StreamObject, index: &StreamIndex) -> io::Result<(usize, Option<String>)> {
    let object = index
        .resolve(object)
        .ok_or_else(|| invalid_data("missing color array"))?;
    let StreamObject::NewArray(array) = object else {
        return Err(invalid_data("sticky.dat color scheme is not an array"));
    };
    let ArrayValues::Object(colors) = &array.values else {
        return Err(invalid_data(
            "sticky.dat color scheme has an invalid component type",
        ));
    };
    let interior = colors
        .get(4)
        .and_then(Option::as_ref)
        .and_then(|color| index.resolve(color))
        .ok_or_else(|| invalid_data("sticky.dat color scheme has no interior color"))?;
    let argb = named_int_field(interior, "java.awt.Color", "value", index)
        .ok_or_else(|| invalid_data("sticky.dat interior color has no RGB value"))?
        as u32;
    let rgb = [
        ((argb >> 16) & 0xff) as u8,
        ((argb >> 8) & 0xff) as u8,
        (argb & 0xff) as u8,
    ];

    if let Some(palette_index) = PALETTE.iter().position(|entry| *entry == rgb) {
        Ok((palette_index, None))
    } else {
        Ok((
            0,
            Some(format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])),
        ))
    }
}

fn root_string(object: &StreamObject, index: &StreamIndex) -> Option<String> {
    match index.resolve(object)? {
        StreamObject::NewString(value) => Some(value.value.clone()),
        _ => None,
    }
}

fn root_float(object: &StreamObject, index: &StreamIndex) -> Option<f32> {
    first_field_value(index.resolve(object)?).and_then(|value| match value {
        FieldValue::Float(value) => Some(*value),
        _ => None,
    })
}

fn root_int(object: &StreamObject, index: &StreamIndex) -> Option<i32> {
    first_field_value(index.resolve(object)?).and_then(|value| match value {
        FieldValue::Int(value) => Some(*value),
        _ => None,
    })
}

fn first_field_value(object: &StreamObject) -> Option<&FieldValue> {
    let StreamObject::NewObject(object) = object else {
        return None;
    };
    object.class_data.iter().find_map(|data| match data {
        ClassData::NoWriteMethod(fields) | ClassData::WriteMethodWithFields(fields, _) => {
            fields.values.first()
        }
        _ => None,
    })
}

fn named_int_field(
    object: &StreamObject,
    class_name: &str,
    field_name: &str,
    index: &StreamIndex,
) -> Option<i32> {
    let StreamObject::NewObject(object) = index.resolve(object)? else {
        return None;
    };
    let class_index = index.class_data_index(object, class_name)?;
    let descriptor = index.class_desc(object, class_name)?;
    let field_index = descriptor.fields.iter().position(
        |field| matches!(field, FieldDesc::Primitive(field) if field.field_name == field_name),
    )?;
    let fields = match object.class_data.get(class_index)? {
        ClassData::NoWriteMethod(fields) | ClassData::WriteMethodWithFields(fields, _) => fields,
        _ => return None,
    };
    match fields.values.get(field_index)? {
        FieldValue::Int(value) => Some(*value),
        _ => None,
    }
}

#[derive(Default)]
struct StreamIndex {
    descriptors: HashMap<u32, java_serialization::NormalClassDesc>,
    objects: HashMap<u32, StreamObject>,
}

impl StreamIndex {
    fn resolve<'a>(&'a self, mut object: &'a StreamObject) -> Option<&'a StreamObject> {
        let mut depth = 0;
        while let StreamObject::PrevObject { handle } = object {
            object = self.objects.get(handle)?;
            depth += 1;
            if depth > self.objects.len() {
                return None;
            }
        }
        Some(object)
    }

    fn class_data_index(
        &self,
        object: &java_serialization::NewObject,
        target: &str,
    ) -> Option<usize> {
        let mut hierarchy = Vec::new();
        let mut desc_ref = &object.class_desc;
        loop {
            match desc_ref {
                ClassDescRef::Null => break,
                ClassDescRef::Inline(desc) => match desc.as_ref() {
                    ClassDesc::Normal(desc) => {
                        hierarchy.push(desc.class_name.clone());
                        desc_ref = &desc.super_class_desc;
                    }
                    ClassDesc::Proxy(_) => break,
                },
                ClassDescRef::Reference { handle } => {
                    let desc = self.descriptors.get(handle)?;
                    hierarchy.push(desc.class_name.clone());
                    desc_ref = &desc.super_class_desc;
                }
            }
        }
        hierarchy.reverse();
        hierarchy.iter().position(|name| name == target)
    }

    fn class_desc(
        &self,
        object: &java_serialization::NewObject,
        target: &str,
    ) -> Option<java_serialization::NormalClassDesc> {
        let mut desc_ref = &object.class_desc;
        loop {
            match desc_ref {
                ClassDescRef::Null => return None,
                ClassDescRef::Inline(desc) => match desc.as_ref() {
                    ClassDesc::Normal(desc) => {
                        if desc.class_name == target {
                            return Some((**desc).clone());
                        }
                        desc_ref = &desc.super_class_desc;
                    }
                    ClassDesc::Proxy(_) => return None,
                },
                ClassDescRef::Reference { handle } => {
                    let desc = self.descriptors.get(handle)?;
                    if desc.class_name == target {
                        return Some(desc.clone());
                    }
                    desc_ref = &desc.super_class_desc;
                }
            }
        }
    }

    fn visit(&mut self, object: &StreamObject) {
        if let Some(handle) = object.handle() {
            if !matches!(object, StreamObject::PrevObject { .. }) {
                self.objects.entry(handle).or_insert_with(|| object.clone());
            }
        }
        match object {
            StreamObject::NewObject(object) => {
                self.visit_desc_ref(&object.class_desc);
                for data in &object.class_data {
                    match data {
                        ClassData::NoWriteMethod(fields) => self.visit_fields(fields),
                        ClassData::WriteMethodWithFields(fields, annotation) => {
                            self.visit_fields(fields);
                            self.visit_annotation(annotation);
                        }
                        ClassData::WriteMethod(annotation)
                        | ClassData::ExternalBlockData(annotation) => {
                            self.visit_annotation(annotation)
                        }
                        ClassData::ExternalContents(_) => {}
                    }
                }
            }
            StreamObject::NewArray(array) => {
                self.visit_desc_ref(&array.class_desc);
                if let ArrayValues::Object(values) = &array.values {
                    for value in values.iter().flatten() {
                        self.visit(value);
                    }
                }
            }
            StreamObject::NewClass(class) => self.visit_desc_ref(&class.class_desc),
            StreamObject::NewEnum(value) => self.visit_desc_ref(&value.class_desc),
            StreamObject::NewClassDesc(desc) => self.visit_desc(desc),
            StreamObject::NewString(_)
            | StreamObject::PrevObject { .. }
            | StreamObject::NullReference
            | StreamObject::Exception
            | StreamObject::Reset => {}
        }
    }

    fn visit_fields(&mut self, fields: &java_serialization::FieldValueSet) {
        for value in &fields.values {
            if let FieldValue::Object(Some(object)) = value {
                self.visit(object);
            }
        }
    }

    fn visit_annotation(&mut self, annotation: &java_serialization::ObjectAnnotation) {
        for element in &annotation.contents {
            if let java_serialization::AnnotationElement::Object(object) = element {
                self.visit(object);
            }
        }
    }

    fn visit_desc_ref(&mut self, desc_ref: &ClassDescRef) {
        if let ClassDescRef::Inline(desc) = desc_ref {
            self.visit_desc(desc);
        }
    }

    fn visit_desc(&mut self, desc: &ClassDesc) {
        if let ClassDesc::Normal(desc) = desc {
            self.descriptors
                .entry(desc.handle)
                .or_insert_with(|| (**desc).clone());
            for element in &desc.class_annotation {
                if let java_serialization::AnnotationElement::Object(object) = element {
                    self.visit(object);
                }
            }
            self.visit_desc_ref(&desc.super_class_desc);
        }
    }
}

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}
