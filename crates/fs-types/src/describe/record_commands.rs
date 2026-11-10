//! Which commands belong on a record.
//!
//! Pattern: Specification — the rule decided 2026-11-10 (written in `types.capnp`): the commands whose input holds a
//! reference to the type itself, plus `change` and `delete`; the rest belongs to the type as a whole.

use fs_interfaces::types_capnp::{BasicType, CommandKind};

use super::{CommandDescription, TypeDescription};

impl TypeDescription {
    /// The commands that work on one record, in the order of the description.
    #[must_use]
    pub fn record_commands(&self) -> Vec<&CommandDescription> {
        self.commands
            .iter()
            .filter(|c| self.works_on_a_record(c))
            .collect()
    }

    /// The commands of the type as a whole (create, find, check ...): the others.
    #[must_use]
    pub fn type_commands(&self) -> Vec<&CommandDescription> {
        self.commands
            .iter()
            .filter(|c| !self.works_on_a_record(c))
            .collect()
    }

    fn works_on_a_record(&self, c: &CommandDescription) -> bool {
        matches!(c.kind, CommandKind::Change | CommandKind::Delete)
            || c.input.iter().any(|f| {
                f.basic == BasicType::Reference && (f.target.is_empty() || f.target == self.name)
            })
    }
}
