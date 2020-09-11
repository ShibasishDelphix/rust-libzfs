extern crate cstr_argument;
extern crate nvpair_sys as sys;
#[macro_use]
extern crate foreign_types;

use cstr_argument::CStrArgument;
use foreign_types::{ForeignType, ForeignTypeRef, Opaque};
use std::ffi;
use std::io;
use std::os::raw::c_int;
use std::ptr;
use std::mem::MaybeUninit;

#[derive(Debug)]
pub enum NvData<'a> {
    Unknown,
    Bool,
    BoolV(bool),
    Byte(u8),
    Int8(i8),
    Uint8(u8),
    Int16(i16),
    Uint16(u16),
    Int32(i32),
    Uint32(u32),
    Int64(i64),
    Uint64(u64),
    Str(&'a ffi::CStr),
    NvListRef(&'a NvListRef),
    // TODO: arrays
    // hrtime
    // double
}

pub trait NvEncode {
    fn insert<S: CStrArgument>(&self, S, &mut NvListRef) -> io::Result<()>;
    //fn read(NvPair &nv) -> io::Result<Self>;
}

impl NvEncode for bool {
    fn insert<S: CStrArgument>(&self, name: S, nv: &mut NvListRef) -> io::Result<()> {
        let name = name.into_cstr();
        let v = unsafe {
            sys::nvlist_add_boolean_value(
                nv.as_mut_ptr(),
                name.as_ref().as_ptr(),
                if *self {
                    sys::boolean_t::B_TRUE
                } else {
                    sys::boolean_t::B_FALSE
                },
            )
        };
        if v != 0 {
            Err(io::Error::from_raw_os_error(v))
        } else {
            Ok(())
        }
    }
}

impl NvEncode for u32 {
    fn insert<S: CStrArgument>(&self, name: S, nv: &mut NvListRef) -> io::Result<()> {
        let name = name.into_cstr();
        let v = unsafe { sys::nvlist_add_uint32(nv.as_mut_ptr(), name.as_ref().as_ptr(), *self) };
        if v != 0 {
            Err(io::Error::from_raw_os_error(v))
        } else {
            Ok(())
        }
    }
}

impl NvEncode for ffi::CStr {
    fn insert<S: CStrArgument>(&self, name: S, nv: &mut NvListRef) -> io::Result<()> {
        let name = name.into_cstr();
        let v = unsafe {
            sys::nvlist_add_string(nv.as_mut_ptr(), name.as_ref().as_ptr(), self.as_ptr())
        };
        if v != 0 {
            Err(io::Error::from_raw_os_error(v))
        } else {
            Ok(())
        }
    }
}

impl NvEncode for NvListRef {
    fn insert<S: CStrArgument>(&self, name: S, nv: &mut NvListRef) -> io::Result<()> {
        let name = name.into_cstr();
        let v = unsafe {
            sys::nvlist_add_nvlist(
                nv.as_mut_ptr(),
                name.as_ref().as_ptr(),
                self.as_ptr() as *mut _,
            )
        };
        if v != 0 {
            Err(io::Error::from_raw_os_error(v))
        } else {
            Ok(())
        }
    }
}

pub enum NvEncoding {
    Native,
    Xdr,
}

impl NvEncoding {
    fn as_raw(&self) -> c_int {
        match self {
            &NvEncoding::Native => sys::NV_ENCODE_NATIVE,
            &NvEncoding::Xdr => sys::NV_ENCODE_XDR,
        }
    }
}

foreign_type! {
    /// An `NvList`
    pub unsafe type NvList {
        type CType = sys::nvlist;
        fn drop = sys::nvlist_free;
    }
}

impl NvList {
    /// Create a new `NvList` with no options
    pub fn new() -> io::Result<Self> {
        let mut n = ptr::null_mut();
        let v = unsafe {
            // TODO: second arg is a bitfield of NV_UNIQUE_NAME|NV_UNIQUE_NAME_TYPE
            sys::nvlist_alloc(&mut n, 0, 0)
        };
        if v != 0 {
            Err(io::Error::from_raw_os_error(v))
        } else {
            Ok(unsafe { Self::from_ptr(n) })
        }
    }

    /// Create a new `NvList` with the `NV_UNIQUE_NAME` constraint
    pub fn new_unqiue_names() -> io::Result<Self> {
        let mut n = ptr::null_mut();
        let v = unsafe { sys::nvlist_alloc(&mut n, sys::NV_UNIQUE_NAME, 0) };
        if v != 0 {
            Err(io::Error::from_raw_os_error(v))
        } else {
            Ok(unsafe { Self::from_ptr(n) })
        }
    }

    pub fn try_clone(&self) -> io::Result<Self> {
        let mut n = ptr::null_mut();
        let v = unsafe { sys::nvlist_dup(self.as_ptr(), &mut n, 0) };
        if v != 0 {
            Err(io::Error::from_raw_os_error(v))
        } else {
            Ok(unsafe { Self::from_ptr(n) })
        }
    }
}

impl Clone for NvList {
    fn clone(&self) -> Self {
        self.try_clone().unwrap()
    }
}

impl NvListRef {
    pub unsafe fn from_mut_ptr<'a>(v: *mut sys::nvlist) -> &'a mut Self {
        std::mem::transmute::<*mut sys::nvlist, &mut Self>(v)
    }

    pub unsafe fn from_ptr<'a>(v: *const sys::nvlist) -> &'a Self {
        std::mem::transmute::<*const sys::nvlist, &Self>(v)
    }

    pub fn as_mut_ptr(&mut self) -> *mut sys::nvlist {
        unsafe { std::mem::transmute::<&mut NvListRef, *mut sys::nvlist>(self) }
    }

    pub fn as_ptr(&self) -> *const sys::nvlist {
        unsafe { std::mem::transmute::<&NvListRef, *const sys::nvlist>(self) }
    }

    pub fn encoded_size(&self, encoding: NvEncoding) -> io::Result<u64> {
        let mut l = 0u64;
        let v = unsafe { sys::nvlist_size(self.as_ptr() as *mut _, &mut l, encoding.as_raw()) };
        if v != 0 {
            Err(io::Error::from_raw_os_error(v))
        } else {
            Ok(l)
        }
    }

    pub fn is_empty(&self) -> bool {
        let v = unsafe { sys::nvlist_empty(self.as_ptr() as *mut _) };
        v != sys::boolean_t::B_FALSE
    }

    pub fn add_boolean<S: CStrArgument>(&mut self, name: S) -> io::Result<()> {
        let name = name.into_cstr();
        let v = unsafe { sys::nvlist_add_boolean(self.as_mut_ptr(), name.as_ref().as_ptr()) };
        if v != 0 {
            Err(io::Error::from_raw_os_error(v))
        } else {
            Ok(())
        }
    }

    pub fn first(&self) -> Option<&NvPair> {
        let np = unsafe { sys::nvlist_next_nvpair(self.as_ptr() as *mut _, ptr::null_mut()) };
        if np.is_null() {
            None
        } else {
            Some(unsafe { NvPair::from_ptr(np) })
        }
    }

    pub fn iter(&self) -> NvListIter {
        NvListIter {
            parent: self,
            pos: ptr::null_mut(),
        }
    }

    pub fn exists<S: CStrArgument>(&self, name: S) -> bool {
        let name = name.into_cstr();
        let v = unsafe { sys::nvlist_exists(self.as_ptr() as *mut _, name.as_ref().as_ptr()) };
        v != sys::boolean_t::B_FALSE
    }

    /*
    // not allowed because `pair` is borrowed from `self`. Need to fiddle around so that we can
    // check:
    //  - `pair` is from `self`
    //  - `pair` is the only outstanding reference to this pair (need by-value semantics)
    pub fn remove(&mut self, pair: &NvPair) -> io::Result<()>
    {
        let v = unsafe { sys::nvlist_remove_nvpair(self.as_mut_ptr(), pair.as_ptr())};
        if v != 0 {
            Err(io::Error::from_raw_os_error(v))
        } else {
            Ok(())
        }
    }
    */

    pub fn lookup<S: CStrArgument>(&self, name: S) -> io::Result<&NvPair> {
        let name = name.into_cstr();
        let mut n = ptr::null_mut();
        let v = unsafe {
            sys::nvlist_lookup_nvpair(self.as_ptr() as *mut _, name.as_ref().as_ptr(), &mut n)
        };
        if v != 0 {
            Err(io::Error::from_raw_os_error(v))
        } else {
            Ok(unsafe { NvPair::from_ptr(n) })
        }
    }

    pub fn try_to_owned(&self) -> io::Result<NvList> {
        let mut n = MaybeUninit::uninit();
        let v = unsafe { sys::nvlist_dup(self.as_ptr() as *mut _, n.as_mut_ptr(), 0) };
        if v != 0 {
            Err(io::Error::from_raw_os_error(v))
        } else {
            Ok(unsafe { NvList::from_ptr(n.assume_init())})
        }
    }

   pub fn lookup_nvlist<S: CStrArgument>(&self, name: S) -> io::Result<NvList> {
        let name = name.into_cstr();

        let mut n = MaybeUninit::uninit();
        let v = unsafe {
            sys::nvlist_lookup_nvlist(self.as_ptr() as *mut _, name.as_ref().as_ptr(), n.as_mut_ptr())
        };
        if v != 0 {
            Err(io::Error::from_raw_os_error(v))
        } else {
            let r = unsafe { NvList::from_ptr(n.assume_init()) };
            Ok(r)
        }
    }

    pub fn lookup_string<S: CStrArgument>(&self, name: S) -> io::Result<ffi::CString> {
        let name = name.into_cstr();
        let mut n = MaybeUninit::uninit();
        let v = unsafe {
            sys::nvlist_lookup_string(self.as_ptr() as *mut _, name.as_ref().as_ptr(), n.as_mut_ptr())
        };

        if v != 0 {
            Err(io::Error::from_raw_os_error(v))
        } else {
            let s = unsafe { ffi::CStr::from_ptr(n.assume_init()).to_owned() };
            Ok(s)
        }
    }

    pub fn lookup_uint64<S: CStrArgument>(&self, name: S) -> io::Result<u64> {
        let name = name.into_cstr();
        let mut n = MaybeUninit::uninit();
        let v = unsafe {
            sys::nvlist_lookup_uint64(self.as_ptr() as *mut _, name.as_ref().as_ptr(), n.as_mut_ptr())
        };
        if v != 0 {
            Err(io::Error::from_raw_os_error(v))
        } else {
            Ok(unsafe { n.assume_init() })
        }
    }


    pub fn lookup_nvlist_array<S: CStrArgument>(&self, name: S) -> io::Result<Vec<NvList>> {
        let name = name.into_cstr();
        let mut n = ptr::null_mut();
        let mut len = 0;
        let v = unsafe {
            sys::nvlist_lookup_nvlist_array(
                self.as_ptr() as *mut _,
                name.as_ref().as_ptr(),
                &mut n,
                &mut len,
            )
        };
        if v != 0 {
            Err(io::Error::from_raw_os_error(v))
        } else {
            let r = unsafe {
                std::slice::from_raw_parts(n, len as usize)
                    .iter()
                    .map(|x| NvList::from_ptr(*x))
                    .collect()
            };

            Ok(r)
        }
    }

    pub fn lookup_uint64_array<S: CStrArgument>(&self, name: S) -> io::Result<Vec<u64>> {
        let name = name.into_cstr();

        let mut n = ptr::null_mut();
        let mut len = 0;
        let v = unsafe {
            sys::nvlist_lookup_uint64_array(
                self.as_ptr() as *mut _,
                name.as_ref().as_ptr(),
                &mut n,
                &mut len,
            )
        };

        if v != 0 {
            Err(io::Error::from_raw_os_error(v))
        } else {
            let r = unsafe {
                ::std::slice::from_raw_parts(n, len as usize)
                    .iter()
                    .map(|x| *x)
                    .collect()
            };

            Ok(r)
        }
    } 
}

impl std::fmt::Debug for NvListRef {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_map()
            .entries(
                self.iter()
                    .map(|&ref pair| (pair.name().to_owned().into_string().unwrap(), pair.data())),
            )
            .finish()
    }
}

pub struct NvListIter<'a> {
    parent: &'a NvListRef,
    pos: *mut sys::nvpair,
}

impl<'a> Iterator for NvListIter<'a> {
    type Item = &'a NvPair;

    fn next(&mut self) -> Option<Self::Item> {
        let np = unsafe { sys::nvlist_next_nvpair(self.parent.as_ptr() as *mut _, self.pos) };
        self.pos = np;
        if np.is_null() {
            None
        } else {
            Some(unsafe { NvPair::from_ptr(np) })
        }
    }
}

pub struct NvPair(Opaque);
unsafe impl ForeignTypeRef for NvPair {
    type CType = sys::nvpair;
}

impl NvPair {
    pub fn name(&self) -> &ffi::CStr {
        unsafe { ffi::CStr::from_ptr(sys::nvpair_name(self.as_ptr())) }
    }

    pub fn data(&self) -> NvData {
        let data_type = unsafe { sys::nvpair_type(self.as_ptr()) };

        match data_type {
            sys::data_type_t::DATA_TYPE_BOOLEAN => {
                NvData::Bool
            }
            sys::data_type_t::DATA_TYPE_BOOLEAN_VALUE => {
                let v = unsafe {
                    let mut v = MaybeUninit::uninit();
                    sys::nvpair_value_boolean_value(self.as_ptr(), v.as_mut_ptr());
                    v.assume_init()
                };

                NvData::BoolV(v == sys::boolean_t::B_TRUE)
            }
            sys::data_type_t::DATA_TYPE_BYTE => {
                let v = unsafe {
                    let mut v = MaybeUninit::uninit();
                    sys::nvpair_value_byte(self.as_ptr(), v.as_mut_ptr());
                    v.assume_init()
                };

                NvData::Byte(v)
            }
            sys::data_type_t::DATA_TYPE_INT8 => {
                let v = unsafe {
                    let mut v = MaybeUninit::uninit();
                    sys::nvpair_value_int8(self.as_ptr(), v.as_mut_ptr());
                    v.assume_init()
                };

                NvData::Int8(v)
            }
            sys::data_type_t::DATA_TYPE_UINT8 => {
                let v = unsafe {
                    let mut v = MaybeUninit::uninit();
                    sys::nvpair_value_uint8(self.as_ptr(), v.as_mut_ptr());
                    v.assume_init()
                };

                NvData::Uint8(v)
            }
            sys::data_type_t::DATA_TYPE_INT16 => {
                let v = unsafe {
                    let mut v = MaybeUninit::uninit();
                    sys::nvpair_value_int16(self.as_ptr(), v.as_mut_ptr());
                    v.assume_init()
                };

                NvData::Int16(v)
            }
            sys::data_type_t::DATA_TYPE_UINT16 => {
                let v = unsafe {
                    let mut v = MaybeUninit::uninit();
                    sys::nvpair_value_uint16(self.as_ptr(), v.as_mut_ptr());
                    v.assume_init()
                };

                NvData::Uint16(v)
            }
            sys::data_type_t::DATA_TYPE_INT32 => {
                let v = unsafe {
                    let mut v = MaybeUninit::uninit();
                    sys::nvpair_value_int32(self.as_ptr(), v.as_mut_ptr());
                    v.assume_init()
                };

                NvData::Int32(v)
            }
            sys::data_type_t::DATA_TYPE_UINT32 => {
                let v = unsafe {
                    let mut v = MaybeUninit::uninit();
                    sys::nvpair_value_uint32(self.as_ptr(), v.as_mut_ptr());
                    v.assume_init()
                };

                NvData::Uint32(v)
            }
            sys::data_type_t::DATA_TYPE_INT64 => {
                let v = unsafe {
                    let mut v = MaybeUninit::uninit();
                    sys::nvpair_value_int64(self.as_ptr(), v.as_mut_ptr());
                    v.assume_init()
                };

                NvData::Int64(v)
            }
            sys::data_type_t::DATA_TYPE_UINT64 => {
                let v = unsafe {
                    let mut v = MaybeUninit::uninit();
                    sys::nvpair_value_uint64(self.as_ptr(), v.as_mut_ptr());
                    v.assume_init()
                };

                NvData::Uint64(v)
            }
            sys::data_type_t::DATA_TYPE_STRING => {
                let s = unsafe {
                    let mut n = MaybeUninit::uninit();
                    sys::nvpair_value_string(self.as_ptr(), n.as_mut_ptr());
                    ffi::CStr::from_ptr(n.assume_init())
                };

                NvData::Str(s)
            }
            sys::data_type_t::DATA_TYPE_NVLIST => {
                let l = unsafe {
                    let mut l = MaybeUninit::uninit();
                    sys::nvpair_value_nvlist(self.as_ptr(), l.as_mut_ptr());
                    NvListRef::from_ptr(l.assume_init())
                };

                NvData::NvListRef(l)
            }
            _ => NvData::Unknown,
        }
    }
}

impl std::fmt::Debug for NvPair {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_tuple("NvPair")
            .field(&self.name())
            .field(&self.data())
            .finish()
    }
}

impl<'a> NvData<'a> {
    pub fn as_str(&self) -> Option<&ffi::CStr> {
        match self {
            NvData::Str(c) => Some(c),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<String> {
        self.as_str()?.to_owned().into_string().ok()
    }

    pub fn as_list(&self) -> Option<&NvListRef> {
        match self {
            NvData::NvListRef(c) => Some(c),
            _ => None,
        }
    }
}
