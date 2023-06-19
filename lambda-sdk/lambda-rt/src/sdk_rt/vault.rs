use anyhow::Result as AnyResult;
use wasmtime::Linker;

use crate::Context;

const MODULE: &str = "sdk::vault";

pub fn link_rt<Ctx>(linker: &mut Linker<Ctx>) -> AnyResult<()>
where
    Ctx: Context,
{
    linker.func_wrap2_async(
        MODULE,
        "fetch_secret~32",
        implementation::fetch_secret::<_, u32>,
    )?;
    linker.func_wrap2_async(
        MODULE,
        "fetch_secret~64",
        implementation::fetch_secret::<_, u64>,
    )?;

    linker.func_wrap(MODULE, "secret_length", implementation::secret_length::<_>)?;

    linker.func_wrap(
        MODULE,
        "read_secret~32",
        implementation::read_secret::<_, u32>,
    )?;
    linker.func_wrap(
        MODULE,
        "read_secret~64",
        implementation::read_secret::<_, u64>,
    )?;

    linker.func_wrap(MODULE, "drop_secret", implementation::drop_secret::<_>)?;

    Ok(())
}

mod implementation {
    use std::future::Future;

    use anyhow::{anyhow, bail, Context as _, Result as AnyResult};
    use wasmtime::Caller;

    use crate::{
        sdk_rt::utils::{self, WasmUsize},
        Context, VaultKeeper, VaultProvider, INIT_ID,
    };

    pub(super) fn fetch_secret<Ctx, Usize>(
        mut env: Caller<'_, Ctx>,
        identifier_ptr: Usize,
        identifier_length: Usize,
    ) -> Box<dyn Future<Output = AnyResult<u64>> + Send + '_>
    where
        Ctx: Context,
        Usize: WasmUsize,
    {
        Box::new(async move {
            if env.data().sdk().vault_keeper.secret.is_some() {
                bail!("Failed to fetch secret because previous secret response is not dropped.");
            }

            let identifier: String = String::from_utf8(utils::read_from_memory_to_buffer(
                &mut env,
                identifier_ptr,
                identifier_length,
            )?)?;

            let keeper: &mut VaultKeeper<Ctx::Vault> = &mut env.data_mut().sdk_mut().vault_keeper;

            keeper.secret = keeper
                .vault
                .fetch_secret(identifier)
                .await
                .context("Failed to fetch secret from vault provider!")?;

            if keeper.secret.is_none() {
                return Ok(0);
            }

            keeper.secret_id = if let Some(id) = keeper.secret_id.checked_add(1) {
                id
            } else {
                INIT_ID
            };

            Ok(keeper.secret_id.get())
        })
    }

    pub(super) fn secret_length<Ctx>(env: Caller<'_, Ctx>, id: u64) -> AnyResult<u64>
    where
        Ctx: Context,
    {
        let keeper: &VaultKeeper<Ctx::Vault> = &env.data().sdk().vault_keeper;

        if id != keeper.secret_id.get() {
            bail!("Expected secret access ID didn't match provided one!");
        }

        keeper
            .secret
            .as_deref()
            .map(Vec::len)
            .ok_or_else(|| anyhow!("No secret has been fetched!"))
            .and_then(u64::from_usize)
    }

    pub(super) fn read_secret<Ctx, Usize>(
        mut env: Caller<'_, Ctx>,
        id: u64,
        buffer_ptr: Usize,
        buffer_length: Usize,
    ) -> AnyResult<Usize>
    where
        Ctx: Context,
        Usize: WasmUsize,
    {
        if id != env.data().sdk().vault_keeper.secret_id.get() {
            bail!("Expected request data access ID didn't match provided one!");
        }

        utils::write_from_buffer_to_memory(
            &mut env,
            |ctx: &mut Ctx| {
                ctx.sdk_mut()
                    .vault_keeper
                    .secret
                    .as_deref_mut()
                    .ok_or_else(|| anyhow!("No secret has been fetched!"))
            },
            buffer_ptr,
            buffer_length,
        )
        .context("Couldn't write secret to memory!")
    }

    pub(super) fn drop_secret<Ctx>(mut env: Caller<'_, Ctx>, id: u64) -> AnyResult<()>
    where
        Ctx: Context,
    {
        let keeper: &mut VaultKeeper<Ctx::Vault> = &mut env.data_mut().sdk_mut().vault_keeper;

        if id != keeper.secret_id.get() {
            bail!("Expected secret access ID didn't match provided one!");
        }

        keeper.secret = None;

        Ok(())
    }
}
