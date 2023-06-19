use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use anyhow::{bail, Context as _, Result as AnyResult};
use tokio::{
    spawn,
    sync::{Mutex, MutexGuard, OwnedSemaphorePermit, Semaphore},
};
use wasmtime::{Engine, Module as WasmModule};

use lambda_rt::{Context as LambdaContext, LinkerWithSdk, SdkInstance, VerifiedModule};

use crate::config::{Id as ModuleId, Module as ConfigModule};

use super::{Request, RequestReceiver, ResponseSender};

pub type Precompiled = HashMap<ModuleId, VerifiedModule>;

pub fn precompile<Modules>(engine: &Engine, modules: Modules) -> AnyResult<Precompiled>
where
    Modules: IntoIterator<Item = ConfigModule>,
{
    modules
        .into_iter()
        .map(|module: ConfigModule| -> AnyResult<_> {
            Ok((
                module.id,
                VerifiedModule::new(WasmModule::from_file(&engine, module.path.into_inner())?)?,
            ))
        })
        .collect()
}

pub async fn spawn_module_worker<Ctx>(
    mut request_receiver: RequestReceiver<Ctx::User>,
    linker: Arc<LinkerWithSdk<Ctx>>,
    module: VerifiedModule,
    global_request_semaphore: Arc<Semaphore>,
    min_instances_pool_size: usize,
    max_instances_pool_size: usize,
) -> AnyResult<()>
where
    Ctx: LambdaContext<ConstructorContext = ()>,
    Ctx::Vault: Clone,
{
    let instance_pool: Arc<Mutex<VecDeque<SdkInstance<Ctx>>>> = Arc::new(Mutex::new({
        let mut deque: VecDeque<SdkInstance<Ctx>> =
            VecDeque::with_capacity(min_instances_pool_size);

        for _ in 0..min_instances_pool_size {
            deque.push_back(SdkInstance::new(&linker, &module, ()).await?);
        }

        deque
    }));

    drop(spawn(async move {
        while let Some(request) = request_receiver.recv().await {
            spawn_request_handling_task(
                linker.clone(),
                module.clone(),
                &global_request_semaphore,
                instance_pool.clone(),
                max_instances_pool_size,
                request,
            );
        }
    }));

    Ok(())
}

#[inline]
fn spawn_request_handling_task<Ctx>(
    linker: Arc<LinkerWithSdk<Ctx>>,
    module: VerifiedModule,
    global_request_semaphore: &Arc<Semaphore>,
    instance_pool: Arc<Mutex<VecDeque<SdkInstance<Ctx>>>>,
    max_instances_pool_size: usize,
    request: Request<Ctx::User>,
) where
    Ctx: LambdaContext<ConstructorContext = ()>,
    Ctx::Vault: Clone,
{
    const CONST_OK: anyhow::Result<()> = Ok(());

    // Safe to drop as future is managed by runtime.
    drop(spawn({
        let permit: Option<_> = request
            .externally_sourced
            .then(|| global_request_semaphore.clone().acquire_owned());

        async move {
            let permit: Option<OwnedSemaphorePermit> = if let Some(permit) = permit {
                Some(permit.await?)
            } else {
                None
            };

            if let Err(error) = handle_request(
                linker,
                module,
                instance_pool,
                request.response_sender,
                max_instances_pool_size,
                request.data,
                Some(request.user),
            )
            .await
            {
                println!(
                    "Error occurred! Context: {}; Root cause: {}",
                    error,
                    error.root_cause()
                );
            }

            // Explicitly drop permit to ensure proper drop order.
            drop(permit);

            CONST_OK
        }
    }));
}

async fn handle_request<Ctx>(
    linker: Arc<LinkerWithSdk<Ctx>>,
    module: VerifiedModule,
    instance_pool: Arc<Mutex<VecDeque<SdkInstance<Ctx>>>>,
    response_sender: ResponseSender,
    max_instances_pool_size: usize,
    request_data: Vec<u8>,
    sender: Option<Ctx::User>,
) -> AnyResult<()>
where
    Ctx: LambdaContext<ConstructorContext = ()>,
    Ctx::Vault: Clone,
{
    let mut instance: SdkInstance<Ctx> =
        if let Some(instance) = instance_pool.lock().await.pop_front() {
            instance
        } else {
            SdkInstance::new(&linker, &module, ())
                .await
                .context("Failed to create new module instance!")?
        };

    let Ok(()) = response_sender.send(instance.execute(request_data, sender).await) else {
        bail!("Failed to send response!");
    };

    let mut instance_pool_guard: MutexGuard<'_, VecDeque<SdkInstance<Ctx>>> =
        instance_pool.lock().await;

    if instance_pool_guard.len() < max_instances_pool_size {
        instance_pool_guard.push_back(instance);
    }

    drop(instance_pool_guard);

    Ok(())
}
