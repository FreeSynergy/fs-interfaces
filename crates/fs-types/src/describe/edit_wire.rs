//! `Edit` on the wire: `types.capnp` `Edit` and `FieldEdit`.
//!
//! Pattern: Adapter between the generated reader/builder and the plain structs, as `wire`.

use fs_interfaces::types_capnp::{BasicType, edit};

use super::{Edit, FieldEdit};

fn len(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

impl Edit {
    /// Write into a builder.
    pub fn write(&self, b: edit::Builder<'_>) {
        let mut list = b.init_fields(len(self.fields.len()));
        for (i, f) in (0..).zip(&self.fields) {
            let mut fb = list.reborrow().get(i);
            fb.set_basic(f.basic);
            let mut path = fb.reborrow().init_path(len(f.path.len()));
            for (j, s) in (0..).zip(&f.path) {
                path.set(j, s.as_str());
            }
            let mut values = fb.init_values(len(f.values.len()));
            for (j, s) in (0..).zip(&f.values) {
                values.set(j, s.as_str());
            }
        }
    }

    /// Read from a reader. A basic type this version does not know reads as `unknown`.
    ///
    /// # Errors
    /// A text that is not valid UTF-8 or a damaged message.
    pub fn read(r: edit::Reader<'_>) -> capnp::Result<Self> {
        let mut fields = Vec::new();
        for f in r.get_fields()? {
            let texts = |l: capnp::text_list::Reader<'_>| -> capnp::Result<Vec<String>> {
                l.iter().map(|t| Ok(t?.to_string()?)).collect()
            };
            fields.push(FieldEdit {
                path: texts(f.get_path()?)?,
                basic: f.get_basic().unwrap_or(BasicType::Unknown),
                values: texts(f.get_values()?)?,
            });
        }
        Ok(Self { fields })
    }
}
