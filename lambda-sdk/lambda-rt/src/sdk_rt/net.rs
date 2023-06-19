use anyhow::Result as AnyResult;
use wasmtime::Linker;

use crate::Context;

const MODULE: &str = "sdk::net";

pub fn link_rt<Ctx>(linker: &mut Linker<Ctx>) -> AnyResult<()>
where
    Ctx: Context,
{
    linker.func_wrap1_async(
        MODULE,
        "send_request~32",
        implementation::send_request::<_, u32>,
    )?;
    linker.func_wrap1_async(
        MODULE,
        "send_request~64",
        implementation::send_request::<_, u64>,
    )?;

    linker.func_wrap(
        MODULE,
        "response_status_code",
        implementation::response_status_code::<_>,
    )?;

    linker.func_wrap(
        MODULE,
        "response_data_length",
        implementation::response_data_length::<_>,
    )?;

    linker.func_wrap(
        MODULE,
        "response_data~32",
        implementation::response_data::<_, u32>,
    )?;
    linker.func_wrap(
        MODULE,
        "response_data~64",
        implementation::response_data::<_, u64>,
    )?;

    linker.func_wrap(
        MODULE,
        "drop_some_response_data",
        implementation::drop_some_response_data::<_>,
    )?;

    linker.func_wrap(MODULE, "drop_response", implementation::drop_response::<_>)?;

    Ok(())
}

mod implementation {
    use std::future::Future;
    use std::slice::ChunksExact;

    use anyhow::{anyhow, bail, Context as _, Result as AnyResult};
    use reqwest::{
        header::{HeaderMap, HeaderName, HeaderValue},
        Body, Client, Method, Request, Response, Url,
    };
    use wasmtime::Caller;

    use crate::sdk_rt::utils::Size;
    use crate::{
        sdk_rt::utils::{self, RawValue, SlicePointer, WasmUsize},
        Context, Network, NetworkResponse, INIT_ID,
    };

    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    #[repr(packed, C)]
    struct HeaderData<Usize>
    where
        Usize: WasmUsize,
    {
        name: SlicePointer<Usize>,
        value: SlicePointer<Usize>,
    }

    impl<Usize> RawValue for HeaderData<Usize>
    where
        Usize: WasmUsize,
    {
        fn from_wasm_bytes(bytes: &[u8]) -> AnyResult<Self> {
            if dbg!(bytes.len()) != Self::SIZE {
                bail!("Slice of length different that the expected was provided!");
            }

            let mut chunks: ChunksExact<'_, u8> = bytes.chunks_exact(SlicePointer::<Usize>::SIZE);

            let name: SlicePointer<Usize> =
                SlicePointer::from_wasm_bytes(chunks.next().unwrap())
                    .context("Failed to deserialize `name` field from bytes!")?;
            let value: SlicePointer<Usize> = SlicePointer::from_wasm_bytes(chunks.next().unwrap())
                .context("Failed to deserialize `value` field from bytes!")?;

            debug_assert_eq!(chunks.remainder(), &[] as &[u8]);

            Ok(Self { name, value })
        }
    }

    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    #[repr(packed, C)]
    struct RequestData<Usize>
    where
        Usize: WasmUsize,
    {
        method: SlicePointer<Usize>,
        url: SlicePointer<Usize>,
        headers: SlicePointer<Usize>,
        body: SlicePointer<Usize>,
    }

    impl<Usize> RawValue for RequestData<Usize>
    where
        Usize: WasmUsize,
    {
        fn from_wasm_bytes(bytes: &[u8]) -> AnyResult<Self> {
            if bytes.len() != Self::SIZE {
                bail!("Slice of length different that the expected was provided!");
            }

            let mut chunks: ChunksExact<'_, u8> = bytes.chunks_exact(SlicePointer::<Usize>::SIZE);

            let method: SlicePointer<Usize> = SlicePointer::from_wasm_bytes(chunks.next().unwrap())
                .context("Failed to deserialize `method` field from bytes!")?;
            let url: SlicePointer<Usize> = SlicePointer::from_wasm_bytes(chunks.next().unwrap())
                .context("Failed to deserialize `url` field from bytes!")?;
            let headers: SlicePointer<Usize> =
                SlicePointer::from_wasm_bytes(chunks.next().unwrap())
                    .context("Failed to deserialize `headers` field from bytes!")?;
            let body: SlicePointer<Usize> =
                SlicePointer::from_wasm_bytes(chunks.next().unwrap())
                    .context("Failed to deserialize `body` field from bytes!")?;

            debug_assert_eq!(chunks.remainder(), &[] as &[u8]);

            Ok(Self {
                method,
                url,
                headers,
                body,
            })
        }
    }

    pub(super) fn send_request<Ctx, Usize>(
        mut env: Caller<'_, Ctx>,
        request_ptr: Usize,
    ) -> Box<dyn Future<Output = AnyResult<u64>> + Send + '_>
    where
        Ctx: Context,
        Usize: WasmUsize,
    {
        Box::new(async move {
            let request_data: RequestData<Usize> =
                utils::read_value_from_memory(&mut env, request_ptr)?;

            let method: Method = Method::from_bytes(&utils::read_from_memory_to_buffer(
                &mut env,
                request_data.method.pointer,
                request_data.method.length,
            )?)?;

            let url: String = String::from_utf8(utils::read_from_memory_to_buffer(
                &mut env,
                request_data.url.pointer,
                request_data.url.length,
            )?)?;

            let body: Vec<u8> = utils::read_from_memory_to_buffer(
                &mut env,
                request_data.body.pointer,
                request_data.body.length,
            )?;

            let mut request: Request = Request::new(method, Url::parse(&{ url })?);

            *request.body_mut() = Some(Body::from(body));

            {
                let headers: &mut HeaderMap<HeaderValue> = request.headers_mut();

                utils::read_value_array_from_memory::<_, _, HeaderData<Usize>>(
                    &mut env,
                    request_data.headers.pointer,
                    request_data.headers.length,
                )?
                .into_iter()
                .try_for_each(|header: HeaderData<Usize>| {
                    if headers
                        .insert(
                            HeaderName::try_from(utils::read_from_memory_to_buffer(
                                &mut env,
                                header.name.pointer,
                                header.name.length,
                            )?)?,
                            HeaderValue::try_from(utils::read_from_memory_to_buffer(
                                &mut env,
                                header.value.pointer,
                                header.value.length,
                            )?)?,
                        )
                        .is_none()
                    {
                        Ok(())
                    } else {
                        Err(anyhow!(
                            "Duplicated header name found while preparing request!"
                        ))
                    }
                })?;
            }

            let response: Response = Client::new().execute(request).await?;

            let network: &mut Network = &mut env.data_mut().sdk_mut().network;

            network.response = Some(NetworkResponse::new(
                response.status().as_u16(),
                response
                    .bytes()
                    .await
                    .context("Failed to fetch network response's data!")?
                    .into(),
            ));

            network.response_id = if let Some(id) = network.response_id.checked_add(1) {
                id
            } else {
                INIT_ID
            };

            Ok(network.response_id.get())
        })
    }

    pub(super) fn response_status_code<Ctx>(env: Caller<'_, Ctx>, id: u64) -> AnyResult<u32>
    where
        Ctx: Context,
    {
        let network: &Network = &env.data().sdk().network;

        if id != network.response_id.get() {
            bail!("Expected request data access ID didn't match provided one!");
        }

        network
            .response
            .as_ref()
            .map(|response| response.status_code().into())
            .ok_or_else(|| anyhow!("No response with such ID exists!"))
            .context("Failed to return network response's status code!")
    }

    pub(super) fn response_data_length<Ctx>(env: Caller<'_, Ctx>, id: u64) -> AnyResult<u64>
    where
        Ctx: Context,
    {
        let network: &Network = &env.data().sdk().network;

        if id != network.response_id.get() {
            bail!("Expected request data access ID didn't match provided one!");
        }

        network
            .response
            .as_ref()
            .map(|response| response.data().len())
            .ok_or_else(|| anyhow!("No response with such ID exists!"))
            .and_then(u64::from_usize)
    }

    pub(super) fn response_data<Ctx, Usize>(
        mut env: Caller<'_, Ctx>,
        id: u64,
        buffer_ptr: Usize,
        buffer_length: Usize,
    ) -> AnyResult<Usize>
    where
        Ctx: Context,
        Usize: WasmUsize,
    {
        let network: &Network = &env.data().sdk().network;

        if id != network.response_id.get() {
            bail!("Expected request data access ID didn't match provided one!");
        }

        utils::write_from_buffer_to_memory(
            &mut env,
            |ctx: &mut Ctx| {
                ctx.sdk_mut()
                    .network
                    .response
                    .as_mut()
                    .map(NetworkResponse::data_mut)
                    .ok_or_else(|| anyhow!("No response with such ID exists!"))
            },
            buffer_ptr,
            buffer_length,
        )
    }

    pub(super) fn drop_some_response_data<Ctx>(
        mut env: Caller<'_, Ctx>,
        id: u64,
        length: u64,
    ) -> AnyResult<()>
    where
        Ctx: Context,
    {
        let network: &mut Network = &mut env.data_mut().sdk_mut().network;

        if id != network.response_id.get() {
            bail!("Expected request data access ID didn't match provided one!");
        }

        let response: &mut Vec<u8> = network
            .response
            .as_mut()
            .ok_or_else(|| anyhow!("No response with such ID exists!"))
            .map(NetworkResponse::data_mut)?;

        let length: usize = length
            .into_usize()
            .context("Failed to convert length to drop into executor's native-width integer!")?;

        if length > response.len() {
            bail!("Response has less unread data than expected length to drop!");
        }

        *response = response.split_off(length);

        Ok(())
    }

    pub(super) fn drop_response<Ctx>(mut env: Caller<'_, Ctx>, id: u64) -> AnyResult<()>
    where
        Ctx: Context,
    {
        let network: &mut Network = &mut env.data_mut().sdk_mut().network;

        if id != network.response_id.get() {
            bail!("Expected request data access ID didn't match provided one!");
        }

        network
            .response
            .as_mut()
            .ok_or_else(|| anyhow!("No response with such ID exists!"))
            .map(NetworkResponse::data_mut)
            .map(|vec: &mut Vec<u8>| *vec = Vec::new())
    }
}
