use crate::bridged_type::StdLibType;

/// Primitive types such as `()`, `u8` and `bool`.
pub(crate) enum BuiltInPrimitive {
    Null,
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    U64,
    I64,
    Usize,
    Isize,
    F32,
    F64,
    Bool,
}

impl BuiltInPrimitive {
    /// U8 -> __private__OptionU8
    pub fn to_option_ffi_repr_name(&self) -> &'static str {
        match self {
            BuiltInPrimitive::Null => "__private__OptionNull",
            BuiltInPrimitive::U8 => "__private__OptionU8",
            BuiltInPrimitive::I8 => "__private__OptionI8",
            BuiltInPrimitive::U16 => "__private__OptionU16",
            BuiltInPrimitive::I16 => "__private__OptionI16",
            BuiltInPrimitive::U32 => "__private__OptionU32",
            BuiltInPrimitive::I32 => "__private__OptionI32",
            BuiltInPrimitive::U64 => "__private__OptionU64",
            BuiltInPrimitive::I64 => "__private__OptionI64",
            BuiltInPrimitive::Usize => "__private__OptionUsize",
            BuiltInPrimitive::Isize => "__private__OptionIsize",
            BuiltInPrimitive::F32 => "__private__OptionF32",
            BuiltInPrimitive::F64 => "__private__OptionF64",
            BuiltInPrimitive::Bool => "__private__OptionBool",
        }
    }

    /// TODO: Temporary... we can delete this when we delete `BridgedType::StdLib`.
    /// See: https://github.com/chinedufn/swift-bridge/issues/186
    pub fn new_with_stdlib_type(ty: &StdLibType) -> Option<Self> {
        let ty = match ty {
            StdLibType::Null => Self::Null,
            StdLibType::U8 => Self::U8,
            StdLibType::I8 => Self::I8,
            StdLibType::U16 => Self::U16,
            StdLibType::I16 => Self::I16,
            StdLibType::U32 => Self::U32,
            StdLibType::I32 => Self::I32,
            StdLibType::U64 => Self::U64,
            StdLibType::I64 => Self::I64,
            StdLibType::Usize => Self::Usize,
            StdLibType::Isize => Self::Isize,
            StdLibType::F32 => Self::F32,
            StdLibType::F64 => Self::F64,
            StdLibType::Bool => Self::Bool,
            _ => None?,
        };
        Some(ty)
    }
}
