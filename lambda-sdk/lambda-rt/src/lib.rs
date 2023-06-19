#![forbid(rust_2018_compatibility, deprecated_in_future)]
#![deny(rust_2021_compatibility, unsafe_code, warnings, clippy::pedantic)]

use std::{future::Future, mem::take, num::NonZeroU64};

use anyhow::{anyhow, bail, Result as AnyResult};
use wasmtime::{ExternType, Instance, Linker, Module, Store, TypedFunc};
use zeroize::Zeroizing;

use self::sdk_rt::link_rt;

mod sdk_rt;

#[derive(Debug, thiserror::Error)]
#[error("Panicked!")]
pub(crate) struct PanickedError;

pub trait User: Send {
    fn username(&self) -> &str;
}

pub trait VaultProvider: Send + Sync + 'static {
    type Result<'r>: Future<Output = AnyResult<Option<Zeroizing<Vec<u8>>>>> + Send + 'r;

    fn fetch_secret(&mut self, identifier: String) -> Self::Result<'_>;
}

pub trait Context: Send + 'static {
    type ConstructorContext;

    type Vault: VaultProvider;

    type User: User;

    fn with_vault_and_context(vault: Self::Vault, context: Self::ConstructorContext) -> Self
    where
        Self: Sized;

    fn sdk(&self) -> &SdkEnv<Self::Vault>;

    fn sdk_mut(&mut self) -> &mut SdkEnv<Self::Vault>;

    fn sender(&self) -> Option<&Self::User>;

    fn set_sender(&mut self, sender: Self::User);

    fn clear_sender(&mut self);
}

#[derive(Clone)]
pub struct LinkerWithSdk<Ctx>
where
    Ctx: Context,
{
    linker: Linker<Ctx>,
    vault: Ctx::Vault,
}

impl<Ctx> LinkerWithSdk<Ctx>
where
    Ctx: Context,
{
    /// Wraps [`wasmtime`]'s [`Linker`] while also linking SDK functions and
    /// sets cross-module call handler.
    /// # Errors
    /// Error may occur when linking SDK to [`wasmtime`]'s [`Linker`].
    pub fn new(mut linker: Linker<Ctx>, vault: Ctx::Vault) -> AnyResult<Self> {
        link_rt(&mut linker)?;

        Ok(Self { linker, vault })
    }
}

impl<Ctx> AsRef<Linker<Ctx>> for LinkerWithSdk<Ctx>
where
    Ctx: Context,
{
    fn as_ref(&self) -> &Linker<Ctx> {
        &self.linker
    }
}

impl<Ctx> AsMut<Linker<Ctx>> for LinkerWithSdk<Ctx>
where
    Ctx: Context,
{
    fn as_mut(&mut self) -> &mut Linker<Ctx> {
        &mut self.linker
    }
}

const INIT_ID: NonZeroU64 = if let Some(id) = NonZeroU64::new(1) {
    id
} else {
    panic!()
};

#[derive(Debug, Eq, PartialEq, Default)]
struct ModuleResponse {
    is_error: bool,
    data: Vec<u8>,
}

impl ModuleResponse {
    pub const fn new() -> Self {
        Self {
            is_error: false,
            data: Vec::new(),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Default)]
struct NetworkResponse {
    status_code: u16,
    data: Vec<u8>,
}

impl NetworkResponse {
    pub const fn new(status_code: u16, data: Vec<u8>) -> Self {
        Self { status_code, data }
    }

    pub const fn status_code(&self) -> u16 {
        self.status_code
    }

    pub const fn data(&self) -> &Vec<u8> {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut Vec<u8> {
        &mut self.data
    }
}

#[derive(Debug)]
struct Network {
    response_id: NonZeroU64,
    response: Option<NetworkResponse>,
}

impl Network {
    pub const fn new() -> Self {
        Self {
            response_id: INIT_ID,
            response: None,
        }
    }
}

#[derive(Debug)]
struct VaultKeeper<Vault>
where
    Vault: VaultProvider,
{
    vault: Vault,
    secret_id: NonZeroU64,
    secret: Option<Zeroizing<Vec<u8>>>,
}

impl<Vault> VaultKeeper<Vault>
where
    Vault: VaultProvider,
{
    pub const fn new(vault: Vault) -> Self {
        Self {
            vault,
            secret_id: INIT_ID,
            secret: None,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SdkUser {
    username: String,
}

impl SdkUser {
    pub const fn new(username: String) -> Self {
        Self { username }
    }
}

impl User for SdkUser {
    fn username(&self) -> &str {
        &self.username
    }
}

#[derive(Debug)]
pub struct SdkEnv<Vault>
where
    Vault: VaultProvider,
{
    request_reader_id: NonZeroU64,
    request: Vec<u8>,
    response: ModuleResponse,
    network: Network,
    vault_keeper: VaultKeeper<Vault>,
}

impl<Vault> SdkEnv<Vault>
where
    Vault: VaultProvider,
{
    pub const fn new(vault: Vault) -> Self {
        Self {
            request_reader_id: INIT_ID,
            request: Vec::new(),
            response: ModuleResponse::new(),
            network: Network::new(),
            vault_keeper: VaultKeeper::new(vault),
        }
    }

    pub fn set_request_data(&mut self, data: Vec<u8>) {
        self.request = data;
        self.request.shrink_to_fit();
    }

    pub fn clear_request_data(&mut self) {
        self.request = vec![];
    }
}

#[derive(Debug)]
pub struct SdkContext<Vault>
where
    Vault: VaultProvider,
{
    env: SdkEnv<Vault>,
    sender: Option<SdkUser>,
}

impl<Vault> SdkContext<Vault>
where
    Vault: VaultProvider,
{
    pub const fn new(env: SdkEnv<Vault>) -> Self {
        Self { env, sender: None }
    }
}

impl<Vault> From<SdkEnv<Vault>> for SdkContext<Vault>
where
    Vault: VaultProvider,
{
    fn from(env: SdkEnv<Vault>) -> Self {
        Self::new(env)
    }
}

impl<Vault> Context for SdkContext<Vault>
where
    Vault: VaultProvider + Sync,
{
    type ConstructorContext = ();

    type Vault = Vault;

    type User = SdkUser;

    fn with_vault_and_context(vault: Vault, (): ()) -> Self
    where
        Self: Sized,
    {
        Self::new(SdkEnv::new(vault))
    }

    fn sdk(&self) -> &SdkEnv<Vault> {
        &self.env
    }

    fn sdk_mut(&mut self) -> &mut SdkEnv<Vault> {
        &mut self.env
    }

    fn sender(&self) -> Option<&SdkUser> {
        self.sender.as_ref()
    }

    fn set_sender(&mut self, sender: SdkUser) {
        self.sender = Some(sender);
    }

    fn clear_sender(&mut self) {
        self.sender = None;
    }
}

#[derive(Debug, Clone)]
pub enum Response {
    Success(Vec<u8>),
    Error(Vec<u8>),
}

#[derive(Clone)]
pub struct VerifiedModule(Module);

impl VerifiedModule {
    /// Verifies that module exports all required symbols and also verifies
    /// their type.
    /// # Errors
    /// Error will occur when module doesn't export all required symbols and/or
    /// their type doesn't match expected one.
    pub fn new(module: Module) -> AnyResult<Self> {
        if !matches!(
            module.get_export("memory").ok_or_else(|| anyhow!(
                r#"Module doesn't contain any memory exported as "memory"!"#
            ))?,
            ExternType::Memory(_)
        ) {
            bail!(r#"Exported symbol "memory" is not a memory!"#);
        }

        let Some(entry): Option<ExternType> = module.get_export("entry") else {
            bail!(r#"Module doesn't contain entry point exported as "entry"!"#)
        };

        let ExternType::Func(func) = entry else {
            bail!(r#"Exported symbol "memory" is not a memory!"#)
        };

        if func.params().count() != 0 {
            bail!("Entry point takes parameters as opposed to not taking any!")
        }

        if func.results().count() != 0 {
            bail!("Entry point returns value as opposed to not returning any!")
        }

        Ok(Self(module))
    }
}

pub enum StaticModuleVerificationError {
    NoMemoryExport,
    InvalidMemoryExport,
    NoEntryPoint,
}

#[must_use]
pub struct SdkInstance<Ctx>
where
    Ctx: Context,
{
    store: Store<Ctx>,
    entry: TypedFunc<(), ()>,
}

impl<Ctx> SdkInstance<Ctx>
where
    Ctx: Context,
{
    /// Creates a new instance with linked SDK that is ready for use.
    /// # Errors
    /// Error will occur when [`Linker`] fails to create instance.
    #[inline]
    pub async fn new(
        linker: &LinkerWithSdk<Ctx>,
        module: &VerifiedModule,
        constructor_context: Ctx::ConstructorContext,
    ) -> AnyResult<Self>
    where
        Ctx::Vault: Clone,
    {
        Self::internal_new(
            &linker.linker,
            Ctx::with_vault_and_context(linker.vault.clone(), constructor_context),
            &module.0,
        )
        .await
    }

    /// Creates a new instance with linked SDK that is ready for use.
    /// # Errors
    /// Error will occur when [`Linker`] fails to create instance.
    #[inline]
    pub async fn consuming_new(
        linker: LinkerWithSdk<Ctx>,
        module: &VerifiedModule,
        constructor_context: Ctx::ConstructorContext,
    ) -> AnyResult<Self> {
        Self::internal_new(
            &linker.linker,
            Ctx::with_vault_and_context(linker.vault, constructor_context),
            &module.0,
        )
        .await
    }

    async fn internal_new(linker: &Linker<Ctx>, context: Ctx, module: &Module) -> AnyResult<Self> {
        let mut store: Store<Ctx> = Store::new(linker.engine(), context);

        let instance: Instance = linker.instantiate_async(&mut store, module).await?;

        let Ok(entry): AnyResult<TypedFunc<(), ()>> = instance.get_typed_func(&mut store, "entry") else {
            unreachable!()
        };

        Ok(Self { store, entry })
    }

    pub async fn execute(
        &mut self,
        data: Vec<u8>,
        sender: Option<Ctx::User>,
    ) -> AnyResult<Response> {
        debug_assert_eq!(
            &self.store.data().sdk().response,
            &ModuleResponse::new(),
            "Response field is dirty before execution of request!"
        );

        let context: &mut Ctx = self.store.data_mut();

        context.sdk_mut().set_request_data(data);

        if let Some(sender) = sender {
            context.set_sender(sender);
        }

        if let Err(error) = self.entry.call_async(&mut self.store, ()).await {
            let _: PanickedError = error.downcast()?;
        }

        let context: &mut Ctx = self.store.data_mut();

        context.sdk_mut().clear_request_data();

        context.clear_sender();

        debug_assert!(context.sender().is_none());

        let response: ModuleResponse = take(&mut self.store.data_mut().sdk_mut().response);

        Ok(if response.is_error {
            Response::Error
        } else {
            Response::Success
        }(response.data))
    }
}
