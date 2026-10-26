// Copyright (c) 2026 Tsingshui
// SPDX-License-Identifier: MIT

use super::Image;
use crate::Result;
use cildec::Token;

fn parent(name: &str) -> Option<&'static str> {
    match name {
        "System.SystemException" | "System.ApplicationException" => Some("System.Exception"),
        "System.ArithmeticException"
        | "System.ArgumentException"
        | "System.NullReferenceException"
        | "System.InvalidOperationException"
        | "System.IndexOutOfRangeException"
        | "System.FormatException"
        | "System.NotImplementedException"
        | "System.NotSupportedException"
        | "System.InvalidCastException"
        | "System.IO.IOException"
        | "System.IO.InvalidDataException" => Some("System.SystemException"),
        "System.ObjectDisposedException" => Some("System.InvalidOperationException"),
        "System.DivideByZeroException" | "System.OverflowException" => {
            Some("System.ArithmeticException")
        }
        "System.ArgumentNullException" | "System.ArgumentOutOfRangeException" => {
            Some("System.ArgumentException")
        }
        "System.IO.FileNotFoundException"
        | "System.IO.DirectoryNotFoundException"
        | "System.IO.EndOfStreamException" => Some("System.IO.IOException"),
        _ => None,
    }
}

pub(super) fn core_name(image: &Image<'_>, token: Token) -> Result<String> {
    let (namespace, name) = image
        .core_type_parts(token)
        .map_err(|_| "EH catch type needs an admitted core exception identity")?;
    let path = format!("{namespace}.{name}");
    if path != "System.Exception" && parent(&path).is_none() {
        return Err("EH catch type needs an admitted core exception identity".into());
    }
    Ok(path)
}

pub(super) fn core_assignable(image: &Image<'_>, actual: u32, expected: u32) -> bool {
    let (Ok(actual), Ok(expected)) = (
        core_name(image, Token(actual)),
        core_name(image, Token(expected)),
    ) else {
        return false;
    };
    let mut current = actual.as_str();
    for _ in 0..8 {
        if current == expected {
            return true;
        }
        let Some(base) = parent(current) else {
            return false;
        };
        current = base;
    }
    false
}
