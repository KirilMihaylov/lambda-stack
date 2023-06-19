use anyhow::Result as AnyResult;
use wasmtime::Linker;

use super::Context;

mod context;
mod debug;
mod io;
mod net;
mod panic;
mod vault;

pub fn link_rt<Ctx>(linker: &mut Linker<Ctx>) -> AnyResult<()>
where
    Ctx: Context,
{
    context::link_rt(linker)?;
    debug::link_rt(linker)?;
    io::link_rt(linker)?;
    net::link_rt(linker)?;
    panic::link_rt(linker)?;
    vault::link_rt(linker)?;

    Ok(())
}

mod utils {
    use std::{
        any::type_name,
        array::from_fn,
        fmt::Debug,
        mem::{replace, size_of, take},
        ops::Add,
    };

    use anyhow::{anyhow, bail, Context as _, Error as AnyError, Result as AnyResult};
    use wasmtime::{Caller, Extern, Memory};

    use crate::Context;

    pub(super) enum Never {}

    impl From<Never> for AnyError {
        fn from(value: Never) -> Self {
            match value {}
        }
    }

    pub(super) type NeverError<T> = Result<T, Never>;

    pub(super) trait Size
    where
        Self: Sized,
    {
        const SIZE: usize;
    }

    impl<T> Size for T {
        const SIZE: usize = size_of::<Self>();
    }

    macro_rules! impl_raw_value {
        ($($type: ty)+) => {
            $(
                impl RawValue for $type {
                    fn from_wasm_bytes(bytes: &[u8]) -> AnyResult<Self> {
                        if bytes.len() != Self::SIZE {
                            bail!("Slice of length different that the expected was provided!");
                        }

                        Ok(Self::from_le_bytes(from_fn(|index: usize| bytes[index])))
                    }
                }
            )+
        };
    }

    pub(super) trait RawValue
    where
        Self: Copy + Eq + Send + Sync + Size + 'static,
    {
        fn from_wasm_bytes(bytes: &[u8]) -> AnyResult<Self>;
    }

    impl_raw_value!(i8 u8 i16 u16 i32 u32 i64 u64 i128 u128);

    pub(super) trait WasmUsize:
        Debug + Eq + PartialEq + Add<Self, Output = Self> + Copy + Send + Sync + RawValue + 'static
    {
        const ZERO: Self;

        fn into_usize(self) -> AnyResult<usize>;

        fn from_usize(_: usize) -> AnyResult<Self>;
    }

    impl WasmUsize for u32 {
        const ZERO: Self = 0;

        fn into_usize(self) -> AnyResult<usize> {
            self.try_into()
                .context("Couldn't convert WASM32 integer to executor's native-width integer!")
        }

        fn from_usize(value: usize) -> AnyResult<Self> {
            value
                .try_into()
                .context("Couldn't convert executor's native-width integer to WASM32 integer!")
        }
    }

    impl WasmUsize for u64 {
        const ZERO: Self = 0;

        fn into_usize(self) -> AnyResult<usize> {
            self.try_into()
                .context("Couldn't convert WASM64 integer to executor's native-width integer!")
        }

        fn from_usize(value: usize) -> AnyResult<Self> {
            value
                .try_into()
                .context("Couldn't convert executor's native-width integer to WASM64 integer!")
        }
    }

    #[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
    #[repr(packed, C)]
    pub(super) struct Pointer<Usize>
    where
        Usize: WasmUsize,
    {
        pub pointer: Usize,
    }

    impl<Usize> RawValue for Pointer<Usize>
    where
        Usize: WasmUsize,
    {
        fn from_wasm_bytes(bytes: &[u8]) -> AnyResult<Self> {
            Usize::from_wasm_bytes(bytes).map(|pointer: Usize| Self { pointer })
        }
    }

    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    #[repr(packed, C)]
    pub(super) struct SlicePointer<Usize>
    where
        Usize: WasmUsize,
    {
        pub pointer: Usize,
        pub length: Usize,
    }

    impl<Usize> RawValue for SlicePointer<Usize>
    where
        Usize: WasmUsize,
    {
        fn from_wasm_bytes(bytes: &[u8]) -> AnyResult<Self> {
            if bytes.len() != Usize::SIZE << 1 {
                bail!("Slice of length different that the expected was provided!");
            }

            let pointer: Usize = Usize::from_wasm_bytes(&bytes[..Usize::SIZE])
                .context("Failed to deserialize `pointer` field from bytes!")?;
            let length: Usize = Usize::from_wasm_bytes(&bytes[Usize::SIZE..])
                .context("Failed to deserialize `length` field from bytes!")?;

            Ok(Self { pointer, length })
        }
    }

    pub(super) fn get_memory<Ctx>(env: &mut Caller<'_, Ctx>) -> AnyResult<Memory>
    where
        Ctx: Context,
    {
        env.get_export("memory")
            .and_then(Extern::into_memory)
            .ok_or_else(|| anyhow!("Memory not exposed by module!"))
    }

    pub(super) fn write_constant_to_memory<Ctx, SourceFn, SourceFnError, Usize>(
        env: &mut Caller<'_, Ctx>,
        source_fn: SourceFn,
        buffer_ptr: Usize,
        buffer_length: Usize,
    ) -> AnyResult<Usize>
    where
        Ctx: Context,
        SourceFn: FnOnce(&Ctx) -> Result<&[u8], SourceFnError>,
        AnyError: From<SourceFnError>,
        Usize: WasmUsize,
    {
        let memory: Memory = get_memory(env)?;

        write_constant_to_memory_with(env, &memory, source_fn, buffer_ptr, buffer_length)
    }

    pub(super) fn write_constant_to_memory_with<Ctx, SourceFn, SourceFnError, Usize>(
        env: &mut Caller<'_, Ctx>,
        memory: &'_ Memory,
        source_fn: SourceFn,
        buffer_ptr: Usize,
        buffer_length: Usize,
    ) -> AnyResult<Usize>
    where
        Ctx: Context,
        SourceFn: FnOnce(&Ctx) -> Result<&[u8], SourceFnError>,
        AnyError: From<SourceFnError>,
        Usize: WasmUsize,
    {
        if buffer_length == Usize::ZERO {
            return Ok(Usize::ZERO);
        }

        let usize_buffer_length: usize = buffer_length.into_usize().unwrap_or(usize::MAX);

        let source: Vec<u8> = {
            let source: &[u8] = source_fn(env.data())?;

            Vec::from(if usize_buffer_length < source.len() {
                &source[..usize_buffer_length]
            } else {
                &*source
            })
        };

        memory
            .write(env, buffer_ptr.into_usize()?, &source)
            .context("Couldn't write down request data to memory!")?;

        Usize::from_usize(source.len()).map_err(Into::into)
    }

    pub(super) fn write_from_buffer_to_memory<Ctx, SourceFn, SourceFnError, Usize>(
        env: &mut Caller<'_, Ctx>,
        source_fn: SourceFn,
        buffer_ptr: Usize,
        buffer_length: Usize,
    ) -> AnyResult<Usize>
    where
        Ctx: Context,
        SourceFn: FnOnce(&mut Ctx) -> Result<&mut Vec<u8>, SourceFnError>,
        AnyError: From<SourceFnError>,
        Usize: WasmUsize,
    {
        let memory: Memory = get_memory(env)?;

        write_from_buffer_to_memory_with(env, &memory, source_fn, buffer_ptr, buffer_length)
    }

    pub(super) fn write_from_buffer_to_memory_with<Ctx, SourceFn, SourceFnError, Usize>(
        env: &mut Caller<'_, Ctx>,
        memory: &Memory,
        source_fn: SourceFn,
        buffer_ptr: Usize,
        buffer_length: Usize,
    ) -> AnyResult<Usize>
    where
        Ctx: Context,
        SourceFn: FnOnce(&mut Ctx) -> Result<&mut Vec<u8>, SourceFnError>,
        AnyError: From<SourceFnError>,
        Usize: WasmUsize,
    {
        if buffer_length == Usize::ZERO {
            return Ok(Usize::ZERO);
        }

        let usize_buffer_length: usize = buffer_length.into_usize().unwrap_or(usize::MAX);

        let source: Vec<u8> = {
            let buffer: &mut Vec<u8> = source_fn(env.data_mut())?;

            if usize_buffer_length < buffer.len() {
                let rest_of_data: Vec<u8> = buffer.split_off(usize_buffer_length);

                replace(buffer, rest_of_data)
            } else {
                take(buffer)
            }
        };

        memory
            .write(env, buffer_ptr.into_usize()?, &source)
            .context("Couldn't write down request data to memory!")?;

        Usize::from_usize(source.len()).map_err(Into::into)
    }

    pub(super) fn read_from_memory_to_buffer<Ctx, Usize>(
        env: &mut Caller<'_, Ctx>,
        buffer_ptr: Usize,
        buffer_length: Usize,
    ) -> AnyResult<Vec<u8>>
    where
        Ctx: Context,
        Usize: WasmUsize,
    {
        let memory: Memory = get_memory(env)?;

        read_from_memory_to_buffer_with(env, &memory, buffer_ptr, buffer_length)
    }

    pub(super) fn read_from_memory_to_buffer_with<Ctx, Usize>(
        env: &mut Caller<'_, Ctx>,
        memory: &Memory,
        buffer_ptr: Usize,
        buffer_length: Usize,
    ) -> AnyResult<Vec<u8>>
    where
        Ctx: Context,
        Usize: WasmUsize,
    {
        let mut buffer: Vec<u8> = vec![0; buffer_length.into_usize()?];

        memory
            .read(env, buffer_ptr.into_usize()?, &mut buffer)
            .context("Couldn't read data from memory!")?;

        Ok(buffer)
    }

    pub(super) fn read_value_from_memory<Ctx, Usize, Value>(
        env: &mut Caller<'_, Ctx>,
        buffer_ptr: Usize,
    ) -> AnyResult<Value>
    where
        Ctx: Context,
        Usize: WasmUsize,
        Value: RawValue,
    {
        let memory: Memory = get_memory(env)?;

        read_value_from_memory_with(env, &memory, buffer_ptr).context(format!(
            "Failed to deserialize `{}` from bytes!",
            type_name::<Value>()
        ))
    }

    pub(super) fn read_value_from_memory_with<Ctx, Usize, Value>(
        env: &mut Caller<'_, Ctx>,
        memory: &Memory,
        buffer_ptr: Usize,
    ) -> AnyResult<Value>
    where
        Ctx: Context,
        Usize: WasmUsize,
        Value: RawValue,
    {
        let mut buffer: Vec<u8> = vec![0; Value::SIZE];

        memory
            .read(env, buffer_ptr.into_usize()?, &mut buffer)
            .context("Couldn't read data from memory!")?;

        Value::from_wasm_bytes(&buffer)
    }

    pub(super) fn read_value_array_from_memory<Ctx, Usize, Value>(
        env: &mut Caller<'_, Ctx>,
        buffer_ptr: Usize,
        value_count: Usize,
    ) -> AnyResult<Vec<Value>>
    where
        Ctx: Context,
        Usize: WasmUsize,
        Value: RawValue,
    {
        let memory: Memory = get_memory(env)?;

        read_value_array_from_memory_with(env, &memory, buffer_ptr, value_count)
    }

    pub(super) fn read_value_array_from_memory_with<Ctx, Usize, Value>(
        env: &mut Caller<'_, Ctx>,
        memory: &Memory,
        buffer_ptr: Usize,
        value_count: Usize,
    ) -> AnyResult<Vec<Value>>
    where
        Ctx: Context,
        Usize: WasmUsize,
        Value: RawValue,
    {
        (0..value_count.into_usize()?).try_fold(Vec::new(), |mut acc: Vec<Value>, index: usize| {
            read_value_from_memory_with(
                env,
                memory,
                buffer_ptr + Usize::from_usize(Value::SIZE * index)?,
            )
            .map(|value: Value| {
                acc.push(value);

                acc
            })
        })
    }
}
