//! The Windows transport against real pipes: the rights, the check of both ends, a caller that hangs up, many callers at once.
//! Pattern: Test (a user that is somebody else cannot be made here, so the check is tried on processes that are not ours).

use std::os::windows::io::AsRawHandle;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use windows_sys::Win32::Security::Authorization::{GetSecurityInfo, SE_KERNEL_OBJECT};
use windows_sys::Win32::Security::{
    ACL, ACL_SIZE_INFORMATION, AclSizeInformation, DACL_SECURITY_INFORMATION, GetAclInformation,
};

use super::super::endpoint::{Accepted, Endpoint};
use super::name::{companion_stem, encode, pipe_name};
use super::{Windows, bind_socket, sid};

fn unique(tag: &str) -> PathBuf {
    static COUNT: AtomicU32 = AtomicU32::new(0);
    PathBuf::from(format!(
        r"\\.\pipe\freesynergy-test-{tag}-{}-{}",
        std::process::id(),
        COUNT.fetch_add(1, Ordering::SeqCst)
    ))
}

#[test]
fn the_name_carries_the_sid_and_different_texts_never_meet() {
    let sid = sid::my_sid_text().unwrap();
    assert!(sid.starts_with("S-1-"));
    let a = pipe_name(&sid, std::path::Path::new("a b"));
    assert_ne!(a, pipe_name(&sid, std::path::Path::new("a_b")));
    assert_ne!(encode("A"), encode("a_"));
    assert_ne!(encode("Ab"), encode("ab"));
    let long = "x".repeat(400);
    assert!(encode(&long).len() <= 120);
    assert_ne!(encode(&long), encode(&format!("{long}y")));
    assert!(!companion_stem(&a).contains("pipe"));
}

#[test]
fn a_process_of_this_user_is_recognised_and_one_that_cannot_be_looked_at_is_not() {
    assert!(sid::process_is_me(std::process::id()));
    // 0 is the idle process, 4 the system: not this user.
    assert!(!sid::process_is_me(0));
    assert!(!sid::process_is_me(4));
}

#[tokio::test]
async fn the_pipe_is_open_to_this_user_alone_and_both_ends_check_the_other() {
    let path = unique("rights");
    let bound = bind_socket(&path).await.unwrap();
    // One entry in the list of rights: this user.
    let mut acl: *mut ACL = std::ptr::null_mut();
    let mut descriptor = std::ptr::null_mut();
    let handle = bound.next.lock().await.as_ref().unwrap().as_raw_handle();
    let code = unsafe {
        GetSecurityInfo(
            handle,
            SE_KERNEL_OBJECT,
            DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &raw mut acl,
            std::ptr::null_mut(),
            &raw mut descriptor,
        )
    };
    assert_eq!(code, 0);
    let mut size = ACL_SIZE_INFORMATION {
        AceCount: 0,
        AclBytesInUse: 0,
        AclBytesFree: 0,
    };
    let ok = unsafe {
        GetAclInformation(
            acl,
            (&raw mut size).cast(),
            u32::try_from(size_of::<ACL_SIZE_INFORMATION>()).unwrap(),
            AclSizeInformation,
        )
    };
    assert_ne!(ok, 0);
    assert_eq!(size.AceCount, 1);

    let (client, accepted) = tokio::join!(Windows::connect(&path), Windows::accept(&bound));
    let client = client.unwrap(); // the client found the server to be this user
    assert!(sid::server_is_me(client.as_raw_handle()));
    let Accepted::Peer(served) = accepted.unwrap() else {
        panic!("the caller is this user and must be served");
    };
    assert!(sid::client_is_me(served.as_raw_handle()));
}

#[tokio::test]
async fn a_caller_that_hangs_up_before_the_accept_does_not_end_the_server() {
    let path = unique("hangup");
    let bound = bind_socket(&path).await.unwrap();
    // Connects and hangs up before anybody accepted.
    drop(Windows::connect(&path).await.unwrap());
    // Skipped, or taken as a connection that is closed already: what matters is that it is no error that ends the server.
    let first = Windows::accept(&bound).await;
    assert!(first.is_ok(), "{:?}", first.err());
    // The next caller is served, on a new instance.
    let (client, accepted) = tokio::join!(Windows::connect(&path), Windows::accept(&bound));
    let mut client = client.unwrap();
    let Accepted::Peer(mut served) = accepted.unwrap() else {
        panic!("not served after a caller hung up");
    };
    client.write_all(b"x").await.unwrap();
    let mut byte = [0_u8; 1];
    served.read_exact(&mut byte).await.unwrap();
    assert_eq!(&byte, b"x");
}

#[tokio::test]
async fn many_callers_at_once_all_find_an_instance() {
    let path = unique("many");
    let bound = bind_socket(&path).await.unwrap();
    let clients = async {
        let all = (0..24).map(|_| Windows::connect(&path));
        futures::future::join_all(all).await
    };
    let accepting = async {
        let mut served = Vec::new();
        while served.len() < 24 {
            if let Accepted::Peer(s) = Windows::accept(&bound).await.unwrap() {
                served.push(s);
            }
        }
        served
    };
    let (results, served) = tokio::join!(clients, accepting);
    assert!(
        results.iter().all(Result::is_ok),
        "{:?}",
        results
            .iter()
            .filter_map(|c| c.as_ref().err())
            .collect::<Vec<_>>()
    );
    assert_eq!(served.len(), 24);
}

#[tokio::test]
async fn a_second_start_on_the_same_name_is_refused() {
    let path = unique("twice");
    let _first = bind_socket(&path).await.unwrap();
    let error = bind_socket(&path).await.err().unwrap();
    assert_eq!(error.kind(), std::io::ErrorKind::AddrInUse);
}

#[test]
fn the_data_folder_is_made_private_and_comes_back_without_the_prefix_of_the_long_form() {
    let dir = std::env::temp_dir().join(format!("fs-interfaces-data-{}", std::process::id()));
    let made = Windows::prepare_data_dir(&dir).unwrap();
    assert!(!made.to_string_lossy().starts_with(r"\\?\"));
    // Again, now that it exists: still ours.
    Windows::prepare_data_dir(&dir).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}
