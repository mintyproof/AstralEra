use std::{
    ffi::{CString, c_void},
    os::raw::c_uint
};
use windows_sys::Win32::{
    Foundation::{
        CloseHandle,
        GetLastError
    },
    System::{
        Diagnostics::Debug::{
            WOW64_CONTEXT, Wow64GetThreadContext,
            ReadProcessMemory, WriteProcessMemory,
        },
        Memory::{VirtualProtectEx, PAGE_READWRITE},
        SystemInformation::GetTickCount,
        Threading::{
            STARTUPINFOA, PROCESS_INFORMATION,
            CreateProcessA, TerminateProcess,
            CREATE_SUSPENDED, NORMAL_PRIORITY_CLASS,
            ResumeThread
        },
    }
};

// ..for some reason this doesn't seem to be part of the windows crate?
// please correct me if i'm wrong. i'm so lost on this
// 
// based on this:
// https://github.com/wine-mirror/wine/blob/master/include/winnt.h#L1169=
const CONTEXT_ARCH: u32 = 0x00100000; // maybe wrong
const CONTEXT_ARCH_CONTROL: u32  = CONTEXT_ARCH | 0x0001;
const CONTEXT_ARCH_INTEGER: u32  = CONTEXT_ARCH | 0x0002;
const CONTEXT_ARCH_SEGMENTS: u32 = CONTEXT_ARCH | 0x0004;
const CONTEXT_FULL: u32 = CONTEXT_ARCH_CONTROL | CONTEXT_ARCH_INTEGER | CONTEXT_ARCH_SEGMENTS;

// addresses and data for patching the game process at launch time.
//
// based on SeventhUmbral's patches:
// https://github.com/jpd002/SeventhUmbral/blob/master/launcher/Launcher.cpp#L8=
const ENCRYPTIONKEY_PATCH_ADDRESS: u32 = 0x9A15E3;
const ENCRYPTIONKEY_PATCH: [u8; 5] = [0xB8, 0x12, 0xE8, 0xE0, 0x50];
const LOBBY_HOSTNAME_PATCH_ADDRESS: u32 = 0xB90110;
const LOBBY_HOSTNAME_PATCH_MAX_SIZE: usize = 0x14;

// hardcoded temp values
pub const GAME_PATH: &'static str = "C:/Program Files (x86)/SquareEnix/FINAL FANTASY XIV/";
pub const GAME_EXE_GAME: &'static str = "C:/Program Files (x86)/SquareEnix/FINAL FANTASY XIV/ffxivgame.exe";

/// fetches the arguments required to launch the game. the game is passed
/// the result of winapi GetTickCount, the language, the region, the server time,
/// and the session ID. these are encrypted with Blowfish, using the tick count as
/// the encryption key, then encoded in base64.
/// (with substitutions - '+' to '-' and '/' to '_')
/// 
/// this is not *all* that is required to launch the game- the game still needs to
/// know the Blowfish encryption key, as this is not provided at launch.
/// we provide this post-launch via patching the running process's memory.
pub fn get_launch_arg(session_id: &str) -> String {
    let tick_count = unsafe { GetTickCount() };

    let args = format!(
        " T ={} /LANG =en-us /REGION =2 /SERVER_UTC =1156916742 /SESSION_ID ={}",
        tick_count, session_id
    );

    let encryption_key = format!("{:08x}", tick_count & !0xFFFF);
    let blowfish = crypto::blowfish::Blowfish::new(encryption_key.as_bytes());
    
    let args_bytes = args.as_bytes();
    let args_len = args.len() + 1;
    let mut args_encrypted_blowfish: Vec<u8> = Vec::new();
    for i in (0 .. args_len & !0x7).step_by(8) {
        use byteorder::ByteOrder;

        let (l, r) = blowfish.encrypt(
            byteorder::LittleEndian::read_u32(&args_bytes[i..i+4]),
            byteorder::LittleEndian::read_u32(&args_bytes[i+4..i+8])
        );
        
        let l: [u8; 4] = unsafe { std::mem::transmute(l.to_le()) };
        let r: [u8; 4] = unsafe { std::mem::transmute(r.to_le()) };

        for byte in l {
            args_encrypted_blowfish.push(byte);
        }
        for byte in r {
            args_encrypted_blowfish.push(byte);
        }
    }

    let args_encrypted_base64 = base64::encode(args_encrypted_blowfish);
    let args_encrypted_base64 = args_encrypted_base64.replace("+", "-");
    let args_encrypted_base64 = args_encrypted_base64.replace("/", "_");
    
    args_encrypted_base64
}

/// helper function for patch_process- given a process, a memory address,
/// and bytes to patch in, it'll.. well, patch it in!
/// 
/// this is based on the process SeventhUmbral uses:
/// https://github.com/jpd002/SeventhUmbral/blob/master/launcher/Launcher.cpp#L14=
pub unsafe fn patch_process_memory(hprocess: isize, address: u32, patch_bytes: &[u8]) -> Result<(), String> {
    let mut old_protect: u32 = 0;
    let remote_address = address as *const c_void;
    let patch_bytes_ptr = patch_bytes.as_ptr() as *const c_void;
    let patch_size = patch_bytes.len();

    if VirtualProtectEx(hprocess, remote_address, patch_size, PAGE_READWRITE, &mut old_protect) == 0 {
        return Err(format!("failed to change page protection({})", GetLastError()));
    }

    let mut num_written: usize = 0;
    let write_process_memory_result =
        WriteProcessMemory(hprocess, remote_address, patch_bytes_ptr, patch_size, &mut num_written);
    if write_process_memory_result == 0 || num_written != patch_size {
        return Err(format!("failed to apply patch ({})", GetLastError()));
    }

    if VirtualProtectEx(hprocess, remote_address, patch_size, old_protect, &mut old_protect) == 0 {
        return Err(format!("failed to restore page protection ({})", GetLastError()));
    }

    Ok(())
}

/// patches the game process. at launch, the game is unaware of our custom server
/// and the encryption key for the launch arguments. this function patches the
/// running process's memory to inform it of both of these things.
/// 
/// this is based on the process SeventhUmbral uses:
/// https://github.com/jpd002/SeventhUmbral/blob/master/launcher/Launcher.cpp#L45=
pub unsafe fn patch_game(hprocess: isize, hthread: isize, lobby_hostname: &str) -> Result<(), String> {
    let mut thread_context: WOW64_CONTEXT = std::mem::zeroed::<WOW64_CONTEXT>();
    thread_context.ContextFlags = CONTEXT_FULL;
    if Wow64GetThreadContext(hthread, &mut thread_context) == 0 {
        return Err(format!("failed to get thread context ({})", GetLastError()));
    }

    let base_address_pointer: u32 = thread_context.Ebx + 8;
    let mut base_address: u32 = 0;
    let mut number_of_bytes_read: usize = 0;
    let result_of_readprocessmemory = ReadProcessMemory(
        hprocess,
        base_address_pointer as *const c_void,
        &mut base_address as *mut c_uint as *mut c_void,
        4,
        &mut number_of_bytes_read
    );
    println!("base_address_pointer: {}", base_address_pointer);
    println!("base_address:         {}", base_address);
    println!("number_of_bytes_read: {}", number_of_bytes_read);
    if result_of_readprocessmemory == 0 {
        return Err(format!("failed to get base address ({})", GetLastError()));
    }

    println!("applying encryption key patch..");
    match patch_process_memory(hprocess, base_address + ENCRYPTIONKEY_PATCH_ADDRESS, &ENCRYPTIONKEY_PATCH) {
        Ok(_) => { println!("patch applied!"); }
        Err(err) => {
            println!("patch failed: {}", err);
            return Err(err);
        }
    }
    
    println!("applying lobby hostname patch..");
    if lobby_hostname.as_bytes().len() > LOBBY_HOSTNAME_PATCH_MAX_SIZE {
        return Err(format!("patch failed: lobby hostname exceeds max size of {} bytes", LOBBY_HOSTNAME_PATCH_MAX_SIZE));
    }
    match patch_process_memory(hprocess, base_address + LOBBY_HOSTNAME_PATCH_ADDRESS, lobby_hostname.as_bytes()) {
        Ok(_) => { println!("patch applied!"); }
        Err(err) => {
            println!("patch failed: {}", err);
            return Err(err);
        }
    }

    Ok(())
}

/// launches the game, of course. what else could it do?
pub fn launch_game(path_to_exe: &str, working_directory: &str) {
    let arg = get_launch_arg("00000000000000000000000000000000000000000000000000000000");

    let command_line = format!("{} sqex0002{}!////", path_to_exe, arg);
    let cstr_working_directory = CString::new(working_directory.clone()).unwrap();
    let cstr_command_line = CString::new(command_line.clone()).unwrap();

    println!("launching game (command line: {})", command_line);

    unsafe {
        let mut startup_info = std::mem::zeroed::<STARTUPINFOA>();
        startup_info.cb = std::mem::size_of::<STARTUPINFOA>() as u32;

        let mut process_info = std::mem::zeroed::<PROCESS_INFORMATION>();

        let process_created = CreateProcessA(
            std::ptr::null(),
            cstr_command_line.as_ptr() as *mut u8,
            std::ptr::null(),
            std::ptr::null(),
            0,
            CREATE_SUSPENDED | NORMAL_PRIORITY_CLASS,
            std::ptr::null(),
            cstr_working_directory.as_ptr() as *const u8,
            &startup_info,
            &mut process_info);
        if process_created == 0 {
            println!("failed to launch game: {}", GetLastError());
        } else {
            println!("launched game!");
        }

        println!("patching game (process {})..", process_info.hProcess);
        let patch_result = patch_game(process_info.hProcess, process_info.hThread, "127.0.0.1\0");
        match patch_result {
            // the game was patched successfully! we can resume the thread now and carry on.
            Ok(_) => {
                println!("successfully patched game process!");
            },
            // ah! it failed to patch! terminate it and toss an error..
            Err(err) => {
                println!("failed to patch game process: {}", err);
                TerminateProcess(process_info.hProcess, 1);
                // error todo. all of this todo
            }
        }

        ResumeThread(process_info.hThread);

        CloseHandle(process_info.hProcess);
        CloseHandle(process_info.hThread);
    }
    
    //let result = Command::new(path_to_exe)
    //    .current_dir(working_directory)
    //    .arg(format!("sqex0002{}!////", arg))
    //    //.arg("sqex0002g_nixFShp-EXVuj4AwDjAvqv1cRXzy0HAXChXj4HORR3Ndg5CWvNTfO0OaFMbyx5QvYHDUfwSkE_vv69lwYF7Y5TklLY0FCKEqFwWbNe3iQCu0wImBb9OvN0_Rp8k8OyY2WWfK37JEl9zbMP_GVUvrjHy5z4VlTgKP17GLVcxaxkMAA=!////")
    //    .status()
    //    .expect("couldn't launch!");
    //println!("{}, {}", result, result.success());
}