//! The delivery loop of a change feed.
//!
//! Pattern: Iterator consumer + State — normal, then after an overflow draining: the queue takes no
//! more events, what is in it still goes out, and a consumer that stops accepting ends it.

use std::future::Future;

use super::feed::{ChangeReceiver, FEED_STALL, Notice, write_change};
use crate::common_capnp::change_listener;

/// Deliver until the feed has to end; the answer says why.
pub(super) async fn pump<E, F, Fut>(
    feed: ChangeReceiver<E>,
    listener: &change_listener::Client,
    resolve: F,
) -> &'static str
where
    F: Fn(E) -> Fut,
    Fut: Future<Output = Option<Notice>>,
{
    let ChangeReceiver {
        mut events,
        overflow,
    } = feed;
    let mut overflowed = false;
    loop {
        let event = if overflowed {
            match events.try_recv() {
                Ok(event) => event,
                Err(_) => return BEHIND,
            }
        } else {
            tokio::select! {
                event = events.recv() => match event {
                    Some(event) => event,
                    None => return "the source closed",
                },
                () = overflow.notified() => {
                    overflowed = true;
                    events.close();
                    continue;
                }
            }
        };
        let Some(notice) = resolve(event).await else {
            continue;
        };
        let mut request = listener.change_request();
        write_change(request.get().init_change(), &notice);
        let sending = request.send();
        tokio::pin!(sending);
        if !overflowed {
            tokio::select! {
                sent = &mut sending => {
                    if sent.is_err() {
                        return FAILED;
                    }
                    continue;
                }
                () = overflow.notified() => {
                    overflowed = true;
                    events.close();
                }
            }
        }
        match tokio::time::timeout(FEED_STALL, &mut sending).await {
            Ok(Ok(())) => {}
            Ok(Err(_)) => return FAILED,
            Err(_) => return BEHIND,
        }
    }
}

const BEHIND: &str = "the consumer fell too far behind";
const FAILED: &str = "the listener failed";
