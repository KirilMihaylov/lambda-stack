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
        "receive_request_data_id",
        implementation::receive_request_data_id::<_>,
    )?;

    linker.func_wrap(
        MODULE,
        "request_data_length",
        implementation::request_data_length::<_>,
    )?;

    linker.func_wrap(
        MODULE,
        "read_request_data~32",
        implementation::read_request_data::<_, u32>,
    )?;
    linker.func_wrap(
        MODULE,
        "read_request_data~64",
        implementation::read_request_data::<_, u64>,
    )?;

    linker.func_wrap(
        MODULE,
        "set_response_is_error",
        implementation::set_response_is_error,
    )?;

    linker.func_wrap(
        MODULE,
        "write_response_data~32",
        implementation::write_response_data::<_, u32>,
    )?;
    linker.func_wrap(
        MODULE,
        "write_response_data~64",
        implementation::write_response_data::<_, u64>,
    )?;

    Ok(())
}

mod implementation {
    use anyhow::{bail, Context as _, Result as AnyResult};
    use wasmtime::Caller;

    use crate::{
        sdk_rt::utils::{self, NeverError, WasmUsize},
        Context, SdkEnv, INIT_ID,
    };

    // ALLOW: Required by "wasmtime" API.
    #[allow(clippy::needless_pass_by_value)]
    pub(super) fn receive_request_data_id<Ctx>(mut env: Caller<'_, Ctx>) -> u64
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

    // ALLOW: Required by "wasmtime" API.
    #[allow(clippy::needless_pass_by_value)]
    pub(super) fn request_data_length<Ctx>(env: Caller<'_, Ctx>, id: u64) -> AnyResult<u64>
    where
        Ctx: Context,
    {
        let sdk: &SdkEnv<Ctx::Vault> = env.data().sdk();

        if id != sdk.request_reader_id.get() {
            bail!("Expected request data access ID didn't match provided one!");
        }

        u64::from_usize(sdk.request.len())
    }

    pub(super) fn read_request_data<Ctx, Usize>(
        mut env: Caller<'_, Ctx>,
        id: u64,
        buffer_ptr: Usize,
        buffer_length: Usize,
    ) -> AnyResult<Usize>
    where
        Ctx: Context,
        Usize: WasmUsize,
    {
        if id != env.data().sdk().request_reader_id.get() {
            bail!("Expected request data access ID didn't match provided one!");
        }

        utils::write_from_buffer_to_memory(
            &mut env,
            |ctx: &mut Ctx| -> NeverError<_> { Ok(&mut ctx.sdk_mut().request) },
            buffer_ptr,
            buffer_length,
        )
        .context("Couldn't write request's data to memory!")
    }

    pub(super) fn set_response_is_error<Ctx>(mut env: Caller<'_, Ctx>, id: u64) -> AnyResult<()>
    where
        Ctx: Context,
    {
        if id != env.data().sdk().request_reader_id.get() {
            bail!("Expected request data access ID didn't match provided one!");
        }

        env.data_mut().sdk_mut().response.is_error = true;

        Ok(())
    }

    pub(super) fn write_response_data<Ctx, Usize>(
        mut env: Caller<'_, Ctx>,
        buffer_ptr: Usize,
        buffer_length: Usize,
    ) -> AnyResult<()>
    where
        Ctx: Context,
        Usize: WasmUsize,
    {
        let mut buffer: Vec<u8> =
            utils::read_from_memory_to_buffer(&mut env, buffer_ptr, buffer_length)
                .context("Couldn't read response data from memory!")?;

        env.data_mut().sdk_mut().response.data.append(&mut buffer);

        Ok(())
    }
}
