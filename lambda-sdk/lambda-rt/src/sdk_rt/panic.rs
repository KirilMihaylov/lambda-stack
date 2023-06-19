use anyhow::Result as AnyResult;
use wasmtime::Linker;

use crate::Context;

const MODULE: &str = "sdk::panic";

pub fn link_rt<Ctx>(linker: &mut Linker<Ctx>) -> AnyResult<()>
where
    Ctx: Context,
{
    linker.func_wrap(MODULE, "panic~32", implementation::panic::<_, u32>)?;
    linker.func_wrap(MODULE, "panic~64", implementation::panic::<_, u64>)?;

    Ok(())
}

mod implementation {
    use anyhow::Result as AnyResult;
    use wasmtime::Caller;

    use crate::{
        sdk_rt::utils::{self, WasmUsize},
        Context, ModuleResponse, PanickedError,
    };

    pub(super) fn panic<Ctx, Usize>(
        mut env: Caller<'_, Ctx>,
        buffer_ptr: Usize,
        buffer_length: Usize,
    ) -> AnyResult<()>
    where
        Ctx: Context,
        Usize: WasmUsize,
    {
        let response: Vec<u8> =
            utils::read_from_memory_to_buffer(&mut env, buffer_ptr, buffer_length)?;

        let module_response: &mut ModuleResponse = &mut env.data_mut().sdk_mut().response;

        module_response.data = response;

        module_response.is_error = true;

        Err(PanickedError.into())
    }
}
