use std::{
    fmt::{Debug, Formatter},
    marker::PhantomData,
};

use rjvm_reader::field_type::BaseType;
use rjvm_utils::type_conversion::ToUsizeSafe;

use crate::{
    array_entry_type::ArrayEntryType, native_methods_impl::array_copy, object::Object,
    value::Value, vm_error::VmError,
};

// Memory layout:
//   first we have 4 bytes with the length
//   then we have the data
// Similary to [Object], we store each value in 8 bytes. This means that we waste quite a bit of
// memory for types that would fit in 32 bits (int or float) or even fewer (bool, byte), but
// whatever. We don't do efficiency :)
#[derive(PartialEq, Clone)]
pub struct Array<'a> {
    data: *mut u8,
    marker: PhantomData<&'a [u8]>,
}

const HEADER_LEN: usize = std::mem::size_of::<u32>() + std::mem::size_of::<ArrayEntryType>();

impl<'a> Array<'a> {
    pub(crate) fn size(length: usize) -> usize {
        HEADER_LEN + length * 8
    }

    pub fn new(elements_type: ArrayEntryType, length: usize, ptr: *mut u8) -> Self {
        unsafe {
            let header_ptr = ptr as *mut u32;
            std::ptr::write(header_ptr, length as u32);

            let header_ptr = header_ptr.add(1) as *mut ArrayEntryType;
            std::ptr::write(header_ptr, elements_type);
        };

        Self {
            data: ptr,
            marker: PhantomData::default(),
        }
    }

    pub fn len(&self) -> u32 {
        unsafe {
            let header_ptr = self.data as *mut u32;
            std::ptr::read(header_ptr)
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get_elements_type(&self) -> ArrayEntryType {
        unsafe {
            let header_ptr = self.data as *const u32;
            let header_ptr = header_ptr.add(1) as *const ArrayEntryType;
            std::ptr::read(header_ptr)
        }
    }

    pub fn get_item_at(&self, index: usize) -> Result<Value<'a>, VmError> {
        if index >= self.len().into_usize_safe() {
            Err(VmError::ArrayIndexOutOfBoundsException)
        } else {
            unsafe {
                let ptr = self.data.add(HEADER_LEN).add(index * 8);
                Ok(match self.get_elements_type() {
                    ArrayEntryType::Base(BaseType::Boolean)
                    | ArrayEntryType::Base(BaseType::Byte)
                    | ArrayEntryType::Base(BaseType::Char)
                    | ArrayEntryType::Base(BaseType::Short)
                    | ArrayEntryType::Base(BaseType::Int) => {
                        Value::Int(std::ptr::read(ptr as *const i32))
                    }
                    ArrayEntryType::Base(BaseType::Long) => {
                        Value::Long(std::ptr::read(ptr as *const i64))
                    }
                    ArrayEntryType::Base(BaseType::Float) => {
                        Value::Float(std::ptr::read(ptr as *const f32))
                    }
                    ArrayEntryType::Base(BaseType::Double) => {
                        Value::Double(std::ptr::read(ptr as *const f64))
                    }
                    ArrayEntryType::Object(_) => match std::ptr::read(ptr as *const i64) {
                        0 => Value::Null,
                        _ => Value::Object(std::ptr::read(ptr as *const Object)),
                    },
                    ArrayEntryType::Array => Value::Array(std::ptr::read(ptr as *const Array)),
                })
            }
        }
    }

    pub fn set_item_at(&self, index: usize, value: Value<'a>) -> Result<(), VmError> {
        if index >= self.len().into_usize_safe() {
            Err(VmError::ArrayIndexOutOfBoundsException)
        } else {
            unsafe {
                let ptr = self.data.add(HEADER_LEN).add(index * 8);
                match self.get_elements_type() {
                    ArrayEntryType::Base(BaseType::Boolean)
                    | ArrayEntryType::Base(BaseType::Byte)
                    | ArrayEntryType::Base(BaseType::Char)
                    | ArrayEntryType::Base(BaseType::Short)
                    | ArrayEntryType::Base(BaseType::Int) => match value {
                        Value::Int(int) => std::ptr::write(ptr as *mut i32, int),
                        _ => return Err(VmError::ValidationException),
                    },
                    ArrayEntryType::Base(BaseType::Long) => match value {
                        Value::Long(long) => std::ptr::write(ptr as *mut i64, long),
                        _ => return Err(VmError::ValidationException),
                    },
                    ArrayEntryType::Base(BaseType::Float) => match value {
                        Value::Float(float) => std::ptr::write(ptr as *mut f32, float),
                        _ => return Err(VmError::ValidationException),
                    },
                    ArrayEntryType::Base(BaseType::Double) => match value {
                        Value::Double(double) => std::ptr::write(ptr as *mut f64, double),
                        _ => return Err(VmError::ValidationException),
                    },
                    ArrayEntryType::Object(_) => match value {
                        Value::Object(object) => std::ptr::write(ptr as *mut Object, object),
                        Value::Null => std::ptr::write(ptr as *mut i64, 0),
                        _ => return Err(VmError::ValidationException),
                    },
                    ArrayEntryType::Array => match value {
                        Value::Array(array) => std::ptr::write(ptr as *mut Array, array),
                        _ => return Err(VmError::ValidationException),
                    },
                };
                Ok(())
            }
        }
    }

    // TODO: impl eq
    pub fn is_same_as(&self, other: &Array) -> bool {
        self.data == other.data
    }

    pub fn copy_from(&self, other: &Array) -> Result<(), VmError> {
        array_copy(other, 0, self, 0, other.len().into_usize_safe())
    }

    // TODO
    pub(crate) fn utf16_code_points(&self) -> Result<Vec<u16>, VmError> {
        match self.get_elements_type() {
            ArrayEntryType::Base(BaseType::Char) => {
                let len = self.len().into_usize_safe();
                let mut vec: Vec<u16> = Vec::with_capacity(len);
                unsafe {
                    let ptr = self.data.add(HEADER_LEN) as *const i64;
                    for i in 0..len {
                        let ptr = ptr.add(i);
                        let next_codepoint = std::ptr::read(ptr as *const i32) as u16;
                        vec.push(next_codepoint);
                    }
                }
                Ok(vec)
            }
            _ => Err(VmError::ValidationException),
        }
    }
}

impl<'a> Debug for Array<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "len:{}, data:{:#0x}", self.len(), self.data as usize)
    }
}
