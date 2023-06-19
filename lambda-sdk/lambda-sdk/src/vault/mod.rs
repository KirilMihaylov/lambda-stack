use std::num::NonZeroU64;

use anyhow::bail;

use crate::interops::{read_with_id, Pointer};

mod external;

pub struct Secret {
    id: NonZeroU64,
    length: u64,
}

impl Secret {
    pub fn fetch_secret(identifier: &str) -> anyhow::Result<Self> {
        let id: u64 = unsafe {
            external::fetch_secret(
                Pointer::from(identifier.as_bytes()).into(),
                identifier.len(),
            )
        };

        if let Some(id) = NonZeroU64::new(id) {
            Ok(Self {
                id,
                length: unsafe { external::secret_length(id) },
            })
        } else {
            bail!("No such secret with provided identifier exists!");
        }
    }

    pub fn unread_length(&self) -> u64 {
        self.length
    }

    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    #[inline]
    pub fn read<'r>(&'_ mut self, buf: &'r mut [u8]) -> &'r [u8] {
        read_with_id(
            external::read_secret,
            external::secret_length,
            self.id,
            &mut self.length,
            buf,
        )
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        unsafe { external::drop_secret(self.id) }
    }
}
