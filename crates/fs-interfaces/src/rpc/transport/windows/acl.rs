//! The rights of the pipe and of the data folder: this user alone. Pattern: Factory (a pipe instance with its descriptor, a
//! folder made private) + RAII ([`Descriptor`]).

use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr::{null, null_mut};

use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use windows_sys::Win32::Foundation::LocalFree;
use windows_sys::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, GetNamedSecurityInfoW, SDDL_REVISION_1,
    SE_FILE_OBJECT, SetNamedSecurityInfoW,
};
use windows_sys::Win32::Security::{
    ACL, DACL_SECURITY_INFORMATION, GetSecurityDescriptorDacl, OWNER_SECURITY_INFORMATION,
    PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID, SECURITY_ATTRIBUTES,
};

use super::sid::{me, my_sid_text};

/// A security descriptor the system allocated.
struct Descriptor(PSECURITY_DESCRIPTOR);

impl Drop for Descriptor {
    fn drop(&mut self) {
        unsafe { LocalFree(self.0) };
    }
}

fn wide(text: &std::ffi::OsStr) -> Vec<u16> {
    text.encode_wide().chain(std::iter::once(0)).collect()
}

/// A protected descriptor with one entry: full access for this user, `flags` as inheritance (`OICI` for a folder).
fn only_me(flags: &str) -> io::Result<Descriptor> {
    let sddl = format!("D:P(A;{flags};FA;;;{})", my_sid_text()?);
    let mut out: PSECURITY_DESCRIPTOR = null_mut();
    let ok = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            wide(sddl.as_ref()).as_ptr(),
            SDDL_REVISION_1,
            &raw mut out,
            null_mut(),
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(Descriptor(out))
}

fn code(error: u32) -> io::Result<()> {
    if error == 0 {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(
            i32::try_from(error).unwrap_or(i32::MAX),
        ))
    }
}

/// One instance of the pipe `name`, open to this user alone (not to the system, not to administrators: the standard rights
/// would let others in). `first` is the rule that decides who owns the name.
pub(super) fn create_pipe(name: &str, first: bool) -> io::Result<NamedPipeServer> {
    let descriptor = only_me("")?;
    let mut attributes = SECURITY_ATTRIBUTES {
        nLength: u32::try_from(size_of::<SECURITY_ATTRIBUTES>()).unwrap_or(0),
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: 0,
    };
    let mut options = ServerOptions::new();
    options
        .first_pipe_instance(first)
        .reject_remote_clients(true);
    // The descriptor outlives the call; the system copies it.
    unsafe { options.create_with_security_attributes_raw(name, (&raw mut attributes).cast()) }
}

/// Is the owner of `dir` this user (or the owner a token of this user gives to what it makes)?
fn owned_by_me(dir: &Path) -> io::Result<bool> {
    let me = me()?;
    let (mut owner, mut descriptor): (PSID, PSECURITY_DESCRIPTOR) = (null_mut(), null_mut());
    code(unsafe {
        GetNamedSecurityInfoW(
            wide(dir.as_os_str()).as_ptr(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            &raw mut owner,
            null_mut(),
            null_mut(),
            null_mut(),
            &raw mut descriptor,
        )
    })?;
    let descriptor = Descriptor(descriptor);
    let mine = me.user.equals(owner) || me.owner.equals(owner);
    drop(descriptor);
    Ok(mine)
}

/// Create `dir` (and what is above), refuse one that another user owns, and leave rights for this user alone: nothing inherited.
///
/// # Errors
/// Another user owns it (`PermissionDenied`), or the system refuses.
pub(super) fn make_private(dir: &Path) -> io::Result<()> {
    std::fs::create_dir_all(dir)?;
    if !owned_by_me(dir)? {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("{} belongs to another user", dir.display()),
        ));
    }
    let descriptor = only_me("OICI")?;
    let (mut present, mut defaulted, mut acl): (i32, i32, *mut ACL) = (0, 0, null_mut());
    if unsafe {
        GetSecurityDescriptorDacl(
            descriptor.0,
            &raw mut present,
            &raw mut acl,
            &raw mut defaulted,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    code(unsafe {
        SetNamedSecurityInfoW(
            wide(dir.as_os_str()).as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            acl,
            null(),
        )
    })
}
