//! `Image`: AS2 `Image`, a link with a media type `image/…` and, when known, a size in pixels.
//!
//! Pattern: Value object over the link rules of `iri`. The media type is never guessed from the file name; typed as one line it is a
//! link only (media type empty, "not known"). Reading the pixels of the file is not the job of a basic type.

use fs_interfaces::types_capnp::BasicType;

use crate::iri::{UNSAFE_FOR_IMAGE, checked, normalized};
use crate::{BasicValue, TypeProblem};

/// A picture reachable by a link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    uri: String,
    media_type: String,
    width: u32,
    height: u32,
}

fn media_type_ok(m: &str) -> bool {
    let Some(sub) = m.strip_prefix("image/") else {
        return false;
    };
    !sub.is_empty()
        && sub.len() <= 127
        && sub
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"!#$&-^_.+".contains(&b))
}

impl Image {
    /// Make an image. `media_type` may be empty (not known); `width` and `height` 0 mean not known.
    ///
    /// # Errors
    /// The errors of a link for `uri`; [`TypeProblem::NotImage`] for a media type that is not `image/<subtype>`.
    pub fn new(uri: &str, media_type: &str, width: u32, height: u32) -> Result<Self, TypeProblem> {
        let uri = normalized(checked(uri, &UNSAFE_FOR_IMAGE)?);
        let media_type = media_type.trim().to_ascii_lowercase();
        if !media_type.is_empty() && !media_type_ok(&media_type) {
            return Err(TypeProblem::NotImage);
        }
        Ok(Self {
            uri,
            media_type,
            width,
            height,
        })
    }

    /// The link.
    #[must_use]
    pub fn uri(&self) -> &str {
        &self.uri
    }

    /// The media type, empty when not known.
    #[must_use]
    pub fn media_type(&self) -> &str {
        &self.media_type
    }

    /// Width and height in pixels, 0 when not known.
    #[must_use]
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

impl BasicValue for Image {
    const KIND: BasicType = BasicType::Image;

    fn parse(typed: &str) -> Result<Self, TypeProblem> {
        Self::new(typed, "", 0, 0)
    }

    fn canonical(&self) -> String {
        self.uri.clone()
    }
}
