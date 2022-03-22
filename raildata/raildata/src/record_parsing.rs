/** Copyright James Lomax 2020 */

use std::io;

pub fn extract_record_field<'a>(fieldname: &str, rec: &'a str, offset: usize, len: usize) -> io::Result<&'a str> {
    if offset + len > rec.len() {
        let msg = format!(
            "Bad record length {} (while parsing field {})",
            rec.len(), fieldname
        );
        return Err(io::Error::new(io::ErrorKind::InvalidData, msg));
    } else {
        return Ok(rec[offset..offset+len].trim());
    }
}

macro_rules! make_record_type {
    ($T:ident, $( ($name:ident, $offset:expr, $len:expr) ),*) => {
        struct $T<'a> {
            $($name: &'a str,)*
        }

        impl<'a> $T<'a> {
            fn read(rec: &'a str) -> io::Result<Self> {
                Ok(Self {
                    $($name: crate::record_parsing::extract_record_field("$name", rec, $offset, $len)?,)*
                })
            }
        }
    }
}

pub fn parse_or_invalid<T>(s: &str, fieldname: &str) -> io::Result<T> 
    where T : std::str::FromStr
{
    match s.parse::<T>() {
        Ok(v) => Ok(v),
        Err(_) => {
            let msg = format!("Could not parse field {} '{}'", fieldname, s);
            Err(io::Error::new(io::ErrorKind::InvalidData, msg))
        }
    }
}
