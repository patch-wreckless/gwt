use anyhow::anyhow;
use std::ffi::{CStr, CString, OsStr};
use std::os::raw::{c_char, c_int, c_void};
use std::process::{self, Stdio};

const GIR_URL_BASENAME_FN_NAME: &'static CStr = c"git_url_basename";

type GitUrlBasename =
    unsafe extern "C" fn(repo: *const c_char, is_bundle: c_int, is_bare: c_int) -> *mut c_char;

pub struct Git {
    git_bin: *mut c_void,
    url_basename: GitUrlBasename,
}

impl Git {
    pub unsafe fn link(filename: &str) -> Result<Self, String> {
        let filename = match CString::new(filename) {
            Ok(path) => Ok(path),
            Err(_) => Err("Filename {filename} contains null bytes.".to_owned()),
        }?;
        let git_bin = unsafe { dlopen(&filename, libc::RTLD_NOW) }?;
        let url_basename = unsafe { dlsym(git_bin, GIR_URL_BASENAME_FN_NAME) }?;
        Ok(Self {
            git_bin: git_bin,
            url_basename,
        })
    }

    pub unsafe fn url_basename(&self, url: &str, is_bundle: bool) -> Result<String, String> {
        let url = match CString::new(url) {
            Ok(path) => Ok(path),
            Err(_) => Err("Url {filename} contains null bytes.".to_owned()),
        }?;
        let result =
            unsafe { (self.url_basename)(url.as_ptr(), is_bundle as c_int, false as c_int) };

        if result.is_null() {
            return Err("git_url_basename returned NULL".into());
        }

        match unsafe { CStr::from_ptr(result).to_str() } {
            Ok(basename) => Ok(basename.to_owned()),
            Err(err) => {
                Err(format!("git_url_basename returned invalid UTF8 string: {err}").to_owned())
            }
        }
    }
}

impl Drop for Git {
    fn drop(&mut self) {
        unsafe {
            libc::dlclose(self.git_bin);
        }
    }
}

pub fn exec<I, S>(args: I) -> anyhow::Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let args = args.into_iter().collect::<Vec<_>>();

    let command_output = process::Command::new("git")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .args(args)
        .output()?;

    match command_output.status.code() {
        Some(0) => Ok(()),
        exit_err => match exit_err {
            Some(non_zero) => Err(anyhow!("git process exited with status code {non_zero}")),
            None => Err(anyhow!("git process did not provide an exit code")),
        },
    }
}

pub fn exec_capture<I, S>(args: I) -> anyhow::Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let args = args.into_iter().collect::<Vec<_>>();

    let command_output = process::Command::new("git")
        .stderr(Stdio::inherit())
        .args(args)
        .output()?;

    match command_output.status.code() {
        Some(0) => match str::from_utf8(&command_output.stdout) {
            Ok(output) => Ok(output.to_owned()),
            Err(_) => Err(anyhow!("git process emitted invalid UTF8 to stdout")),
        },
        exit_err => match exit_err {
            Some(non_zero) => Err(anyhow!("git process exited with status code {non_zero}")),
            None => Err(anyhow!("git process did not provide an exit code")),
        },
    }
}

unsafe fn dlopen(filename: &CStr, flag: c_int) -> Result<*mut c_void, String> {
    let f = || unsafe { libc::dlopen(filename.as_ptr(), flag) };
    unsafe { dlcall("dlopen", &f) }
}

unsafe fn dlsym<T>(handle: *mut c_void, symbol: &CStr) -> Result<T, String> {
    let f = || unsafe { libc::dlsym(handle, symbol.as_ptr()) };
    let ptr = unsafe { dlcall("dlopen", &f) }?;
    Ok(unsafe { std::mem::transmute_copy::<*mut c_void, T>(&ptr) })
}

unsafe fn dlcall(fn_name: &str, r#fn: &dyn Fn() -> *mut c_void) -> Result<*mut c_void, String> {
    // dl* function calls that succeed don't clear dlerror so we clear it before calling one to
    // avoid finding a pre-existing error after our own call.
    _ = unsafe { libc::dlerror() };

    let ptr = r#fn();
    let err = unsafe { libc::dlerror() };
    if !err.is_null() {
        let msg = unsafe { CStr::from_ptr(err).to_string_lossy() };
        return Err(format!("{fn_name}(): {msg}").to_owned());
    }

    if ptr.is_null() {
        return Err(format!("{fn_name}() returned null pointer but no error").to_owned());
    }

    Ok(ptr)
}
