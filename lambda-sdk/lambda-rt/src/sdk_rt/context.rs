use anyhow::Result as AnyResult;
use wasmtime::Linker;

use crate::Context;

const MODULE: &str = "sdk::io";

pub fn link_rt<Ctx>(linker: &mut Linker<Ctx>) -> AnyResult<()>
where
    Ctx: Context,
{
    linker.func_wrap(
        MODULE,
        "sender_username_length",
        implementation::sender_username_length::<_>,
    )?;

    linker.func_wrap(
        MODULE,
        "sender_username~32",
        implementation::sender_username::<_, u32>,
    )?;
    linker.func_wrap(
        MODULE,
        "sender_username~64",
        implementation::sender_username::<_, u64>,
    )?;

    Ok(())
}

mod implementation {
    use anyhow::{Context as _, Result as AnyResult};
    use wasmtime::Caller;

    use crate::{
        sdk_rt::utils::{self, NeverError, WasmUsize},
        Context, SdkEnv, User, INIT_ID,
    };

    // ALLOW: Required by "wasmtime" API.
    #[allow(clippy::needless_pass_by_value)]
    pub(super) fn sender_username_length<Ctx>(mut env: Caller<'_, Ctx>) -> u64
    where
        Ctx: Context,
    {
        let ctx: &mut SdkEnv<Ctx::Vault> = env.data_mut().sdk_mut();

        ctx.request_reader_id = if let Some(id) = ctx.request_reader_id.checked_add(1) {
            id
        } else {
            INIT_ID
        };

        ctx.request_reader_id.get()
    }

    pub(super) fn sender_username<Ctx, Usize>(
        mut env: Caller<'_, Ctx>,
        buffer_ptr: Usize,
        buffer_length: Usize,
        offset: u64,
    ) -> AnyResult<Usize>
    where
        Ctx: Context,
        Usize: WasmUsize,
    {
        let offset: usize = u64::into_usize(offset)?;

        utils::write_constant_to_memory(
            &mut env,
            |ctx: &Ctx| -> NeverError<_> {
                Ok(ctx
                    .sender()
                    .and_then(|user: &Ctx::User| user.username().as_bytes().get(..offset))
                    .unwrap_or(&[]))
            },
            buffer_ptr,
            buffer_length,
        )
        .context("Couldn't write request's data to memory!")
    }
}
