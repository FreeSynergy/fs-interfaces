//! Descriptions on the wire: `types.capnp` ⇄ Rust.
//!
//! Pattern: Adapter between the generated reader/builder types and the plain structs. A kind or command kind this
//! version does not know reads as `unknown` (the schema's promise); a text that is not UTF-8 is an error.

use capnp::struct_list;
use fs_interfaces::types_capnp::{
    BasicType, CommandKind, command_description, field_description, type_description,
};

use super::{CommandDescription, FieldDescription, TypeDescription, TypeRegistry};

fn write_field(f: &FieldDescription, mut b: field_description::Builder<'_>) {
    b.set_name(&f.name);
    b.set_basic(f.basic);
    b.set_required(f.required);
    b.set_multiple(f.multiple);
    b.set_read_only(f.read_only);
    b.set_label(&f.label);
    b.set_target(&f.target);
    let mut list = b.init_choices(u32::try_from(f.choices.len()).unwrap_or(u32::MAX));
    for (i, c) in (0..).zip(&f.choices) {
        list.set(i, c.as_str());
    }
}

fn read_field(r: field_description::Reader<'_>) -> capnp::Result<FieldDescription> {
    let mut choices = Vec::new();
    for c in r.get_choices()? {
        choices.push(c?.to_string()?);
    }
    Ok(FieldDescription {
        name: r.get_name()?.to_string()?,
        basic: r.get_basic().unwrap_or(BasicType::Unknown),
        read_only: r.get_read_only(),
        required: r.get_required(),
        multiple: r.get_multiple(),
        label: r.get_label()?.to_string()?,
        target: r.get_target()?.to_string()?,
        choices,
    })
}

fn write_fields(
    fields: &[FieldDescription],
    mut list: struct_list::Builder<'_, field_description::Owned>,
) {
    for (i, f) in (0..).zip(fields) {
        write_field(f, list.reborrow().get(i));
    }
}

fn read_fields(
    list: struct_list::Reader<'_, field_description::Owned>,
) -> capnp::Result<Vec<FieldDescription>> {
    list.iter().map(read_field).collect()
}

fn len(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

impl TypeDescription {
    /// Write into a builder.
    pub fn write(&self, mut b: type_description::Builder<'_>) {
        b.set_name(&self.name);
        b.set_program(&self.program);
        b.set_standard(&self.standard);
        b.set_label(&self.label);
        write_fields(
            &self.fields,
            b.reborrow().init_fields(len(self.fields.len())),
        );
        let mut cmds = b.init_commands(len(self.commands.len()));
        for (i, c) in (0..).zip(&self.commands) {
            let mut cb = cmds.reborrow().get(i);
            cb.set_name(&c.name);
            cb.set_kind(c.kind);
            cb.set_label(&c.label);
            cb.set_supported(c.supported);
            write_fields(&c.input, cb.reborrow().init_input(len(c.input.len())));
            write_fields(&c.result, cb.init_result(len(c.result.len())));
        }
    }

    /// Read from a reader.
    ///
    /// # Errors
    /// A text that is not valid UTF-8 or a damaged message.
    pub fn read(r: type_description::Reader<'_>) -> capnp::Result<Self> {
        let mut commands = Vec::new();
        for c in r.get_commands()? {
            commands.push(read_command(c)?);
        }
        Ok(Self {
            name: r.get_name()?.to_string()?,
            program: r.get_program()?.to_string()?,
            standard: r.get_standard()?.to_string()?,
            label: r.get_label()?.to_string()?,
            fields: read_fields(r.get_fields()?)?,
            commands,
        })
    }
}

fn read_command(c: command_description::Reader<'_>) -> capnp::Result<CommandDescription> {
    Ok(CommandDescription {
        name: c.get_name()?.to_string()?,
        kind: c.get_kind().unwrap_or(CommandKind::Unknown),
        label: c.get_label()?.to_string()?,
        supported: c.get_supported(),
        input: read_fields(c.get_input()?)?,
        result: read_fields(c.get_result()?)?,
    })
}

impl TypeRegistry {
    /// Write all descriptions (the answer of `TypeCatalog.describe`).
    pub fn write(&self, mut list: struct_list::Builder<'_, type_description::Owned>) {
        for (i, t) in (0..).zip(self.iter()) {
            t.write(list.reborrow().get(i));
        }
    }

    /// Read a list of descriptions into a registry, validating each.
    ///
    /// # Errors
    /// A damaged message, or a description that does not validate (the defect is the message of the error).
    pub fn read(list: struct_list::Reader<'_, type_description::Owned>) -> capnp::Result<Self> {
        let mut reg = Self::new();
        for t in list {
            reg.add_read(TypeDescription::read(t)?)
                .map_err(|p| capnp::Error::failed(p.to_string()))?;
        }
        Ok(reg)
    }
}
