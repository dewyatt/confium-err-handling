#![feature(backtrace)]
use std::backtrace::Backtrace;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};

pub type Result<T> = std::result::Result<T, Error>;

pub struct ErrorCommon {
    pub source: Option<Box<Error>>,
    pub backtrace: Option<Backtrace>,
}

pub enum Error {
    NullPointer { common: ErrorCommon },
    InvalidFormat { common: ErrorCommon },
    InvalidHexDigit { common: ErrorCommon, ch: char },
    Overflow { common: ErrorCommon },
    InvalidUTF8 { common: ErrorCommon },
}

fn cstring(s: *const c_char) -> Result<String> {
    if s.is_null() {
        return Err(Error::NullPointer {
            common: ErrorCommon {
                source: None,
                backtrace: Some(Backtrace::capture()),
            },
        });
    }
    let s = unsafe { std::ffi::CStr::from_ptr(s).to_str() };
    match s {
        Ok(s) => Ok(s.to_string()),
        Err(_) => {
            return Err(Error::InvalidUTF8 {
                common: ErrorCommon {
                    source: None,
                    backtrace: Some(Backtrace::capture()),
                },
            });
        }
    }
}

fn parsehex(s: &str) -> Result<u8> {
    if s.is_empty() {
        return Err(Error::InvalidFormat {
            common: ErrorCommon {
                source: None,
                backtrace: Some(Backtrace::capture()),
            },
        });
    }
    let mut result: u8 = 0;
    for (idx, ch) in s.trim_start_matches("0x").char_indices() {
        let x = match ch.to_digit(16) {
            Some(x) => x,
            None => {
                return Err(Error::InvalidHexDigit {
                    common: ErrorCommon {
                        source: None,
                        backtrace: Some(Backtrace::capture()),
                    },
                    ch: ch,
                });
            }
        } as u8;
        result = match result.checked_mul(16) {
            Some(result) => result,
            None => {
                return Err(Error::Overflow {
                    common: ErrorCommon {
                        source: None,
                        backtrace: Some(Backtrace::capture()),
                    },
                });
            }
        };
        result = match result.checked_add(x) {
            Some(result) => result,
            None => {
                return Err(Error::Overflow {
                    common: ErrorCommon {
                        source: None,
                        backtrace: Some(Backtrace::capture()),
                    },
                });
            }
        };
    }
    Ok(result)
}

fn error_code(error: &Error) -> u32 {
    match error {
        Error::NullPointer { common: _ } => 1,
        Error::InvalidFormat { common: _ } => 2,
        Error::InvalidHexDigit { common: _, ch: _ } => 3,
        Error::Overflow { common: _ } => 4,
        Error::InvalidUTF8 { common: _ } => 5,
    }
}

fn error_message(error: &Error) -> String {
    match error {
        Error::NullPointer { common: _ } => "Null pointer".to_string(),
        Error::InvalidFormat { common: _ } => "Invalid format".to_string(),
        Error::InvalidHexDigit { common: _, ch } => {
            format!("Invalid hex digit: {}", ch)
        }
        Error::InvalidUTF8 { common: _ } => "Invalid UTF-8".to_string(),
        Error::Overflow { common: _ } => "Overflow".to_string(),
    }
}

fn error_backtrace(error: &Error) -> &Option<Backtrace> {
    match error {
        Error::NullPointer { common } => &common.backtrace,
        Error::InvalidFormat { common } => &common.backtrace,
        Error::InvalidHexDigit { common, ch: _ } => &common.backtrace,
        Error::InvalidUTF8 { common } => &common.backtrace,
        Error::Overflow { common } => &common.backtrace,
    }
}

fn error_source(error: &Error) -> &Option<Box<Error>> {
    match error {
        Error::NullPointer { common } => &common.source,
        Error::InvalidFormat { common } => &common.source,
        Error::InvalidHexDigit { common, ch: _ } => &common.source,
        Error::InvalidUTF8 { common } => &common.source,
        Error::Overflow { common } => &common.source,
    }
}

impl From<Error> for u32 {
    #[inline]
    fn from(error: Error) -> u32 {
        error_code(&error)
    }
}

impl From<&Error> for u32 {
    #[inline]
    fn from(error: &Error) -> u32 {
        error_code(error)
    }
}

#[macro_escape]
macro_rules! handle_err {
    ($error:ident, $errptr:ident) => {{
        let code = error_code(&$error);
        if !$errptr.is_null() {
            unsafe {
                *$errptr = Box::into_raw(Box::new($error));
            }
        }
        return code;
    }};
}

#[macro_escape]
macro_rules! non_null {
    ($param:ident, $errptr:ident) => {{
        if $param.is_null() {
            let err = Error::NullPointer {
                common: ErrorCommon {
                    source: None,
                    backtrace: Some(Backtrace::capture()),
                },
            };
            let code = error_code(&err);
            unsafe {
                *$errptr = Box::into_raw(Box::new(err));
            }
            return code;
        }
    }};
}

#[no_mangle]
pub extern "C" fn parse_hex(s: *const c_char, result: *mut u8, err: *mut *mut Error) -> u32 {
    non_null!(result, err);
    let s = match cstring(s) {
        Ok(s) => s,
        Err(e) => {
            handle_err!(e, err);
        }
    };
    match parsehex(&s) {
        Ok(value) => {
            unsafe {
                *result = value;
            }
            0
        }
        Err(e) => {
            handle_err!(e, err);
        }
    }
}

#[no_mangle]
pub extern "C" fn cfm_err_get_msg(err: *const Error, msg: *mut *mut c_char) -> u32 {
    unsafe {
        *msg = std::ptr::null_mut();
        let errmsg = error_message(&*err);
        println!("rust msg: {}", errmsg);
        match CString::new(errmsg) {
            Ok(s) => *msg = s.into_raw(),
            Err(e) => {
                eprintln!("{:?}", e);
                panic!("fail");
            }
        }
    }
    0
}

/*
 * potential for misunderstanding/misuse
 *
 * easier to return the code, but inconsistent with the rest of the api
 * doing it this way lets you detect unintentionally passing null ptr...
 * what would the alternative look like? return what when passed a null err?
 */
#[no_mangle]
pub extern "C" fn cfm_err_get_code(err: *const Error, code: *mut u32) -> u32 {
    unsafe {
        *code = error_code(&*err);
    }
    0
}

#[no_mangle]
pub extern "C" fn cfm_err_get_source(err: *const Error, src: *mut *mut Error) -> u32 {
    unsafe {
        *src = match error_source(&*err) {
            None => std::ptr::null_mut(),
            // TODO: double box?
            Some(ref source) => Box::into_raw(*source),
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn cfm_err_get_backtrace(err: *mut Error, backtrace: *mut *const c_char) -> u32 {
    unsafe {
        *backtrace = match error_backtrace(&*err) {
            None => std::ptr::null_mut(),
            Some(bt) => match CString::new(bt.to_string()) {
                Ok(s) => s.into_raw(),
                Err(e) => {
                    panic!("fail");
                }
            },
        };
    }
    0
}

#[no_mangle]
pub extern "C" fn cfm_err_destroy(err: *mut Error) {
    unsafe {
        Box::from_raw(err);
    }
}

////////////////////
#[no_mangle]
pub extern "C" fn cfm_dotest(input: *const c_char, err: *mut *mut c_void) -> u32 {
    0
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
