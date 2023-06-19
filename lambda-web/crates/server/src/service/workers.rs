use std::{collections::BTreeMap, sync::Arc};

use anyhow::{anyhow, Context as _, Result as AnyResult};
use tokio::sync::{mpsc::channel as mpsc_channel, Semaphore};

use lambda_rt::{Context as LambdaContext, LinkerWithSdk};

use crate::config::{
    Global as GlobalConfig, Id as ModuleId, Route as ConfigRoute, RoutePath as ConfigRoutePath,
};

use super::{
    modules::{spawn_module_worker, Precompiled as PrecompiledModules},
    RequestReceiver, RequestSender,
};

pub async fn generate_route_handlers<Ctx>(
    config: GlobalConfig,
    routes: Vec<ConfigRoute>,
    modules: PrecompiledModules,
    linker: Arc<LinkerWithSdk<Ctx>>,
) -> AnyResult<BTreeMap<ConfigRoutePath, RequestSender<Ctx::User>>>
where
    Ctx: LambdaContext<ConstructorContext = ()>,
    Ctx::Vault: Clone,
{
    let request_handlers_senders: BTreeMap<ModuleId, RequestSender<Ctx::User>> =
        generate_module_workers(config, modules, linker).await?;

    routes
        .into_iter()
        .try_fold(
            BTreeMap::new(),
            |mut acc: BTreeMap<ConfigRoutePath, RequestSender<Ctx::User>>,
             route: ConfigRoute| -> AnyResult<BTreeMap<ConfigRoutePath, RequestSender<Ctx::User>>> {
                request_handlers_senders
                    .get(&route.module)
                    .cloned()
                    .ok_or_else(
                        || anyhow!(
                            r#"Module with ID "{module}", required by route with path "{path}", not defined!"#,
                            module = route.module.0,
                            path = route.path.0,
                        )
                    )
                    .and_then(|request_handlers_sender: RequestSender<Ctx::User>| {
                        acc
                            .insert(route.path.clone(), request_handlers_sender)
                            .is_none()
                            .then_some(acc)
                            .ok_or_else(|| anyhow!(
                            r#"Route with path "{path}", serving module with ID "{module}", already defined!"#,
                            path = route.path.0,
                            module = route.module.0,
                        ))
                    })
            })
        .context("Failed to generate route handlers!")
}

async fn generate_module_workers<Ctx>(
    config: GlobalConfig,
    modules: PrecompiledModules,
    linker: Arc<LinkerWithSdk<Ctx>>,
) -> AnyResult<BTreeMap<ModuleId, RequestSender<Ctx::User>>>
where
    Ctx: LambdaContext<ConstructorContext = ()>,
    Ctx::Vault: Clone,
{
    let global_requests_semaphore: Arc<Semaphore> =
        Arc::new(Semaphore::new(config.requests.max_concurrent.get().into()));

    let mut module_workers: BTreeMap<ModuleId, RequestSender<Ctx::User>> = BTreeMap::new();

    for (module_id, module) in modules {
        let (sender, receiver): (RequestSender<Ctx::User>, RequestReceiver<Ctx::User>) =
            mpsc_channel(config.requests.max_concurrent.get().into());

        spawn_module_worker(
            receiver,
            linker.clone(),
            module,
            global_requests_semaphore.clone(),
            config.instances.init_pool_size.into(),
            config.instances.max_idle_pool_size.into(),
        )
        .await?;

        let maybe_sender: Option<RequestSender<Ctx::User>> =
            module_workers.insert(module_id, sender);

        debug_assert!(maybe_sender.is_none(), "Module ID repetition!");
    }

    Ok(module_workers)
}
