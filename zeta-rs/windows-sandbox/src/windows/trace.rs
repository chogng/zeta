use std::collections::HashMap;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Diagnostics::Debug::*;
use windows_sys::Win32::System::LibraryLoader::*;
use windows_sys::Win32::System::Threading::*;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetThreadContext(thread: HANDLE, context: *mut CONTEXT) -> i32;
    fn SetThreadContext(thread: HANDLE, context: *const CONTEXT) -> i32;
}

unsafe fn read(process: HANDLE, address: usize, length: usize) -> Vec<u8> {
    let mut bytes = vec![0; length];
    if unsafe {
        ReadProcessMemory(
            process,
            address as _,
            bytes.as_mut_ptr().cast(),
            length,
            std::ptr::null_mut(),
        )
    } == 0
    {
        return Vec::new();
    }
    bytes
}

unsafe fn name(process: HANDLE, attrs: usize) -> String {
    let object = unsafe { read(process, attrs, 48) };
    if object.len() != 48 {
        return String::new();
    }
    let ptr = usize::from_le_bytes(object[16..24].try_into().unwrap());
    let string = unsafe { read(process, ptr, 16) };
    if string.len() != 16 {
        return String::new();
    }
    let count = u16::from_le_bytes(string[..2].try_into().unwrap()) as usize;
    let ptr = usize::from_le_bytes(string[8..16].try_into().unwrap());
    if count > 4096 {
        return String::new();
    }
    let bytes = unsafe { read(process, ptr, count) };
    String::from_utf16_lossy(
        &bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes(c.try_into().unwrap()))
            .collect::<Vec<_>>(),
    )
}

pub(super) fn run(process: HANDLE) {
    unsafe {
        let module = GetModuleHandleW("ntdll.dll\0".encode_utf16().collect::<Vec<_>>().as_ptr());
        let mut hooks = HashMap::new();
        let mut entries = HashMap::new();
        let mut arguments = HashMap::new();
        for function in [
            "NtCreateFile",
            "NtOpenFile",
            "NtOpenKey",
            "NtOpenKeyEx",
            "NtCreateKey",
            "NtCreateSection",
            "NtOpenSection",
            "NtCreateMutant",
            "NtOpenMutant",
            "NtCreateEvent",
            "NtOpenEvent",
            "NtCreateSemaphore",
            "NtOpenSemaphore",
            "NtOpenProcess",
            "NtOpenThread",
            "NtOpenProcessToken",
            "NtOpenThreadToken",
            "NtOpenProcessTokenEx",
            "NtOpenThreadTokenEx",
        ] {
            let text = std::ffi::CString::new(function).unwrap();
            let Some(address) = GetProcAddress(module, text.as_ptr().cast()).map(|f| f as usize)
            else {
                continue;
            };
            let bytes = read(process, address, 40);
            if bytes.starts_with(&[0x4c, 0x8b, 0xd1]) {
                let trap = 0xccu8;
                assert_ne!(
                    WriteProcessMemory(
                        process,
                        address as _,
                        (&trap as *const u8).cast(),
                        1,
                        std::ptr::null_mut()
                    ),
                    0
                );
                entries.insert(address, function);
            }
            for i in 10..bytes.len() {
                if bytes[i] == 0xc3
                    && (bytes[i - 2..i] == [0x0f, 0x05] || bytes[i - 2..i] == [0xcd, 0x2e])
                {
                    let trap = 0xccu8;
                    assert_ne!(
                        WriteProcessMemory(
                            process,
                            (address + i) as _,
                            (&trap as *const u8).cast(),
                            1,
                            std::ptr::null_mut()
                        ),
                        0
                    );
                    FlushInstructionCache(process, (address + i) as _, 1);
                    hooks.insert(address + i, function);
                }
            }
        }
        let until = std::time::Instant::now() + std::time::Duration::from_secs(15);
        while std::time::Instant::now() < until {
            let mut event = std::mem::zeroed::<DEBUG_EVENT>();
            if WaitForDebugEvent(&mut event, 1000) == 0 {
                continue;
            }
            let mut status = 0x00010002;
            if event.dwDebugEventCode == EXCEPTION_DEBUG_EVENT {
                let address = event.u.Exception.ExceptionRecord.ExceptionAddress as usize;
                if let Some(function) = entries.get(&address) {
                    let thread =
                        OpenThread(THREAD_GET_CONTEXT | THREAD_SET_CONTEXT, 0, event.dwThreadId);
                    let mut context = std::mem::zeroed::<CONTEXT>();
                    context.ContextFlags = 0x0010001f;
                    assert_ne!(GetThreadContext(thread, &mut context), 0);
                    let stack = read(process, (context.Rsp + 0x28) as usize, 24);
                    arguments.insert(
                        (event.dwThreadId, *function),
                        (context.Rdx, context.R8, stack),
                    );
                    context.R10 = context.Rcx;
                    context.Rip = address as u64 + 3;
                    assert_ne!(SetThreadContext(thread, &context), 0);
                    CloseHandle(thread);
                } else if let Some(function) = hooks.get(&address) {
                    let thread =
                        OpenThread(THREAD_GET_CONTEXT | THREAD_SET_CONTEXT, 0, event.dwThreadId);
                    let mut context = std::mem::zeroed::<CONTEXT>();
                    context.ContextFlags = 0x0010001f;
                    assert_ne!(GetThreadContext(thread, &mut context), 0);
                    if context.Rax as u32 == 0xc0000022 {
                        let (access, attrs, stack) =
                            arguments.get(&(event.dwThreadId, *function)).unwrap();
                        eprintln!(
                            "DENIED {function}: mask={access:x} object={} attrs={attrs:x} stack={stack:02x?}",
                            name(process, *attrs as usize)
                        );
                        if *function == "NtCreateSection" && stack.len() == 24 {
                            let file = u64::from_le_bytes(stack[16..24].try_into().unwrap());
                            let mut local = std::ptr::null_mut();
                            if DuplicateHandle(
                                process,
                                file as _,
                                GetCurrentProcess(),
                                &mut local,
                                0,
                                0,
                                2,
                            ) != 0
                            {
                                let mut path = [0u16; 2048];
                                let n=windows_sys::Win32::Storage::FileSystem::GetFinalPathNameByHandleW(local,path.as_mut_ptr(),2048,0);
                                if n > 0 && n < 2048 {
                                    eprintln!(
                                        "file={}",
                                        String::from_utf16_lossy(&path[..n as usize])
                                    );
                                }
                                CloseHandle(local);
                            }
                        }
                    }
                    let ret = read(process, context.Rsp as usize, 8);
                    context.Rip = u64::from_le_bytes(ret.try_into().unwrap());
                    context.Rsp += 8;
                    assert_ne!(SetThreadContext(thread, &context), 0);
                    CloseHandle(thread);
                } else if event.u.Exception.ExceptionRecord.ExceptionCode as u32 != 0x80000003 {
                    status = 0x80010001u32 as i32;
                }
            }
            if event.dwDebugEventCode == LOAD_DLL_DEBUG_EVENT && !event.u.LoadDll.hFile.is_null() {
                CloseHandle(event.u.LoadDll.hFile);
            }
            ContinueDebugEvent(event.dwProcessId, event.dwThreadId, status);
            if event.dwDebugEventCode == EXIT_PROCESS_DEBUG_EVENT {
                break;
            }
        }
    }
}
