#![no_std]

use core::ffi::c_void;
use core::ptr::null_mut;
use alloc::vec::Vec;
use alloc::{slice, string::String};
use wdk::*;
use wdk_alloc::WdkAllocator;
use wdk_sys::ntddk::*;
use wdk_sys::*;

#[repr(C)]
struct ThreadData {
    pub thread_id: u32,
    pub priority: i32,
}

extern crate wdk_panic;
extern crate alloc;

#[global_allocator]
static GLOBAL_ALLOCATOR: WdkAllocator = WdkAllocator;

#[export_name = "DriverEntry"]
pub unsafe extern "system" fn driver_entry(
    driver: &mut DRIVER_OBJECT,
    registry_path: PUNICODE_STRING,
) -> NTSTATUS {
    println!("DriverEntry from Rust! {:p}", &driver);

    let registry_path = unicode_to_string(registry_path);
    println!("Registry Path: {}", registry_path);

    let mut dev = null_mut();
    let mut dev_name = UNICODE_STRING::default();
    string_to_ustring("\\Device\\Booster", &mut dev_name);

    let status = IoCreateDevice(
        driver,
        0,
        &mut dev_name,
        FILE_DEVICE_UNKNOWN,
        0,
        0u8,
        &mut dev,
    );
    if !nt_success(status) {
        println!("Error creating device 0x{:X}", status);
        return status;
    }

    let mut sym_name = UNICODE_STRING::default();
    string_to_ustring("\\??\\Booster", &mut sym_name);
    let status = IoCreateSymbolicLink(&mut sym_name, &mut dev_name);
    if !nt_success(status) {
        println!("Error creating symbolic link 0x{:X}", status);
        IoDeleteDevice(dev);
        return status;
    }

    (*dev).Flags |= DO_BUFFERED_IO;

    driver.DriverUnload = Some(boost_unload);
    driver.MajorFunction[IRP_MJ_CREATE as usize] = Some(boost_create_close);
    driver.MajorFunction[IRP_MJ_CLOSE as usize] = Some(boost_create_close);
    driver.MajorFunction[IRP_MJ_WRITE as usize] = Some(boost_write);

    STATUS_SUCCESS
}

unsafe extern "C" fn boost_unload(driver: *mut DRIVER_OBJECT) {
    let mut sym_name = UNICODE_STRING::default();
    string_to_ustring("\\??\\Booster", &mut sym_name);
    let _ = IoDeleteSymbolicLink(&mut sym_name);
    IoDeleteDevice((*driver).DeviceObject);
}

unsafe extern "C" fn boost_create_close(_device: *mut DEVICE_OBJECT, irp: *mut IRP) -> NTSTATUS {
    (*irp).IoStatus.__bindgen_anon_1.Status = STATUS_SUCCESS;
    (*irp).IoStatus.Information = 0;
    IofCompleteRequest(irp, 0);
    STATUS_SUCCESS
}

fn unicode_to_string(str: PCUNICODE_STRING) -> String {
    String::from_utf16_lossy(unsafe {
        slice::from_raw_parts((*str).Buffer, (*str).Length as usize / 2)
    })
}

fn _string_to_wstring(s: &str) -> Vec<u16> {
    let mut wstring: Vec<_> = s.encode_utf16().collect();
    wstring.push(0); // null terminator
    wstring
}

fn string_to_ustring<'a>(s: &str, uc: &'a mut UNICODE_STRING) -> &'a mut UNICODE_STRING {
    let mut wstring: Vec<_> = s.encode_utf16().collect();
    uc.Length = wstring.len() as u16 * 2;
    uc.MaximumLength = wstring.len() as u16 * 2;
    uc.Buffer = wstring.as_mut_ptr();
    uc
}

unsafe extern "C" fn boost_write(_device: *mut DEVICE_OBJECT, irp: *mut IRP) -> NTSTATUS {
    let data = (*irp).AssociatedIrp.SystemBuffer as *const ThreadData;
    let status;
    loop {
        if data == null_mut() {
            status = STATUS_INVALID_PARAMETER;
            break;
        }
        if (*data).priority < 1 || (*data).priority > 31 {
            status = STATUS_INVALID_PARAMETER;
            break;
        }

        let mut thread = null_mut();
        status = PsLookupThreadByThreadId(((*data).thread_id) as *mut c_void, &mut thread);
        if !nt_success(status) {
            break;
        }

        KeSetPriorityThread(thread, (*data).priority);
        ObfDereferenceObject(thread as *mut c_void);
        break;
    }
    (*irp).IoStatus.__bindgen_anon_1.Status = status;
    (*irp).IoStatus.Information = 0;
    IofCompleteRequest(irp, 0);
    status
}
