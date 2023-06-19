use anyhow::Result as AnyResult;
use wasmtime::Linker;

use crate::Context;

const MODULE: &str = "sdk::debug";

pub fn link_rt<Ctx>(linker: &mut Linker<Ctx>) -> AnyResult<()>
where
    Ctx: Context,
{
    linker.func_wrap(MODULE, "debug_str~32", implementation::debug_str::<_, u32>)?;
    linker.func_wrap(MODULE, "debug_str~64", implementation::debug_str::<_, u64>)?;

    Ok(())
}

mod implementation {
    use anyhow::{Context as _, Result as AnyResult};
    use wasmtime::Caller;

    use crate::{
        sdk_rt::utils::{self, WasmUsize},
        Context,
    };

    pub(super) fn debug_str<Ctx, Usize>(
        mut env: Caller<'_, Ctx>,
        buffer_ptr: Usize,
        buffer_length: Usize,
    ) -> AnyResult<()>
    where
        Ctx: Context,
        Usize: WasmUsize,
    {
        println!(
            "{}",
            String::from_utf8(
                utils::read_from_memory_to_buffer(&mut env, buffer_ptr, buffer_length)
                    .context("Couldn't read debug string from memory!")?
            )
            .context("Invalid UTF-8 encoded debug string passed to `debug_str`!")?
        );

        Ok(())
    }
}
